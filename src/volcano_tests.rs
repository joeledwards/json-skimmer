//! Published-generator fixtures exercised through the parser API.
use crate::{Event, Mode, Parser};
use json_volcano::{Config, Generator, Scenario, Shape};
use std::io::Read;

fn config(shape: Shape, scenario: Scenario) -> Config {
    Config {
        bytes: 512,
        candidates: 1,
        seed: 42,
        depth: 2,
        escape_percent: 15,
        unicode_percent: 15,
        shape,
        scenario,
    }
}
fn bytes(config: Config, chunk: usize) -> Vec<u8> {
    let mut source = Generator::new(config).unwrap();
    let mut all = Vec::new();
    let mut buffer = vec![0; chunk];
    loop {
        // Empty reads must neither consume input nor indicate a persistent EOF.
        assert_eq!(source.read(&mut []).unwrap(), 0);
        let n = source.read(&mut buffer).unwrap();
        if n == 0 {
            break;
        }
        all.extend_from_slice(&buffer[..n]);
    }
    assert_eq!(source.read(&mut buffer).unwrap(), 0);
    all
}
#[derive(Debug, Default, PartialEq, Eq)]
struct Observation {
    events: Vec<String>,
    tokens: Vec<Vec<u8>>,
    completed: usize,
    errors: Vec<crate::ParseError>,
}
fn observe(input: &[u8], cuts: &[usize], mode: Mode) -> Observation {
    let mut parser = Parser::new(mode);
    let mut seen = Observation::default();
    let mut record = |event: Event<'_>| {
        match &event {
            Event::StringStart { .. } | Event::NumberStart { .. } => seen.tokens.push(Vec::new()),
            Event::StringFragment { bytes, .. } | Event::NumberFragment { bytes, .. } => {
                seen.tokens.last_mut().unwrap().extend_from_slice(bytes);
                return;
            }
            Event::CandidateEnd { .. } => seen.completed += 1,
            Event::Invalid(error) => seen.errors.push(error.clone()),
            _ => (),
        }
        seen.events.push(format!("{event:?}"));
    };
    let mut previous = 0;
    for end in cuts.iter().copied().chain(std::iter::once(input.len())) {
        parser.push(&input[previous..end], &mut record);
        previous = end;
    }
    parser.finish(&mut record);
    seen
}

#[test]
fn exact_sizes_reproducibility_and_candidate_lifecycle_across_chunks() {
    for shape in [
        Shape::Records,
        Shape::String,
        Shape::Array,
        Shape::Object,
        Shape::Mixed,
    ] {
        for scenario in [
            Scenario::Valid,
            Scenario::Invalid,
            Scenario::Mixed,
            Scenario::Truncated,
        ] {
            let mut cfg = config(shape, scenario);
            cfg.candidates = if scenario == Scenario::Truncated {
                1
            } else {
                3
            };
            cfg.bytes = 611; // Uneven candidate sizes, some too small for complete records.
            let expected = bytes(cfg.clone(), 8192);
            assert_eq!(expected.len(), 611);
            assert_eq!(expected, bytes(cfg.clone(), 1));
            assert_eq!(expected, bytes(cfg.clone(), 17));
            let baseline = observe(&expected, &[], Mode::Scan);
            let (completed, invalid) = match scenario {
                Scenario::Valid => (3, 0),
                Scenario::Invalid => (0, 3),
                Scenario::Mixed => (2, 1),
                Scenario::Truncated => (0, 1),
            };
            assert_eq!(
                (baseline.completed, baseline.errors.len()),
                (completed, invalid),
                "{shape:?} {scenario:?}"
            );
            if scenario == Scenario::Truncated {
                assert_eq!(baseline.errors[0].kind, crate::ErrorKind::UnexpectedEnd);
            }
            for split in 0..=expected.len() {
                assert_eq!(
                    observe(&expected, &[split], Mode::Scan),
                    baseline,
                    "{shape:?} {scenario:?} split={split}"
                );
            }
            assert_eq!(
                observe(
                    &expected,
                    &(0..expected.len()).collect::<Vec<_>>(),
                    Mode::Scan
                ),
                baseline
            );
        }
    }
}

