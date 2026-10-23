#![no_main]
use json_skimmer::{Event, Mode, Parser};
use libfuzzer_sys::fuzz_target;
use std::num::{NonZeroU64, NonZeroUsize};

fn observe(input: &[u8], mode: Mode, chunk: usize, depth: usize, limit: u64) -> (Vec<String>, u64) {
    let mut parser = Parser::with_limits(
        mode,
        NonZeroUsize::new(depth).unwrap(),
        NonZeroU64::new(limit),
    );
    let mut events = Vec::new();
    let mut emit = |event: Event<'_>| match event {
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
    };
    for bytes in input.chunks(chunk) {
        parser.push(bytes, &mut emit);
    }
    parser.finish(&mut emit);
    let offset = parser.offset();
    parser.finish(|event| panic!("duplicate EOF: {event:?}"));
    parser.push(b"{}", |event| panic!("post-EOF: {event:?}"));
    assert_eq!(parser.offset(), offset);
    (events, offset)
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 3 || data.len() > 4096 {
        return;
    }
    let depth = 1 + usize::from(data[0] % 32);
    let limit = if data[1] % 2 == 0 {
        0
    } else {
        1 + u64::from(data[1])
    };
    let chunk = 1 + usize::from(data[2]);
    // A sentinel after arbitrary junk exercises forward-only recovery as well
    // as lexical errors, truncation and configured resource limits.
    let mut input = data[3..].to_vec();
    input.extend_from_slice(b"\0{} []");
    for mode in [Mode::Scan, Mode::Single] {
        let whole = observe(&input, mode, input.len(), depth, limit);
        assert_eq!(whole, observe(&input, mode, 1, depth, limit));
        assert_eq!(whole, observe(&input, mode, chunk, depth, limit));
    }
});
