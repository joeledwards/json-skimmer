//! Bounded, reproducible properties without new test dependencies.
use super::*;

struct Generator(u64);

impl Generator {
    fn pick(&mut self, upper: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 32) as usize) % upper
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Atom {
    Open(ContainerKind),
    Close(ContainerKind),
    String(StringKind, Vec<u8>),
    Number(Vec<u8>),
    Boolean(bool),
    Null,
}

enum Value {
    Array(Vec<Value>),
    Object(Vec<(&'static str, Value)>),
    String(&'static str),
    Number(&'static str),
    Boolean(bool),
    Null,
}

const STRINGS: &[&str] = &[
    "",
    "plain",
    "雪😀",
    r"a\nb\t",
    r#"\"\\\/"#,
    r"\u0041\uD834\uDD1E",
];
const NUMBERS: &[&str] = &[
    "0",
    "-0",
    "123456789012345678901234567890",
    "-1.2300",
    "1e+09",
    "2E-99",
];

fn value(generator: &mut Generator, depth: usize) -> Value {
    match generator.pick(if depth == 0 { 4 } else { 6 }) {
        0 => Value::String(STRINGS[generator.pick(STRINGS.len())]),
        1 => Value::Number(NUMBERS[generator.pick(NUMBERS.len())]),
        2 => Value::Boolean(generator.pick(2) == 0),
        3 => Value::Null,
        4 => Value::Array(
            (0..generator.pick(4))
                .map(|_| value(generator, depth - 1))
                .collect(),
        ),
        _ => Value::Object(
            (0..generator.pick(4))
                .map(|_| {
                    (
                        STRINGS[generator.pick(STRINGS.len())],
                        value(generator, depth - 1),
                    )
                })
                .collect(),
        ),
    }
}

// Serialization and expected semantic events are separate traversals of a tree.
impl Value {
    fn render(&self) -> String {
        match self {
            Self::Array(values) => format!(
                "[ {} ]",
                values
                    .iter()
                    .map(Self::render)
                    .collect::<Vec<_>>()
                    .join(",\n")
            ),
            Self::Object(members) => format!(
                "{{ {} }}",
                members
                    .iter()
                    .map(|(key, value)| format!("\"{key}\" : {}", value.render()))
                    .collect::<Vec<_>>()
                    .join(",\t")
            ),
            Self::String(raw) => format!("\"{raw}\""),
            Self::Number(raw) => raw.to_string(),
            Self::Boolean(value) => value.to_string(),
            Self::Null => "null".into(),
        }
    }

    fn expected(&self, atoms: &mut Vec<Atom>) {
        match self {
            Self::Array(values) => {
                atoms.push(Atom::Open(ContainerKind::Array));
                for value in values {
                    value.expected(atoms);
                }
                atoms.push(Atom::Close(ContainerKind::Array));
            }
            Self::Object(members) => {
                atoms.push(Atom::Open(ContainerKind::Object));
                for (key, value) in members {
                    atoms.push(Atom::String(StringKind::Key, key.as_bytes().to_vec()));
                    value.expected(atoms);
                }
                atoms.push(Atom::Close(ContainerKind::Object));
            }
            Self::String(raw) => {
                atoms.push(Atom::String(StringKind::Value, raw.as_bytes().to_vec()))
            }
            Self::Number(raw) => atoms.push(Atom::Number(raw.as_bytes().to_vec())),
            Self::Boolean(value) => atoms.push(Atom::Boolean(*value)),
            Self::Null => atoms.push(Atom::Null),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Default)]
struct Observation {
    atoms: Vec<Atom>,
    errors: Vec<ParseError>,
    spans: Vec<Range<u64>>,
    // Normalize fragments to byte records, preserving their absolute offsets.
    trace: Vec<String>,
    offset: u64,
}

fn observe(input: &[u8], mode: Mode, cuts: &[usize]) -> Observation {
    let mut parser = Parser::with_max_depth(mode, NonZeroUsize::new(8).unwrap());
    let mut result = Observation::default();
    let mut active = None;
    let mut token = Vec::new();
    let mut emit = |event| {
        match &event {
            Event::StringFragment { bytes, offset, .. }
            | Event::NumberFragment { bytes, offset } => {
                assert_eq!(
                    *bytes,
                    &input[*offset as usize..*offset as usize + bytes.len()]
                );
                token.extend_from_slice(bytes);
                for (index, byte) in bytes.iter().enumerate() {
                    result
                        .trace
                        .push(format!("fragment {} {byte}", offset + index as u64));
                }
            }
            _ => result.trace.push(format!("{event:?}")),
        }
        match event {
            Event::CandidateStart { offset, .. } => {
                assert!(active.replace(offset).is_none());
            }
            Event::CandidateEnd { span, .. } => {
                assert_eq!(active.take(), Some(span.start));
                assert!(span.start < span.end && span.end <= input.len() as u64);
                result.spans.push(span);
            }
            Event::Invalid(error) => {
                assert_eq!(active.take(), error.candidate_start);
                assert!(error.offset <= input.len() as u64);
                result.errors.push(error);
                token.clear();
            }
            Event::ObjectStart { .. } => result.atoms.push(Atom::Open(ContainerKind::Object)),
            Event::ObjectEnd { .. } => result.atoms.push(Atom::Close(ContainerKind::Object)),
            Event::ArrayStart { .. } => result.atoms.push(Atom::Open(ContainerKind::Array)),
            Event::ArrayEnd { .. } => result.atoms.push(Atom::Close(ContainerKind::Array)),
            Event::StringStart { .. } | Event::NumberStart { .. } => token.clear(),
            Event::StringEnd { kind, .. } => result
                .atoms
                .push(Atom::String(kind, std::mem::take(&mut token))),
            Event::NumberEnd { .. } => result.atoms.push(Atom::Number(std::mem::take(&mut token))),
            Event::Boolean { value, .. } => result.atoms.push(Atom::Boolean(value)),
            Event::Null { .. } => result.atoms.push(Atom::Null),
            _ => {}
        }
    };
    let mut start = 0;
    for &end in cuts {
        parser.push(&input[start..end], &mut emit);
        start = end;
    }
    parser.push(&input[start..], &mut emit);
    // The reborrow adapts finish's Event<'static> to the closure's input
    // lifetime. Passing `emit` directly fails with E0521.
    #[allow(clippy::redundant_closure)]
    parser.finish(|event| emit(event));
    assert!(active.is_none());
    result.offset = parser.offset();
    parser.finish(|event| panic!("duplicate EOF event: {event:?}"));
    parser.push(b"[]", |event| panic!("post-EOF event: {event:?}"));
    assert_eq!(parser.offset(), result.offset);
    result
}

fn partitions(length: usize, seed: u64) -> Vec<Vec<usize>> {
    let mut generator = Generator(seed);
    let mut cuts = vec![0];
    while *cuts.last().unwrap() < length {
        cuts.push((cuts.last().unwrap() + 1 + generator.pick(11)).min(length));
    }
    vec![vec![], vec![length / 2], (0..=length).collect(), cuts]
}

#[test]
fn generated_documents_match_tree_events_across_chunks() {
    for seed in 0..128 {
        let mut generator = Generator(seed);
        let child = value(&mut generator, 3);
        let root = if seed % 2 == 0 {
            Value::Array(vec![child])
        } else {
            Value::Object(vec![("key", child)])
        };
        let input = root.render();
        let mut expected = Vec::new();
        root.expected(&mut expected);
        let baseline = observe(input.as_bytes(), Mode::Single, &[]);
        assert!(
            baseline.errors.is_empty(),
            "seed {seed}, input {input}: {baseline:?}"
        );
        assert_eq!(baseline.atoms, expected, "seed {seed}, input {input}");
        assert_eq!(baseline.spans, vec![0..input.len() as u64]);
        for cuts in partitions(input.len(), seed) {
            assert_eq!(
                observe(input.as_bytes(), Mode::Single, &cuts),
                baseline,
                "seed {seed}, input {input}, cuts {cuts:?}"
            );
        }
    }
}

#[test]
fn generated_truncations_and_mutations_reject_and_recover() {
    for seed in 0..32 {
        let root = Value::Array(vec![value(&mut Generator(seed), 2)]);
        let input = root.render().into_bytes();
        for position in 0..input.len() {
            let mut mutated = input.clone();
            mutated[position] = 0xff;
            for broken in [&input[..position], mutated.as_slice()] {
                let baseline = observe(broken, Mode::Single, &[]);
                assert!(
                    !baseline.errors.is_empty(),
                    "seed {seed}, position {position}, input {broken:?}"
                );
                assert!(baseline.spans.is_empty());
                for cuts in partitions(broken.len(), seed) {
                    assert_eq!(
                        observe(broken, Mode::Single, &cuts),
                        baseline,
                        "seed {seed}, position {position}, input {broken:?}, cuts {cuts:?}"
                    );
                }
                // NUL forces any active lexical/structural candidate to fail,
                // making a following sentinel discoverable under discard recovery.
                let mut stream = broken.to_vec();
                stream.push(0);
                let start = stream.len() as u64;
                stream.extend_from_slice(b"[true]");
                let scan = observe(&stream, Mode::Scan, &[]);
                assert_eq!(scan.spans.last(), Some(&(start..stream.len() as u64)));
                for cuts in partitions(stream.len(), seed) {
                    assert_eq!(
                        observe(&stream, Mode::Scan, &cuts),
                        scan,
                        "recovery seed {seed}, position {position}, input {stream:?}, cuts {cuts:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn deterministic_arbitrary_bytes_preserve_lifecycle_and_chunk_results() {
    const GRAMMAR: &[u8] = b"{}[]\"\\012e-+trufalsn,: \n\x00\xff\xe2\x80";
    for seed in 0..256 {
        let mut generator = Generator(seed);
        let length = generator.pick(97);
        let input: Vec<_> = (0..length)
            .map(|_| {
                if seed % 2 == 0 {
                    generator.pick(256) as u8
                } else {
                    GRAMMAR[generator.pick(GRAMMAR.len())]
                }
            })
            .collect();
        for mode in [Mode::Single, Mode::Scan] {
            let baseline = observe(&input, mode, &[]);
            for cuts in partitions(input.len(), seed) {
                assert_eq!(
                    observe(&input, mode, &cuts),
                    baseline,
                    "seed {seed}, mode {mode:?}, input {input:?}, cuts {cuts:?}"
                );
            }
        }
    }
}