#[test]
fn scalar_tokens_unicode_escapes_numbers_depth_and_seed() {
    for shape in [
        Shape::Records,
        Shape::String,
        Shape::Array,
        Shape::Object,
        Shape::Mixed,
    ] {
        for (escape, unicode) in [(0, 0), (100, 0), (0, 100), (50, 50)] {
            let mut cfg = config(shape, Scenario::Valid);
            cfg.bytes = 8193;
            cfg.depth = 4;
            cfg.escape_percent = escape;
            cfg.unicode_percent = unicode;
            let output = bytes(cfg.clone(), 31);
            let baseline = observe(&output, &[], Mode::Single);
            assert_eq!(baseline.completed, 1);
            assert!(baseline.errors.is_empty());
            assert!(std::str::from_utf8(&output).is_ok());
            assert_eq!(
                observe(
                    &output,
                    &(0..output.len()).collect::<Vec<_>>(),
                    Mode::Single
                ),
                baseline
            );
            if unicode > 0 {
                assert!(output.iter().any(|b| *b >= 128));
            }
            if escape > 0 {
                assert!(output.contains(&b'\\'));
            }
            if shape == Shape::Object {
                assert!(
                    !baseline
                        .events
                        .iter()
                        .any(|event| event.starts_with("ArrayStart"))
                );
            }
            if shape == Shape::Array {
                assert!(
                    !baseline
                        .events
                        .iter()
                        .any(|event| event.starts_with("ObjectStart"))
                );
            }
            if shape == Shape::Records {
                assert!(output.windows(7).any(|w| w == b"\"none\":"));
                assert!(baseline.events.iter().any(|e| e.starts_with("NumberStart")));
            }
            cfg.seed += 1;
            assert_ne!(output, bytes(cfg, 31));
        }
    }
}

#[test]
fn streamed_roots_respect_depth_caps_and_depth_biases_across_chunks() {
    use json_volcano::{Bias, StreamConfig, StreamGenerator, StreamMode};
    for shape in [Shape::Array, Shape::Object, Shape::Mixed, Shape::String] {
        for maximum in [1, 4, 128] {
            let mut sums = Vec::new();
            for bias in [Bias::Smaller, Bias::Uniform, Bias::Larger] {
                let config = StreamConfig {
                    fixture: Config {
                        bytes: 2048,
                        candidates: 1,
                        shape,
                        ..Config::default()
                    },
                    mode: StreamMode::Infinite,
                    max_depth: Some(maximum),
                    depth_bias: bias,
                    ..StreamConfig::default()
                };
                let mut input = Vec::new();
                StreamGenerator::new(config.clone())
                    .unwrap()
                    .take(2048 * 32)
                    .read_to_end(&mut input)
                    .unwrap();
                let mut parser = Parser::with_max_depth(
                    Mode::Scan,
                    std::num::NonZeroUsize::new(maximum).unwrap(),
                );
                let mut depth = 0;
                let mut peak = 0;
                let mut sum = 0;
                let mut completed = 0;
                parser.push(&input, |event| match event {
                    Event::ObjectStart { .. } | Event::ArrayStart { .. } => {
                        depth += 1;
                        peak = peak.max(depth);
                    }
                    Event::ObjectEnd { .. } | Event::ArrayEnd { .. } => depth -= 1,
                    Event::CandidateEnd { .. } => {
                        assert!(peak <= maximum);
                        sum += peak;
                        peak = 0;
                        completed += 1;
                    }
                    Event::Invalid(error) => {
                        panic!("{shape:?} maximum={maximum} {bias:?}: {error:?}")
                    }
                    _ => (),
                });
                parser.finish(|event| {
                    assert!(!matches!(event, Event::Invalid(_)));
                });
                assert_eq!(completed, 32);
                assert_eq!(depth, 0);
                sums.push(sum);
                // A complete individual candidate also has normalized events
                // independent of every split, including UTF-8 and escapes.
                if maximum <= 4 {
                    let candidate = &input[..2048];
                    let baseline = observe(candidate, &[], Mode::Single);
                    assert!(baseline.errors.is_empty());
                    for split in 0..=candidate.len() {
                        assert_eq!(observe(candidate, &[split], Mode::Single), baseline);
                    }
                    assert_eq!(
                        observe(
                            candidate,
                            &(0..candidate.len()).collect::<Vec<_>>(),
                            Mode::Single
                        ),
                        baseline
                    );
                }
                let mut singles = Vec::new();
                let mut source = StreamGenerator::new(config)
                    .unwrap()
                    .take(input.len() as u64);
                let mut byte = [0];
                while source.read(&mut byte).unwrap() != 0 {
                    singles.push(byte[0]);
                }
                assert_eq!(singles, input);
            }
            assert!(
                sums[0] <= sums[1] && sums[1] <= sums[2],
                "{shape:?} {maximum}: {sums:?}"
            );
            if maximum > 1 {
                assert!(sums[0] < sums[2]);
            }
        }
    }
}

