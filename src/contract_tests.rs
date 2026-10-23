use super::*;

#[test]
fn finish_is_final_and_idempotent_for_all_candidate_outcomes() {
    for mode in [Mode::Scan, Mode::Single] {
        for input in [
            b"".as_slice(),
            b"{}",
            b"[1",
            b"[\"\xe2",
            b"[true]",
            b"[truX]",
        ] {
            let mut parser = Parser::new(mode);
            parser.push(input, |_| {});
            let mut first_finish = Vec::new();
            parser.finish(|event| record(&mut first_finish, event));
            if input == b"[1" {
                assert_eq!(first_finish.len(), 2); // NumberEnd, then UnexpectedEnd.
                assert!(first_finish[0].starts_with("NumberEnd"));
                assert!(first_finish[1].contains("UnexpectedEnd"));
            }
            let offset = parser.offset();
            for _ in 0..3 {
                parser.finish(|event| panic!("duplicate finish event: {event:?}"));
                parser.push(b"{} [] garbage", |event| {
                    panic!("post-finish event: {event:?}")
                });
                parser.push(b"", |event| panic!("post-finish empty event: {event:?}"));
                assert_eq!(parser.offset(), offset);
            }
        }
    }
}

#[test]
fn terminal_errors_freeze_offset_after_consuming_the_offending_byte() {
    for (input, expected_offset) in [
        (b"x{}".as_slice(), 1),
        (b"{} []", 4),
        (b"[truX] {}", 5),
        (b"[\"\xff\"] {}", 3),
    ] {
        for split in 0..=input.len() {
            let mut parser = Parser::new(Mode::Single);
            let mut errors = Vec::new();
            for chunk in [&input[..split], &input[split..]] {
                parser.push(chunk, |event| {
                    if let Event::Invalid(error) = event {
                        errors.push(error);
                    }
                });
            }
            assert_eq!(errors.len(), 1);
            assert_eq!(parser.offset(), expected_offset, "{input:?}, split {split}");
            parser.push(b"[]", |event| panic!("terminal event: {event:?}"));
            parser.finish(|event| panic!("terminal finish event: {event:?}"));
            assert_eq!(parser.offset(), expected_offset);
        }
    }
    let mut parser = Parser::with_max_depth(Mode::Single, std::num::NonZeroUsize::new(1).unwrap());
    parser.push(b"[[]] {}", |_| {});
    assert_eq!(parser.offset(), 2);
    parser.push(b"[]", |_| panic!("terminal depth error resumed"));
    assert_eq!(parser.offset(), 2);
}

