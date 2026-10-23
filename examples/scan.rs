//! Scan stdin without retaining complete documents or scalar tokens.
use std::io::{self, Read};
use std::num::{NonZeroU64, NonZeroUsize};

use json_skimmer::{Event, Mode, Parser};

fn main() -> io::Result<()> {
    scan(io::stdin().lock())
}

fn scan(mut input: impl Read) -> io::Result<()> {
    let mut parser = Parser::with_limits(
        Mode::Scan,
        NonZeroUsize::new(1000).unwrap(),
        NonZeroU64::new(16 * 1024 * 1024),
    );
    let mut buffer = [0; 8192];
    let mut candidate_bytes = 0_u64;
    let mut observe = |event: Event<'_>| match event {
        Event::CandidateStart { .. } => candidate_bytes = 0,
        // Fragments borrow the current input buffer and may split UTF-8 or
        // escapes. Their contents are speculative until CandidateEnd.
        Event::StringFragment { bytes, .. } | Event::NumberFragment { bytes, .. } => {
            candidate_bytes += bytes.len() as u64;
        }
        Event::CandidateEnd { span, .. } => {
            eprintln!("valid candidate {span:?}: {candidate_bytes} raw token bytes");
        }
        Event::Invalid(error) => {
            candidate_bytes = 0; // Discard speculative candidate statistics.
            eprintln!("discarded candidate: {error:?}");
        }
        _ => {}
    };
    loop {
        match input.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => parser.push(&buffer[..count], &mut observe),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    // Required even after the last successful read: detects truncated tokens
    // and containers, then permanently closes the parser.
    parser.finish(&mut observe);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_valid_invalid_and_truncated_candidates() {
        scan(b"noise {\"a\":123} [invalid] [\"truncated".as_slice()).unwrap();
    }

    #[test]
    fn read_errors_propagate() {
        struct Failed;
        impl Read for Failed {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::other("input failed"))
            }
        }
        assert_eq!(scan(Failed).unwrap_err().to_string(), "input failed");
    }
}