#[test]
fn record_string_sizes_obey_bounds_and_size_biases() {
    use json_volcano::{Bias, StreamConfig, StreamGenerator, ValueSizes};
    let mut averages = Vec::new();
    for bias in [Bias::Smaller, Bias::Uniform, Bias::Larger] {
        let config = StreamConfig {
            fixture: Config {
                bytes: 32 * 4096,
                candidates: 32,
                shape: Shape::Object,
                escape_percent: 0,
                unicode_percent: 0,
                ..Config::default()
            },
            value_sizes: Some(ValueSizes {
                min_tokens: 1,
                max_tokens: 128,
                bias,
            }),
            ..StreamConfig::default()
        };
        let mut input = Vec::new();
        StreamGenerator::new(config)
            .unwrap()
            .read_to_end(&mut input)
            .unwrap();
        assert_eq!(input.len(), 32 * 4096);
        let mut lengths = Vec::new();
        let mut current = 0;
        let mut parser = Parser::new(Mode::Scan);
        parser.push(&input, |event| match event {
            Event::StringStart {
                kind: crate::StringKind::Value,
                ..
            } => current = 0,
            Event::StringFragment {
                kind: crate::StringKind::Value,
                bytes,
                ..
            } => current += bytes.len(),
            Event::StringEnd {
                kind: crate::StringKind::Value,
                ..
            } => lengths.push(current),
            Event::Invalid(error) => panic!("{error:?}"),
            _ => (),
        });
        parser.finish(|event| assert!(!matches!(event, Event::Invalid(_))));
        assert!(!lengths.is_empty());
        assert!(lengths.iter().all(|length| (1..=128).contains(length)));
        averages.push(lengths.iter().sum::<usize>() as f64 / lengths.len() as f64);
    }
    assert!(
        averages[0] < averages[1] && averages[1] < averages[2],
        "{averages:?}"
    );
}

#[test]
fn streamed_finite_variations_preserve_lifecycle_across_every_split() {
    use json_volcano::{Bias, StreamConfig, StreamGenerator, ValueSizes};
    for scenario in [
        Scenario::Valid,
        Scenario::Invalid,
        Scenario::Mixed,
        Scenario::Truncated,
    ] {
        let config = StreamConfig {
            fixture: Config {
                bytes: 611,
                candidates: if scenario == Scenario::Truncated {
                    1
                } else {
                    3
                },
                shape: Shape::Mixed,
                scenario,
                ..Config::default()
            },
            max_depth: Some(4),
            depth_bias: Bias::Smaller,
            value_sizes: Some(ValueSizes {
                min_tokens: 0,
                max_tokens: 16,
                bias: Bias::Larger,
            }),
            ..StreamConfig::default()
        };
        let mut input = Vec::new();
        StreamGenerator::new(config)
            .unwrap()
            .read_to_end(&mut input)
            .unwrap();
        assert_eq!(input.len(), 611);
        let baseline = observe(&input, &[], Mode::Scan);
        let expected = match scenario {
            Scenario::Valid => (3, 0),
            Scenario::Invalid => (0, 3),
            Scenario::Mixed => (2, 1),
            Scenario::Truncated => (0, 1),
        };
        assert_eq!((baseline.completed, baseline.errors.len()), expected);
        for split in 0..=input.len() {
            assert_eq!(observe(&input, &[split], Mode::Scan), baseline);
        }
        assert_eq!(
            observe(&input, &(0..input.len()).collect::<Vec<_>>(), Mode::Scan),
            baseline
        );
    }
}
