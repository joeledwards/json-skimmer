//! Developer-run parser measurement. See docs/performance.md.
use json_skimmer::{Event, Mode, Parser};
use std::io::{self, Read};
use std::time::Instant;

#[derive(Debug, Default, PartialEq, Eq)]
struct Counts {
    input_bytes: u64,
    token_bytes: u64,
    candidates: u64,
    invalid: u64,
}

fn scan(mut input: impl Read, chunk: usize) -> io::Result<Counts> {
    let mut parser = Parser::new(Mode::Scan);
    let mut counts = Counts::default();
    let mut buffer = vec![0; chunk];
    let mut observe = |event: Event<'_>| match event {
        Event::StringFragment { bytes, .. } | Event::NumberFragment { bytes, .. } => {
            counts.token_bytes += bytes.len() as u64;
        }
        Event::CandidateEnd { .. } => counts.candidates += 1,
        Event::Invalid(_) => counts.invalid += 1,
        _ => {}
    };
    loop {
        match input.read(&mut buffer) {
            Ok(0) => break,
            Ok(size) => {
                counts.input_bytes += size as u64;
                parser.push(&buffer[..size], &mut observe);
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    parser.finish(&mut observe);
    Ok(counts)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 || !matches!(args[1].as_str(), "memory" | "stream") {
        return Err("usage: measure memory|stream FILE CHUNK_BYTES REPEATS".into());
    }
    let chunk: usize = args[3].parse()?;
    let repeats: usize = args[4].parse()?;
    if chunk == 0 || repeats < 3 {
        return Err("chunk must be positive and repeats must be at least three".into());
    }
    // Loading is excluded from internal parse timing. Process RSS includes it.
    let data = if args[1] == "memory" {
        Some(std::fs::read(&args[2])?)
    } else {
        None
    };
    let mut expected = None;
    for iteration in 1..=repeats {
        // File open is excluded; reads and EOF handling are included.
        let mut input: Box<dyn Read> = match &data {
            Some(bytes) => Box::new(bytes.as_slice()),
            None => Box::new(std::fs::File::open(&args[2])?),
        };
        let started = Instant::now();
        let counts = scan(&mut input, chunk)?;
        let nanos = started.elapsed().as_nanos();
        if counts.invalid != 0 || counts.candidates == 0 {
            return Err(format!("expected valid container input: {counts:?}").into());
        }
        if let Some(previous) = &expected {
            if previous != &counts {
                return Err("counts changed between repetitions".into());
            }
        }
        println!(
            "{{\"iteration\":{iteration},\"elapsed_ns\":{nanos},\"input_bytes\":{},\"token_bytes\":{},\"candidates\":{}}}",
            counts.input_bytes, counts.token_bytes, counts.candidates
        );
        expected = Some(counts);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_are_independent_of_chunk_boundaries() {
        for input in [
            br#"{"a":"\u1234","b":12} []"#.as_slice(),
            "[\"é\"] [bad] [\"unfinished".as_bytes(),
        ] {
            let expected = scan(input, input.len()).unwrap();
            for split in 1..input.len() {
                struct Split<'a> {
                    bytes: &'a [u8],
                    first: usize,
                }
                impl Read for Split<'_> {
                    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
                        let n = self.bytes.len().min(out.len()).min(self.first);
                        out[..n].copy_from_slice(&self.bytes[..n]);
                        self.bytes = &self.bytes[n..];
                        self.first = usize::MAX;
                        Ok(n)
                    }
                }
                assert_eq!(
                    scan(
                        Split {
                            bytes: input,
                            first: split
                        },
                        input.len()
                    )
                    .unwrap(),
                    expected
                );
            }
            assert_eq!(scan(input, 1).unwrap(), expected);
        }
        let counts = scan(br#"{"a":"xyz","b":12} []"#.as_slice(), 3).unwrap();
        assert_eq!(counts.candidates, 2);
        assert_eq!(counts.invalid, 0);
        assert_eq!(counts.token_bytes, 7);
        assert_eq!(scan(b"[".as_slice(), 1).unwrap().invalid, 1);
        assert!(scan(b"[bad]".as_slice(), 1).unwrap().invalid > 0);
    }

    #[test]
    fn read_failures_propagate() {
        struct Failed;
        impl Read for Failed {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::other("read failed"))
            }
        }
        assert_eq!(scan(Failed, 8).unwrap_err().to_string(), "read failed");
    }
}
