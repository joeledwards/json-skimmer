//! Bounded developer-run allocation/depth/stream probe; see docs/performance.md.
#[path = "support/allocation.rs"]
mod allocation;
use json_skimmer::{Event, Mode, Parser};
use std::num::NonZeroUsize;

#[global_allocator]
static ALLOCATOR: allocation::Counting = allocation::Counting;

#[derive(Debug)]
struct Report {
    bytes: usize,
    completed: usize,
    invalid: usize,
    warm: allocation::Snapshot,
    end: allocation::Snapshot,
}

fn fixture(depth: usize, token_bytes: usize) -> Vec<u8> {
    let mut bytes = vec![b'['; depth];
    bytes.push(b'"');
    // Includes UTF-8 and a raw JSON escape; fill the remainder with ASCII.
    let pattern = "é\\n".as_bytes();
    while bytes.len() - depth - 1 + pattern.len() <= token_bytes {
        bytes.extend_from_slice(pattern);
    }
    bytes.resize(depth + 1 + token_bytes, b'x');
    bytes.push(b'"');
    bytes.extend(std::iter::repeat_n(b']', depth));
    bytes
}

fn probe(
    input: &[u8],
    depth: usize,
    chunk: usize,
    candidates: usize,
) -> (Report, allocation::Snapshot) {
    allocation::measure(|| {
        let mut parser = Parser::with_max_depth(Mode::Scan, NonZeroUsize::new(depth).unwrap());
        let mut completed = 0;
        let mut invalid = 0;
        let mut observe = |event: Event<'_>| match event {
            Event::CandidateEnd { .. } => completed += 1,
            Event::Invalid(_) => invalid += 1,
            _ => {}
        };
        for part in input.chunks(chunk) {
            parser.push(part, &mut observe);
        }
        let warm = allocation::snapshot();
        for _ in 1..candidates {
            for part in input.chunks(chunk) {
                parser.push(part, &mut observe);
            }
        }
        parser.finish(&mut observe);
        let end = allocation::snapshot();
        drop(parser);
        Report {
            bytes: input.len() * candidates,
            completed,
            invalid,
            warm,
            end,
        }
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err("usage: resources DEPTH TOKEN_BYTES CANDIDATES CHUNK_BYTES".into());
    }
    let values: Vec<usize> = args[1..]
        .iter()
        .map(|value| value.parse())
        .collect::<Result<_, _>>()?;
    let (depth, token, candidates, chunk) = (values[0], values[1], values[2], values[3]);
    if !(1..=4096).contains(&depth)
        || token > 16 * 1024 * 1024
        || candidates == 0
        || candidates > 1_000_000
        || chunk == 0
        || chunk > 1024 * 1024
    {
        return Err(
            "bounds: depth 1..4096, token 0..16MiB, candidates 1..1000000, chunk 1..1MiB".into(),
        );
    }
    let input = fixture(depth, token);
    let (report, released) = probe(&input, depth, chunk, candidates);
    println!(
        "{{\"depth\":{depth},\"token_bytes\":{token},\"candidates\":{candidates},\"chunk_bytes\":{chunk},\"input_bytes\":{},\"completed\":{},\"invalid\":{},\"allocation_calls\":{},\"requested_bytes\":{},\"peak_live_bytes\":{},\"retained_after_first\":{},\"retained_at_end\":{},\"calls_after_first\":{},\"live_after_drop\":{}}}",
        report.bytes,
        report.completed,
        report.invalid,
        report.end.calls,
        report.end.requested,
        report.end.peak,
        report.warm.live,
        report.end.live,
        report.end.calls - report.warm.calls,
        released.live
    );
    if report.completed != candidates
        || report.invalid != 0
        || released.live != 0
        || released.accounting_error
    {
        return Err("candidate counts or parser allocation cleanup failed".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_events_and_allocations_across_depth_tokens_and_chunks() {
        for depth in [1, 8, 64] {
            let mut retained = None;
            for token in [0, 9, 4096] {
                let input = fixture(depth, token);
                for chunk in [1, 7, 8192] {
                    let (report, released) = probe(&input, depth, chunk, 8);
                    assert_eq!((report.completed, report.invalid), (8, 0));
                    assert_eq!(report.end.live, report.warm.live);
                    assert_eq!(report.end.calls, report.warm.calls);
                    assert_eq!(released.live, 0);
                    if let Some(previous) = retained {
                        assert_eq!(report.end.live, previous);
                    }
                    retained = Some(report.end.live);
                }
            }
        }
    }

    #[test]
    fn invalid_and_truncated_candidates_release_storage() {
        for input in [b"[[bad]]".as_slice(), b"[[\"unfinished".as_slice()] {
            for chunk in 1..=input.len() {
                let (report, released) = probe(input, 8, chunk, 1);
                assert_eq!(report.completed, 0);
                assert!(report.invalid > 0);
                assert_eq!(released.live, 0);
            }
        }
    }

    #[test]
    fn allocator_accounts_for_allocation_growth_and_release() {
        let (_, final_state) = allocation::measure(|| {
            let mut bytes = Vec::with_capacity(8);
            bytes.resize(4096, 1_u8);
            std::hint::black_box(&bytes);
            let state = allocation::snapshot();
            assert!(state.live >= 4096);
            assert!(state.calls >= 2);
            assert!(state.requested >= state.live);
            drop(bytes);
        });
        assert_eq!(final_state.live, 0);
    }
}