fn nested_containers(depth: usize, shape: &str) -> (Vec<u8>, Vec<u64>) {
    let mut input = Vec::new();
    let mut openers = Vec::new();
    let mut closers = Vec::new();
    for level in 0..depth {
        openers.push(input.len() as u64);
        if shape == "objects" || (shape == "mixed" && level % 2 == 1) {
            input.extend_from_slice(br#"{"k":"#);
            closers.push(b'}');
        } else {
            input.push(b'[');
            closers.push(b']');
        }
    }
    input.extend_from_slice(b"null");
    input.extend(closers.into_iter().rev());
    (input, openers)
}

#[test]
fn configurable_depth_limits_count_only_open_containers() {
    for shape in ["arrays", "objects", "mixed"] {
        for limit in 1_usize..=4 {
            for depth in [limit.saturating_sub(1).max(1), limit, limit + 1] {
                let (input, openers) = nested_containers(depth, shape);
                let maximum = std::num::NonZeroUsize::new(limit).unwrap();
                let (events, errors, spans) = parse_with_depth(&input, Mode::Single, &[], maximum);
                let starts = events
                    .iter()
                    .filter(|event| {
                        event.starts_with("ObjectStart") || event.starts_with("ArrayStart")
                    })
                    .count();
                assert_eq!(starts, depth.min(limit));
                if depth <= limit {
                    assert!(errors.is_empty(), "{shape}, {depth}, {limit}: {errors:?}");
                    assert_eq!(spans, vec![0..input.len() as u64]);
                } else {
                    assert_eq!(
                        errors,
                        vec![ParseError {
                            kind: ErrorKind::DepthLimitExceeded,
                            candidate_start: Some(0),
                            offset: openers[limit],
                        }]
                    );
                    assert!(spans.is_empty());
                }
                assert_depth_chunk_invariance(&input, Mode::Single, maximum);
            }
        }
    }
    // Sequential siblings do not accumulate depth, and scalars add none.
    for input in [b"[1,2,3,null,true,\"text\"]".as_slice(), b"[[],{},[],{}]"] {
        let maximum = std::num::NonZeroUsize::new(if input[1] == b'1' { 1 } else { 2 }).unwrap();
        assert!(
            parse_with_depth(input, Mode::Single, &[], maximum)
                .1
                .is_empty()
        );
        assert_depth_chunk_invariance(input, Mode::Single, maximum);
    }
}

#[test]
fn default_depth_is_1000_and_can_be_raised() {
    assert_eq!(DEFAULT_MAX_DEPTH.get(), 1000);
    for depth in [999, 1000, 1001] {
        let (input, _) = nested_containers(depth, "mixed");
        for cuts in [vec![], (0..=input.len()).collect()] {
            let (_, errors, spans) = parse(&input, Mode::Single, &cuts);
            assert_eq!(errors.is_empty(), depth <= 1000);
            assert_eq!(spans.len(), usize::from(depth <= 1000));
            let (_, errors, spans) = parse_with_depth(
                &input,
                Mode::Single,
                &cuts,
                std::num::NonZeroUsize::new(1001).unwrap(),
            );
            assert!(errors.is_empty());
            assert_eq!(spans, vec![0..input.len() as u64]);
        }
    }
}

#[test]
fn depth_recovery_consumes_rejected_opener_and_resets_for_later_candidates() {
    let input = b"[[]] {\"ok\":true} []";
    let maximum = std::num::NonZeroUsize::new(1).unwrap();
    let (events, errors, spans) = parse_with_depth(input, Mode::Scan, &[], maximum);
    assert_eq!(
        errors,
        vec![ParseError {
            kind: ErrorKind::DepthLimitExceeded,
            candidate_start: Some(0),
            offset: 1,
        }]
    );
    assert_eq!(spans, vec![5..16, 17..19]);
    assert!(
        !events
            .iter()
            .any(|event| event == "ArrayStart { offset: 1 }")
    );
    assert_depth_chunk_invariance(input, Mode::Scan, maximum);
    let (_, errors, spans) = parse_with_depth(input, Mode::Single, &[], maximum);
    assert_eq!(errors.len(), 1);
    assert!(spans.is_empty());
    assert_depth_chunk_invariance(input, Mode::Single, maximum);
    // Later openers may be discovered; discard recovery does not skip a document.
    let (_, errors, spans) = parse_with_depth(b"[[[]]]", Mode::Scan, &[], maximum);
    assert_eq!(errors.len(), 1);
    assert_eq!(spans, vec![2..4]);
}

// Use Rust's encoder as an independent source of common text and boundary cases.
fn utf8_sequences() -> Vec<(char, Vec<u8>)> {
    [
        '\u{80}',
        '¢',
        'é',
        '\u{7ff}',
        '\u{800}',
        '€',
        '雪',
        '\u{d7ff}',
        '\u{e000}',
        '\u{ffff}',
        '\u{10000}',
        '😀',
        '\u{10ffff}',
    ]
    .into_iter()
    .map(|character| (character, character.to_string().into_bytes()))
    .collect()
}

fn string_documents(payload: &[u8]) -> Vec<Vec<u8>> {
    [(b"[\"".as_slice(), b"\"]".as_slice()), (b"{\"", b"\":0}")]
        .into_iter()
        .map(|(prefix, suffix)| [prefix, payload, suffix].concat())
        .collect()
}

#[test]
fn every_continuation_replacement_agrees_with_rust_utf8_validation() {
    for (character, original) in utf8_sequences() {
        for position in 1..original.len() {
            for replacement in 0..=u8::MAX {
                let mut payload = original.clone();
                payload[position] = replacement;
                let valid = std::str::from_utf8(&payload).is_ok();
                for input in string_documents(&payload) {
                    // Bound this generated corpus to whole-input and one-byte
                    // chunks; focused fixtures above/below exercise every split.
                    for cuts in [vec![], (0..=input.len()).collect()] {
                        let (_, errors, spans) = parse(&input, Mode::Single, &cuts);
                        let context = format!(
                            "{character:?}, byte {position}, replacement {replacement:#x}, cuts {cuts:?}"
                        );
                        if valid {
                            assert!(errors.is_empty(), "{context}: {errors:?}");
                            assert_eq!(spans, vec![0..input.len() as u64], "{context}");
                        } else {
                            assert_eq!(
                                errors,
                                vec![ParseError {
                                    kind: ErrorKind::InvalidUtf8,
                                    candidate_start: Some(0),
                                    offset: (2 + position) as u64,
                                }],
                                "{context}"
                            );
                            assert!(spans.is_empty(), "{context}");
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn every_multibyte_prefix_is_rejected_at_quote_and_at_eof() {
    for (character, bytes) in utf8_sequences() {
        for length in 1..bytes.len() {
            for input in string_documents(&bytes[..length]) {
                for at_eof in [false, true] {
                    let input = if at_eof {
                        &input[..2 + length]
                    } else {
                        &input[..]
                    };
                    let (_, errors, spans) = parse(input, Mode::Single, &[]);
                    assert_eq!(
                        errors,
                        vec![ParseError {
                            kind: ErrorKind::InvalidUtf8,
                            candidate_start: Some(0),
                            offset: (2 + length) as u64,
                        }],
                        "{character:?}, prefix length {length}, EOF {at_eof}"
                    );
                    assert!(spans.is_empty());
                    assert_chunk_invariance(input, Mode::Single);
                }
            }
        }
    }
}

#[test]
fn common_international_text_and_escapes_preserve_raw_fragments() {
    for payload in [
        "café naïve résumé",
        "你好，世界",
        "مرحبا بالعالم",
        "नमस्ते",
        "😀🌍🚀",
        "e\u{301}",
        "\u{feff}text",
        "a¢€😀z",
        r#"雪\n😀\u0041é\"\\終"#,
    ] {
        for input in string_documents(payload.as_bytes()) {
            let mut parser = Parser::new(Mode::Single);
            let mut fragments = Vec::new();
            let mut completed = 0;
            for byte in input.chunks(1) {
                parser.push(byte, |event| match event {
                    Event::StringFragment { bytes, .. } => fragments.extend_from_slice(bytes),
                    Event::CandidateEnd { .. } => completed += 1,
                    Event::Invalid(error) => panic!("{payload:?}: {error:?}"),
                    _ => {}
                });
            }
            parser.finish(|event| assert!(!matches!(event, Event::Invalid(_))));
            assert_eq!(completed, 1);
            assert_eq!(fragments, payload.as_bytes());
            assert_chunk_invariance(&input, Mode::Single);
        }
    }
}

#[test]
fn repeated_utf8_failures_do_not_contaminate_later_candidates() {
    let candidates = [
        b"[\"\xc2\"]".as_slice(),
        b"{\"\xff\":0}",
        b"[\"\xe0\x80\x80\"]",
    ];
    let mut input = b"\xfflog ".to_vec();
    let mut expected_errors = Vec::new();
    for (index, candidate) in candidates.into_iter().enumerate() {
        let start = input.len() as u64;
        expected_errors.push(ParseError {
            kind: ErrorKind::InvalidUtf8,
            candidate_start: Some(start),
            offset: start + if index == 1 { 2 } else { 3 },
        });
        input.extend_from_slice(candidate);
        input.push(b' ');
    }
    let valid_start = input.len() as u64;
    input.extend_from_slice("{\"雪\":\"😀\"}".as_bytes());
    let (_, errors, spans) = parse(&input, Mode::Scan, &[]);
    assert_eq!(errors, expected_errors);
    assert_eq!(spans, vec![valid_start..input.len() as u64]);
    assert_chunk_invariance(&input, Mode::Scan);
}

#[test]
fn malformed_utf8_cannot_complete_a_candidate() {
    for input in [
        b"[\"\xff\"]".as_slice(),
        b"[\"\xc0\xaf\"]",
        b"[\"\xe2\x82\"]",
    ] {
        let (_, errors, spans) = parse(input, Mode::Single, &[]);
        assert!(!errors.is_empty(), "accepted invalid UTF-8: {input:?}");
        assert!(spans.is_empty());
        assert_chunk_invariance(input, Mode::Single);
    }
}

#[test]
fn utf8_boundaries_and_invalid_sequences_have_precise_offsets() {
    let cases: &[(&[u8], Option<usize>)] = &[
        (b"\xc2\x80", None),
        (b"\xdf\xbf", None),
        (b"\xe0\xa0\x80", None),
        (b"\xed\x9f\xbf", None),
        (b"\xee\x80\x80", None),
        (b"\xef\xbf\xbf", None),
        (b"\xf0\x90\x80\x80", None),
        (b"\xf4\x8f\xbf\xbf", None),
        (b"\x80", Some(0)),
        (b"\xbf", Some(0)),
        (b"\xc0\xaf", Some(0)),
        (b"\xc1\xbf", Some(0)),
        (b"\xf5\x80\x80\x80", Some(0)),
        (b"\xfe", Some(0)),
        (b"\xff", Some(0)),
        (b"\xe0\x9f\xbf", Some(1)),
        (b"\xed\xa0\x80", Some(1)),
        (b"\xf0\x8f\xbf\xbf", Some(1)),
        (b"\xf4\x90\x80\x80", Some(1)),
        (b"\xc2x", Some(1)),
        (b"\xe2\x82x", Some(2)),
        (b"\xf0\x90\x80x", Some(3)),
        (b"\xc2", Some(1)),
        (b"\xe2\x82", Some(2)),
        (b"\xf0\x90\x80", Some(3)),
        (b"\xc2\\n", Some(1)),
        (b"\xc2\n", Some(1)),
    ];
    for &(bytes, invalid_at) in cases {
        assert_eq!(std::str::from_utf8(bytes).is_ok(), invalid_at.is_none());
        for (prefix, suffix) in [(b"[\"".as_slice(), b"\"]".as_slice()), (b"{\"", b"\":0}")] {
            let input = [prefix, bytes, suffix].concat();
            let (_, errors, spans) = parse(&input, Mode::Single, &[]);
            if let Some(at) = invalid_at {
                assert_eq!(
                    errors,
                    vec![ParseError {
                        kind: ErrorKind::InvalidUtf8,
                        candidate_start: Some(0),
                        offset: (prefix.len() + at) as u64,
                    }],
                    "{input:?}"
                );
                assert!(spans.is_empty());
            } else {
                assert!(errors.is_empty(), "{input:?}");
                assert_eq!(spans, vec![0..input.len() as u64]);
            }
            assert_chunk_invariance(&input, Mode::Single);
        }
    }
}

#[test]
fn incomplete_utf8_at_eof_reports_eof_and_recovery_clears_state() {
    for bytes in [b"\xc2".as_slice(), b"\xe2", b"\xe2\x82", b"\xf0\x90\x80"] {
        let input = [b"[\"".as_slice(), bytes].concat();
        let (_, errors, spans) = parse(&input, Mode::Single, &[]);
        assert_eq!(
            errors,
            vec![ParseError {
                kind: ErrorKind::InvalidUtf8,
                candidate_start: Some(0),
                offset: input.len() as u64,
            }]
        );
        assert!(spans.is_empty());
        assert_chunk_invariance(&input, Mode::Single);
    }
    // A conflicting opener can immediately start a new candidate.
    let input = b"[\"\xe2{\"ok\":\"\xc2\xa2\"}";
    let (_, errors, spans) = parse(input, Mode::Scan, &[]);
    assert_eq!(
        errors,
        vec![ParseError {
            kind: ErrorKind::InvalidUtf8,
            candidate_start: Some(0),
            offset: 3,
        }]
    );
    assert_eq!(spans, vec![3..input.len() as u64]);
    assert_chunk_invariance(input, Mode::Scan);
}

// Expand fragments to byte records so callback fragment sizes are not a contract.
fn record(events: &mut Vec<String>, event: Event<'_>) {
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
}

fn parse(
    input: &[u8],
    mode: Mode,
    cuts: &[usize],
) -> (Vec<String>, Vec<ParseError>, Vec<Range<u64>>) {
    parse_with_depth(input, mode, cuts, DEFAULT_MAX_DEPTH)
}

fn parse_with_depth(
    input: &[u8],
    mode: Mode,
    cuts: &[usize],
    max_depth: std::num::NonZeroUsize,
) -> (Vec<String>, Vec<ParseError>, Vec<Range<u64>>) {
    parse_with_limits(input, mode, cuts, max_depth, None)
}

fn parse_with_limits(
    input: &[u8],
    mode: Mode,
    cuts: &[usize],
    max_depth: std::num::NonZeroUsize,
    maximum: Option<std::num::NonZeroU64>,
) -> (Vec<String>, Vec<ParseError>, Vec<Range<u64>>) {
    let mut parser = Parser::with_limits(mode, max_depth, maximum);
    let mut events = Vec::new();
    let mut errors = Vec::new();
    let mut spans = Vec::new();
    let mut emit = |event| {
        match &event {
            Event::Invalid(error) => errors.push(error.clone()),
            Event::CandidateEnd { span, .. } => spans.push(span.clone()),
            _ => {}
        }
        record(&mut events, event);
    };
    let mut start = 0;
    for &end in cuts {
        parser.push(&input[start..end], &mut emit);
        start = end;
    }
    parser.push(&input[start..], &mut emit);
    parser.finish(|event| {
        match &event {
            Event::Invalid(error) => errors.push(error.clone()),
            Event::CandidateEnd { span, .. } => spans.push(span.clone()),
            _ => {}
        }
        record(&mut events, event);
    });
    let consumed = if mode == Mode::Single {
        errors.first().map_or(input.len() as u64, |error| {
            (error.offset + 1).min(input.len() as u64)
        })
    } else {
        input.len() as u64
    };
    assert_eq!(parser.offset(), consumed);
    (events, errors, spans)
}

fn assert_chunk_invariance(input: &[u8], mode: Mode) {
    assert_depth_chunk_invariance(input, mode, DEFAULT_MAX_DEPTH);
}

fn assert_depth_chunk_invariance(input: &[u8], mode: Mode, max_depth: std::num::NonZeroUsize) {
    let parse = |input: &[u8], mode, cuts: &[usize]| parse_with_depth(input, mode, cuts, max_depth);
    let expected = parse(input, mode, &[]);
    for split in 0..=input.len() {
        assert_eq!(
            parse(input, mode, &[split]),
            expected,
            "input {input:?}, split {split}"
        );
    }
    let singles: Vec<_> = (0..=input.len()).collect();
    assert_eq!(parse(input, mode, &singles), expected);
    for seed in 1_u64..=8 {
        let mut state = seed;
        let mut cuts = vec![0];
        while *cuts.last().unwrap() < input.len() {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            cuts.push((cuts.last().unwrap() + 1 + (state >> 32) as usize % 7).min(input.len()));
        }
        assert_eq!(
            parse(input, mode, &cuts),
            expected,
            "seed {seed}, input {input:?}"
        );
    }
}

#[test]
fn valid_grammar_corpus_is_chunk_independent() {
    let cases: &[&[u8]] = &[
        b"{}",
        b"[]",
        b" \r\n\t{ } \n",
        b"[{},[],[null],true,false]",
        br#"{"a":0,"a":-0,"":123456789012345678901234567890}"#,
        br#"[0,-0,12,-12,0.5,-0.50,1e2,1E-2,1e+2,1.200e03]"#,
        br#"["","\"\\\/\b\f\n\r\t","\u0041\uD834\uDD1E"]"#,
        "{\"café\":\"雪😀\"}".as_bytes(),
    ];
    for &input in cases {
        let (_, errors, spans) = parse(input, Mode::Single, &[]);
        assert!(errors.is_empty(), "{input:?}: {errors:?}");
        let start = input.iter().position(|b| matches!(b, b'{' | b'[')).unwrap();
        let end = input
            .iter()
            .rposition(|b| matches!(b, b'}' | b']'))
            .unwrap()
            + 1;
        assert_eq!(spans, vec![start as u64..end as u64]);
        assert_chunk_invariance(input, Mode::Single);
    }
}

#[test]
fn invalid_grammar_corpus_reports_exact_errors_across_chunks() {
    use ErrorKind::*;
    let cases: &[(&[u8], ErrorKind, u64)] = &[
        (b"[01]", InvalidNumber, 2),
        (b"[-]", InvalidNumber, 2),
        (b"[1.]", InvalidNumber, 3),
        (b"[1e]", InvalidNumber, 3),
        (b"[1e+]", InvalidNumber, 4),
        (b"[truX]", UnexpectedByte, 4),
        (br#"["\q"]"#, InvalidEscape, 3),
        (br#"["\u12x4"]"#, InvalidUnicodeEscape, 6),
        (b"[\"a\nb\"]", ControlCharacterInString, 3),
        (b"[1,]", UnexpectedByte, 3),
        (b"{\"a\":1,}", UnexpectedByte, 7),
        (b"{\"a\" 1}", UnexpectedByte, 5),
        (b"[true false]", UnexpectedByte, 6),
        (b"[}", UnexpectedByte, 1),
        (b"{]", UnexpectedByte, 1),
        (b"[", UnexpectedEnd, 1),
        (b"[1", UnexpectedEnd, 2),
        (b"[1e", UnexpectedEnd, 3),
        (b"[\"abc", UnexpectedEnd, 5),
        (b"[\"\\", UnexpectedEnd, 3),
        (br#"["\u12"#, UnexpectedEnd, 6),
    ];
    for &(input, kind, offset) in cases {
        let (_, errors, spans) = parse(input, Mode::Single, &[]);
        assert_eq!(
            errors,
            vec![ParseError {
                kind,
                candidate_start: Some(0),
                offset
            }],
            "{input:?}"
        );
        assert!(spans.is_empty(), "{input:?}");
        assert_chunk_invariance(input, Mode::Single);
    }
}

#[test]
fn recovery_preserves_candidate_boundaries_and_absolute_offsets() {
    let input = b"\xff{} [1,] [true] {\"broken\" {\"ok\":null}";
    let (_, errors, spans) = parse(input, Mode::Scan, &[]);
    assert_eq!(spans, vec![1..3, 9..15, 26..37]);
    assert_eq!(
        errors,
        vec![
            ParseError {
                kind: ErrorKind::UnexpectedByte,
                candidate_start: Some(4),
                offset: 7
            },
            ParseError {
                kind: ErrorKind::UnexpectedByte,
                candidate_start: Some(16),
                offset: 26
            },
        ]
    );
    assert_chunk_invariance(input, Mode::Scan);
}

#[test]
fn single_mode_rejects_missing_roots_and_trailing_data() {
    for (input, kind, offset) in [
        (b"".as_slice(), ErrorKind::ExpectedSingleValue, 0),
        (b" \t".as_slice(), ErrorKind::ExpectedSingleValue, 2),
        (b"true".as_slice(), ErrorKind::ExpectedSingleValue, 0),
        (b"{} []".as_slice(), ErrorKind::TrailingData, 3),
    ] {
        let (_, errors, _) = parse(input, Mode::Single, &[]);
        assert_eq!(
            errors,
            vec![ParseError {
                kind,
                candidate_start: None,
                offset
            }]
        );
        assert_chunk_invariance(input, Mode::Single);
    }
}

#[test]
fn candidate_byte_limits_are_exact_and_chunk_independent() {
    let cases: &[&[u8]] = &[
        b"{}",
        b"[ ]",
        br#"[1.25e+3]"#,
        br#"{"key":"a\u1234\n"}"#,
        "{\"雪\":\"😀\"}".as_bytes(),
        b"[true,false,null]",
        b"[[]]",
        b"[1,]",
        b"[\"\xff\"]",
        br#"["\uXX"]"#,
    ];
    for &input in cases {
        for end in 0..=input.len() {
            let input = &input[..end]; // Every truncated prefix, including UTF-8 and escapes.
            for limit in 1..=input.len() as u64 + 1 {
                for mode in [Mode::Scan, Mode::Single] {
                    let maximum = std::num::NonZeroU64::new(limit);
                    let parse = |cuts: &[usize]| {
                        parse_with_limits(input, mode, cuts, DEFAULT_MAX_DEPTH, maximum)
                    };
                    let expected = parse(&[]);
                    for split in 0..=input.len() {
                        assert_eq!(parse(&[split]), expected);
                    }
                    assert_eq!(parse(&(0..=input.len()).collect::<Vec<_>>()), expected);
                    if let Some(error) = expected.1.first() {
                        assert!(error.offset <= limit);
                        if error.kind == ErrorKind::CandidateLimitExceeded {
                            assert_eq!(error.offset, limit);
                            assert_eq!(error.candidate_start, Some(0));
                            assert!(expected.2.is_empty());
                        }
                    }
                }
            }
        }
        let (_, errors, spans) = parse_with_limits(
            input,
            Mode::Single,
            &[],
            DEFAULT_MAX_DEPTH,
            std::num::NonZeroU64::new(input.len() as u64),
        );
        if ![b"[1,]".as_slice(), b"[\"\xff\"]", br#"["\uXX"]"#].contains(&input) {
            assert!(errors.is_empty());
            assert_eq!(spans, vec![0..input.len() as u64]);
            let (_, errors, spans) = parse_with_limits(
                input,
                Mode::Single,
                &[],
                DEFAULT_MAX_DEPTH,
                std::num::NonZeroU64::new(input.len() as u64 - 1),
            );
            assert_eq!(
                errors,
                vec![ParseError {
                    kind: ErrorKind::CandidateLimitExceeded,
                    candidate_start: Some(0),
                    offset: input.len() as u64 - 1,
                }]
            );
            assert!(spans.is_empty());
        }
    }
}

#[test]
fn candidate_limit_recovery_consumes_offender_resets_and_freezes_single_mode() {
    let input = b"junk [  {] {} []";
    for mode in [Mode::Scan, Mode::Single] {
        let input = if mode == Mode::Scan {
            input.as_slice()
        } else {
            &input[5..]
        };
        for split in 0..=input.len() {
            let mut parser =
                Parser::with_limits(mode, DEFAULT_MAX_DEPTH, std::num::NonZeroU64::new(3));
            let mut errors = Vec::new();
            let mut spans = Vec::new();
            let mut starts = Vec::new();
            for chunk in [&input[..split], &input[split..]] {
                parser.push(chunk, |event| match event {
                    Event::Invalid(error) => errors.push(error),
                    Event::CandidateStart { offset, .. } => starts.push(offset),
                    Event::CandidateEnd { span, .. } => spans.push(span),
                    _ => {}
                });
            }
            let start = if mode == Mode::Scan { 5 } else { 0 };
            assert_eq!(
                errors,
                vec![ParseError {
                    kind: ErrorKind::CandidateLimitExceeded,
                    candidate_start: Some(start),
                    offset: start + 3
                }]
            );
            if mode == Mode::Scan {
                assert_eq!(starts, vec![5, 11, 14]); // Offending '{' at 8 is consumed.
                assert_eq!(spans, vec![11..13, 14..16]);
            } else {
                assert_eq!(starts, vec![0]);
                assert!(spans.is_empty());
                assert_eq!(parser.offset(), 4);
            }
            parser.finish(|_| panic!("unexpected EOF event"));
            let offset = parser.offset();
            parser.finish(|_| panic!("duplicate EOF event"));
            parser.push(b"[]", |_| panic!("resumed closed parser"));
            assert_eq!(parser.offset(), offset);
        }
    }
    // At the exact limit, unfinished tokens keep their EOF diagnosis.
    for (input, kind) in [
        (b"[1".as_slice(), ErrorKind::UnexpectedEnd),
        (b"[\"\xe2", ErrorKind::InvalidUtf8),
    ] {
        let (_, errors, _) = parse_with_limits(
            input,
            Mode::Single,
            &[],
            DEFAULT_MAX_DEPTH,
            std::num::NonZeroU64::new(input.len() as u64),
        );
        assert_eq!(errors[0].kind, kind);
    }
    // Size checking wins over depth and syntax checking on the rejected byte.
    let (_, errors, _) = parse_with_limits(
        b"[[]]",
        Mode::Single,
        &[],
        std::num::NonZeroUsize::new(1).unwrap(),
        std::num::NonZeroU64::new(1),
    );
    assert_eq!(errors[0].kind, ErrorKind::CandidateLimitExceeded);
}
