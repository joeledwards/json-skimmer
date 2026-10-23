//! Bounded development probe used by tools/reference_check.py.
//! Hex input lines in; strict acceptance and scan spans as JSON lines out.
use std::io::{self, BufRead};

use json_skimmer::{Event, Mode, Parser};

fn observe(input: &[u8], mode: Mode, chunks: &[usize]) -> (bool, Vec<(u64, u64)>, Vec<String>) {
    let mut parser = Parser::new(mode);
    let mut invalid = false;
    let mut spans = Vec::new();
    let mut events = Vec::new();
    let mut emit = |event: Event<'_>| {
        match &event {
            Event::Invalid(_) => invalid = true,
            Event::CandidateEnd { span, .. } => spans.push((span.start, span.end)),
            _ => {}
        }
        match event {
            Event::StringFragment {
                kind,
                bytes,
                offset,
            } => {
                for (index, byte) in bytes.iter().enumerate() {
                    events.push(format!("string {kind:?} {} {byte}", offset + index as u64));
                }
            }
            Event::NumberFragment { bytes, offset } => {
                for (index, byte) in bytes.iter().enumerate() {
                    events.push(format!("number {} {byte}", offset + index as u64));
                }
            }
            other => events.push(format!("{other:?}")),
        }
    };
    let mut start = 0;
    for &end in chunks {
        parser.push(&input[start..end], &mut emit);
        start = end;
    }
    parser.push(&input[start..], &mut emit);
    parser.finish(&mut emit);
    let offset = parser.offset();
    parser.finish(|event| panic!("repeated EOF: {event:?}"));
    parser.push(b"{}", |event| panic!("post-EOF: {event:?}"));
    assert_eq!(offset, parser.offset());
    (!invalid, spans, events)
}

// Keep this development probe compatible with compilers before slice::as_chunks.
#[allow(clippy::chunks_exact_to_as_chunks)]
fn main() -> io::Result<()> {
    for line in io::stdin().lock().lines() {
        let line = line?;
        assert!(
            line.len() <= 8192 && line.len() % 2 == 0,
            "bounded hex input required"
        );
        let input: Vec<u8> = line
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        let strict = observe(&input, Mode::Single, &[]);
        let scan = observe(&input, Mode::Scan, &[]);
        for (mode, baseline) in [(Mode::Single, &strict), (Mode::Scan, &scan)] {
            // Every two-chunk split and one-byte chunks, normalizing fragments.
            for split in 0..=input.len() {
                assert_eq!(
                    &observe(&input, mode, &[split]),
                    baseline,
                    "split {split}: {input:?}"
                );
            }
            assert_eq!(
                &observe(&input, mode, &(0..=input.len()).collect::<Vec<_>>()),
                baseline
            );
        }
        print!("{{\"strict\":{},\"spans\":[", strict.0);
        for (index, (start, end)) in scan.1.iter().enumerate() {
            if index > 0 {
                print!(",");
            }
            print!("[{start},{end}]");
        }
        println!("]}}");
    }
    Ok(())
}
