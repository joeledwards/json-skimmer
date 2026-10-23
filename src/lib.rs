//! Incremental, recoverable parsing of JSON objects and arrays.
//!
//! [`Parser::push`] accepts arbitrarily divided byte chunks. Events borrowing
//! token fragments are delivered to a callback before `push` returns, allowing
//! the parser to remain independent of the size of the input document.

use std::num::NonZeroUsize;
use std::ops::Range;

/// Maximum simultaneous open containers by default; the root has depth one.
pub const DEFAULT_MAX_DEPTH: NonZeroUsize = NonZeroUsize::new(1000).unwrap();

/// How the parser treats bytes outside a JSON candidate.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Mode {
    /// Ignore non-JSON text and continue discovering objects and arrays.
    #[default]
    Scan,
    /// Require exactly one object or array, surrounded only by JSON whitespace.
    Single,
}

/// The kind of a JSON container.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContainerKind {
    Object,
    Array,
}

/// Whether a string token is an object key or a value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StringKind {
    Key,
    Value,
}

/// A parser event. Fragment bytes are the original JSON representation between
/// the string quotes, including any escape sequences.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event<'a> {
    CandidateStart {
        kind: ContainerKind,
        offset: u64,
    },
    ObjectStart {
        offset: u64,
    },
    ObjectEnd {
        offset: u64,
    },
    ArrayStart {
        offset: u64,
    },
    ArrayEnd {
        offset: u64,
    },
    StringStart {
        kind: StringKind,
        offset: u64,
    },
    StringFragment {
        kind: StringKind,
        bytes: &'a [u8],
        offset: u64,
    },
    StringEnd {
        kind: StringKind,
        offset: u64,
    },
    NumberStart {
        offset: u64,
    },
    NumberFragment {
        bytes: &'a [u8],
        offset: u64,
    },
    NumberEnd {
        offset: u64,
    },
    Boolean {
        value: bool,
        span: Range<u64>,
    },
    Null {
        span: Range<u64>,
    },
    CandidateEnd {
        kind: ContainerKind,
        span: Range<u64>,
    },
    Invalid(ParseError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    pub kind: ErrorKind,
    pub candidate_start: Option<u64>,
    pub offset: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    UnexpectedByte,
    InvalidEscape,
    InvalidUnicodeEscape,
    /// Malformed UTF-8 at the offending byte, or an incomplete sequence at EOF.
    InvalidUtf8,
    /// Opening another container would exceed the configured maximum depth.
    DepthLimitExceeded,
    ControlCharacterInString,
    InvalidNumber,
    UnexpectedEnd,
    ExpectedSingleValue,
    TrailingData,
}

#[derive(Clone, Copy, Debug)]
enum Frame {
    Object(ObjectState),
    Array(ArrayState),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ObjectState {
    KeyOrEnd,
    Key,
    Colon,
    Value,
    CommaOrEnd,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArrayState {
    ValueOrEnd,
    Value,
    CommaOrEnd,
}

#[derive(Clone, Copy, Debug)]
enum Token {
    String {
        kind: StringKind,
        escape: bool,
        unicode_left: u8,
    },
    Number(NumberState),
    Literal {
        kind: LiteralKind,
        matched: u8,
        start: u64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NumberState {
    Minus,
    Zero,
    Integer,
    Dot,
    Fraction,
    Exponent,
    ExponentSign,
    ExponentDigits,
}

#[derive(Clone, Copy, Debug)]
enum LiteralKind {
    True,
    False,
    Null,
}

/// Only continuation bounds and a count are retained, never string contents.
#[derive(Clone, Copy, Debug, Default)]
struct Utf8State {
    remaining: u8,
    minimum: u8,
    maximum: u8,
}

impl Utf8State {
    fn advance(self, byte: u8) -> Option<Self> {
        if self.remaining > 0 {
            return (self.minimum..=self.maximum)
                .contains(&byte)
                .then_some(Self {
                    remaining: self.remaining - 1,
                    minimum: 0x80,
                    maximum: 0xbf,
                });
        }
        let (remaining, minimum, maximum) = match byte {
            0x00..=0x7f => (0, 0x80, 0xbf),
            0xc2..=0xdf => (1, 0x80, 0xbf),
            0xe0 => (2, 0xa0, 0xbf),
            0xe1..=0xec | 0xee..=0xef => (2, 0x80, 0xbf),
            0xed => (2, 0x80, 0x9f),
            0xf0 => (3, 0x90, 0xbf),
            0xf1..=0xf3 => (3, 0x80, 0xbf),
            0xf4 => (3, 0x80, 0x8f),
            _ => return None,
        };
        Some(Self {
            remaining,
            minimum,
            maximum,
        })
    }
}

impl LiteralKind {
    fn bytes(self) -> &'static [u8] {
        match self {
            Self::True => b"true",
            Self::False => b"false",
            Self::Null => b"null",
        }
    }
}

/// Stateful incremental parser.
#[derive(Debug)]
pub struct Parser {
    mode: Mode,
    max_depth: NonZeroUsize,
    offset: u64,
    candidate_start: Option<u64>,
    root_kind: Option<ContainerKind>,
    stack: Vec<Frame>,
    token: Option<Token>,
    utf8: Utf8State,
    completed_single: bool,
    terminal: bool,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new(Mode::Scan)
    }
}

impl Parser {
    pub fn new(mode: Mode) -> Self {
        Self::with_max_depth(mode, DEFAULT_MAX_DEPTH)
    }

    /// Sets a positive container-depth limit. The root counts as depth one.
    /// A rejected opener is consumed; scan mode resumes at the following byte.
    pub fn with_max_depth(mode: Mode, max_depth: NonZeroUsize) -> Self {
        Self {
            mode,
            max_depth,
            offset: 0,
            candidate_start: None,
            root_kind: None,
            stack: Vec::new(),
            token: None,
            utf8: Utf8State::default(),
            completed_single: false,
            terminal: false,
        }
    }

    /// Number of consumed bytes. A terminal error consumes its offending byte
    /// but leaves the remainder of that chunk unconsumed. Closed parsers do
    /// not advance this offset.
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// Processes input until exhausted or a terminal error occurs.
    /// After [`Self::finish`] or a terminal error, this is a no-op: no events
    /// are emitted and [`Self::offset`] remains unchanged.
    pub fn push<'a>(&mut self, input: &'a [u8], mut emit: impl FnMut(Event<'a>)) {
        if self.terminal {
            return;
        }
        for (index, &byte) in input.iter().enumerate() {
            let absolute = self.offset;
            let fragment = &input[index..index + 1];
            let was_active = self.candidate_start.is_some();
            let mut depth_exceeded = false;
            self.process(byte, fragment, absolute, &mut |event| {
                depth_exceeded |= matches!(&event, Event::Invalid(error) if error.kind == ErrorKind::DepthLimitExceeded);
                emit(event);
            });
            // Discard recovery does not revisit earlier input, but the byte that
            // exposed the conflict can itself open the next candidate.
            if was_active
                && self.candidate_start.is_none()
                && !self.terminal
                && !depth_exceeded
                && matches!(byte, b'{' | b'[')
            {
                self.scan(byte, absolute, &mut emit);
            }
            self.offset += 1;
            if self.terminal {
                break;
            }
        }
    }

    /// Signals EOF and permanently closes this parser after emitting any final
    /// token or error events. Repeated calls emit nothing. Later pushes are
    /// ignored; construct a new parser to process another stream.
    pub fn finish(&mut self, mut emit: impl FnMut(Event<'static>)) {
        if self.terminal {
            return;
        }
        if let Some(Token::Number(state)) = self.token
            && number_can_end(state)
        {
            emit(Event::NumberEnd {
                offset: self.offset,
            });
            self.token = None;
            self.value_complete();
        }
        if self.candidate_start.is_some() {
            let kind = if self.utf8.remaining > 0 {
                ErrorKind::InvalidUtf8
            } else {
                ErrorKind::UnexpectedEnd
            };
            self.invalidate(kind, self.offset, &mut emit);
        } else if self.mode == Mode::Single && !self.completed_single {
            self.fail_single(ErrorKind::ExpectedSingleValue, self.offset, &mut emit);
        }
        self.terminal = true;
    }

    fn process<'a>(
        &mut self,
        byte: u8,
        fragment: &'a [u8],
        at: u64,
        emit: &mut impl FnMut(Event<'a>),
    ) {
        if self.terminal {
            return;
        }
        if self.candidate_start.is_none() {
            self.scan(byte, at, emit);
            return;
        }

        if let Some(token) = self.token {
            match token {
                Token::String {
                    kind,
                    escape,
                    unicode_left,
                } => {
                    if self.utf8.remaining > 0 {
                        if let Some(next) = self.utf8.advance(byte) {
                            self.utf8 = next;
                            emit(Event::StringFragment {
                                kind,
                                bytes: fragment,
                                offset: at,
                            });
                        } else {
                            self.invalidate(ErrorKind::InvalidUtf8, at, emit);
                        }
                    } else if unicode_left > 0 {
                        if !byte.is_ascii_hexdigit() {
                            self.invalidate(ErrorKind::InvalidUnicodeEscape, at, emit);
                        } else {
                            emit(Event::StringFragment {
                                kind,
                                bytes: fragment,
                                offset: at,
                            });
                            self.token = Some(Token::String {
                                kind,
                                escape: false,
                                unicode_left: unicode_left - 1,
                            });
                        }
                    } else if escape {
                        if byte == b'u' || b"\"\\/bfnrt".contains(&byte) {
                            emit(Event::StringFragment {
                                kind,
                                bytes: fragment,
                                offset: at,
                            });
                            self.token = Some(Token::String {
                                kind,
                                escape: false,
                                unicode_left: if byte == b'u' { 4 } else { 0 },
                            });
                        } else {
                            self.invalidate(ErrorKind::InvalidEscape, at, emit);
                        }
                    } else if byte == b'"' {
                        emit(Event::StringEnd { kind, offset: at });
                        self.token = None;
                        if kind == StringKind::Key {
                            self.set_object(ObjectState::Colon);
                        } else {
                            self.value_complete();
                        }
                    } else if byte == b'\\' {
                        emit(Event::StringFragment {
                            kind,
                            bytes: fragment,
                            offset: at,
                        });
                        self.token = Some(Token::String {
                            kind,
                            escape: true,
                            unicode_left: 0,
                        });
                    } else if byte < 0x20 {
                        self.invalidate(ErrorKind::ControlCharacterInString, at, emit);
                    } else if let Some(next) = self.utf8.advance(byte) {
                        self.utf8 = next;
                        emit(Event::StringFragment {
                            kind,
                            bytes: fragment,
                            offset: at,
                        });
                    } else {
                        self.invalidate(ErrorKind::InvalidUtf8, at, emit);
                    }
                }
                Token::Number(state) => {
                    if let Some(next) = number_step(state, byte) {
                        emit(Event::NumberFragment {
                            bytes: fragment,
                            offset: at,
                        });
                        self.token = Some(Token::Number(next));
                    } else if number_can_end(state) && is_value_delimiter(byte) {
                        emit(Event::NumberEnd { offset: at });
                        self.token = None;
                        self.value_complete();
                        self.process_structure(byte, fragment, at, emit);
                    } else {
                        self.invalidate(ErrorKind::InvalidNumber, at, emit);
                    }
                }
                Token::Literal {
                    kind,
                    matched,
                    start,
                } => {
                    let expected = kind.bytes();
                    if expected.get(matched as usize) == Some(&byte) {
                        let next = matched + 1;
                        if next as usize == expected.len() {
                            match kind {
                                LiteralKind::True => emit(Event::Boolean {
                                    value: true,
                                    span: start..at + 1,
                                }),
                                LiteralKind::False => emit(Event::Boolean {
                                    value: false,
                                    span: start..at + 1,
                                }),
                                LiteralKind::Null => emit(Event::Null {
                                    span: start..at + 1,
                                }),
                            }
                            self.token = None;
                            self.value_complete();
                        } else {
                            self.token = Some(Token::Literal {
                                kind,
                                matched: next,
                                start,
                            });
                        }
                    } else {
                        self.invalidate(ErrorKind::UnexpectedByte, at, emit);
                    }
                }
            }
        } else {
            self.process_structure(byte, fragment, at, emit);
        }
    }

    fn scan<'a>(&mut self, byte: u8, at: u64, emit: &mut impl FnMut(Event<'a>)) {
        if self.mode == Mode::Single && self.completed_single {
            if !is_ws(byte) {
                self.fail_single(ErrorKind::TrailingData, at, emit);
            }
            return;
        }
        let kind = match byte {
            b'{' => Some(ContainerKind::Object),
            b'[' => Some(ContainerKind::Array),
            _ => None,
        };
        if let Some(kind) = kind {
            self.candidate_start = Some(at);
            self.root_kind = Some(kind);
            emit(Event::CandidateStart { kind, offset: at });
            self.open(kind, at, emit);
        } else if self.mode == Mode::Single && !is_ws(byte) {
            self.fail_single(ErrorKind::ExpectedSingleValue, at, emit);
        }
    }

    fn process_structure<'a>(
        &mut self,
        byte: u8,
        fragment: &'a [u8],
        at: u64,
        emit: &mut impl FnMut(Event<'a>),
    ) {
        if is_ws(byte) {
            return;
        }
        let Some(frame) = self.stack.last().copied() else {
            return;
        };
        match frame {
            Frame::Object(ObjectState::KeyOrEnd) if byte == b'}' => {
                self.close(ContainerKind::Object, at, emit)
            }
            Frame::Object(ObjectState::KeyOrEnd) if byte == b'"' => {
                self.set_object(ObjectState::Key);
                emit(Event::StringStart {
                    kind: StringKind::Key,
                    offset: at,
                });
                self.token = Some(Token::String {
                    kind: StringKind::Key,
                    escape: false,
                    unicode_left: 0,
                });
            }
            Frame::Object(ObjectState::Key) if byte == b'"' => {
                emit(Event::StringStart {
                    kind: StringKind::Key,
                    offset: at,
                });
                self.token = Some(Token::String {
                    kind: StringKind::Key,
                    escape: false,
                    unicode_left: 0,
                });
            }
            Frame::Object(ObjectState::Colon) if byte == b':' => {
                self.set_object(ObjectState::Value)
            }
            Frame::Object(ObjectState::Value) => self.start_value(byte, fragment, at, emit),
            Frame::Object(ObjectState::CommaOrEnd) if byte == b',' => {
                self.set_object(ObjectState::Key)
            }
            Frame::Object(ObjectState::CommaOrEnd) if byte == b'}' => {
                self.close(ContainerKind::Object, at, emit)
            }
            Frame::Array(ArrayState::ValueOrEnd) if byte == b']' => {
                self.close(ContainerKind::Array, at, emit)
            }
            Frame::Array(ArrayState::ValueOrEnd | ArrayState::Value) => {
                self.start_value(byte, fragment, at, emit)
            }
            Frame::Array(ArrayState::CommaOrEnd) if byte == b',' => {
                self.set_array(ArrayState::Value)
            }
            Frame::Array(ArrayState::CommaOrEnd) if byte == b']' => {
                self.close(ContainerKind::Array, at, emit)
            }
            _ => self.invalidate(ErrorKind::UnexpectedByte, at, emit),
        }
    }

    fn start_value<'a>(
        &mut self,
        byte: u8,
        fragment: &'a [u8],
        at: u64,
        emit: &mut impl FnMut(Event<'a>),
    ) {
        match byte {
            b'{' => self.open(ContainerKind::Object, at, emit),
            b'[' => self.open(ContainerKind::Array, at, emit),
            b'"' => {
                emit(Event::StringStart {
                    kind: StringKind::Value,
                    offset: at,
                });
                self.token = Some(Token::String {
                    kind: StringKind::Value,
                    escape: false,
                    unicode_left: 0,
                });
            }
            b't' => {
                self.token = Some(Token::Literal {
                    kind: LiteralKind::True,
                    matched: 1,
                    start: at,
                })
            }
            b'f' => {
                self.token = Some(Token::Literal {
                    kind: LiteralKind::False,
                    matched: 1,
                    start: at,
                })
            }
            b'n' => {
                self.token = Some(Token::Literal {
                    kind: LiteralKind::Null,
                    matched: 1,
                    start: at,
                })
            }
            b'-' => {
                emit(Event::NumberStart { offset: at });
                emit(Event::NumberFragment {
                    bytes: fragment,
                    offset: at,
                });
                self.token = Some(Token::Number(NumberState::Minus));
            }
            b'0' => {
                emit(Event::NumberStart { offset: at });
                emit(Event::NumberFragment {
                    bytes: fragment,
                    offset: at,
                });
                self.token = Some(Token::Number(NumberState::Zero));
            }
            b'1'..=b'9' => {
                emit(Event::NumberStart { offset: at });
                emit(Event::NumberFragment {
                    bytes: fragment,
                    offset: at,
                });
                self.token = Some(Token::Number(NumberState::Integer));
            }
            _ => self.invalidate(ErrorKind::UnexpectedByte, at, emit),
        }
    }

    fn open<'a>(&mut self, kind: ContainerKind, at: u64, emit: &mut impl FnMut(Event<'a>)) {
        if self.stack.len() >= self.max_depth.get() {
            self.invalidate(ErrorKind::DepthLimitExceeded, at, emit);
            return;
        }
        match kind {
            ContainerKind::Object => {
                self.stack.push(Frame::Object(ObjectState::KeyOrEnd));
                emit(Event::ObjectStart { offset: at });
            }
            ContainerKind::Array => {
                self.stack.push(Frame::Array(ArrayState::ValueOrEnd));
                emit(Event::ArrayStart { offset: at });
            }
        }
    }

    fn close<'a>(&mut self, kind: ContainerKind, at: u64, emit: &mut impl FnMut(Event<'a>)) {
        self.stack.pop();
        match kind {
            ContainerKind::Object => emit(Event::ObjectEnd { offset: at }),
            ContainerKind::Array => emit(Event::ArrayEnd { offset: at }),
        }
        if self.stack.is_empty() {
            let start = self
                .candidate_start
                .take()
                .expect("active candidate has a start");
            let root = self
                .root_kind
                .take()
                .expect("active candidate has a root kind");
            emit(Event::CandidateEnd {
                kind: root,
                span: start..at + 1,
            });
            if self.mode == Mode::Single {
                self.completed_single = true;
            }
        } else {
            self.value_complete();
        }
    }

    fn value_complete(&mut self) {
        match self.stack.last_mut() {
            Some(Frame::Object(state)) => *state = ObjectState::CommaOrEnd,
            Some(Frame::Array(state)) => *state = ArrayState::CommaOrEnd,
            None => {}
        }
    }

    fn set_object(&mut self, next: ObjectState) {
        if let Some(Frame::Object(state)) = self.stack.last_mut() {
            *state = next;
        }
    }
    fn set_array(&mut self, next: ArrayState) {
        if let Some(Frame::Array(state)) = self.stack.last_mut() {
            *state = next;
        }
    }

    fn invalidate<'a>(&mut self, kind: ErrorKind, at: u64, emit: &mut impl FnMut(Event<'a>)) {
        let start = self.candidate_start;
        emit(Event::Invalid(ParseError {
            kind,
            candidate_start: start,
            offset: at,
        }));
        self.stack.clear();
        self.token = None;
        self.candidate_start = None;
        self.utf8 = Utf8State::default();
        self.root_kind = None;
        if self.mode == Mode::Single {
            self.terminal = true;
        }
    }

    fn fail_single<'a>(&mut self, kind: ErrorKind, at: u64, emit: &mut impl FnMut(Event<'a>)) {
        emit(Event::Invalid(ParseError {
            kind,
            candidate_start: self.candidate_start,
            offset: at,
        }));
        self.terminal = true;
    }
}

fn is_ws(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r')
}
fn is_value_delimiter(byte: u8) -> bool {
    is_ws(byte) || matches!(byte, b',' | b']' | b'}')
}
fn number_can_end(state: NumberState) -> bool {
    matches!(
        state,
        NumberState::Zero
            | NumberState::Integer
            | NumberState::Fraction
            | NumberState::ExponentDigits
    )
}

fn number_step(state: NumberState, byte: u8) -> Option<NumberState> {
    use NumberState::*;
    match (state, byte) {
        (Minus, b'0') => Some(Zero),
        (Minus, b'1'..=b'9') => Some(Integer),
        (Zero | Integer, b'.') => Some(Dot),
        (Integer, b'0'..=b'9') => Some(Integer),
        (Zero | Integer, b'e' | b'E') => Some(Exponent),
        (Dot, b'0'..=b'9') | (Fraction, b'0'..=b'9') => Some(Fraction),
        (Fraction, b'e' | b'E') => Some(Exponent),
        (Exponent, b'+' | b'-') => Some(ExponentSign),
        (Exponent | ExponentSign, b'0'..=b'9') | (ExponentDigits, b'0'..=b'9') => {
            Some(ExponentDigits)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_across_every_chunk_boundary() {
        let input = br#"{"a":"x\u0041","n":-12.5e+2,"v":[true,false,null]}"#;
        for split in 0..=input.len() {
            let mut parser = Parser::default();
            let mut completed = 0;
            let mut invalid = 0;
            parser.push(&input[..split], |event| match event {
                Event::CandidateEnd { .. } => completed += 1,
                Event::Invalid(_) => invalid += 1,
                _ => {}
            });
            parser.push(&input[split..], |event| match event {
                Event::CandidateEnd { .. } => completed += 1,
                Event::Invalid(_) => invalid += 1,
                _ => {}
            });
            parser.finish(|event| {
                if matches!(event, Event::Invalid(_)) {
                    invalid += 1
                }
            });
            assert_eq!((completed, invalid), (1, 0), "split at {split}");
        }
    }

    #[test]
    fn reports_invalid_candidate_and_finds_the_next_one() {
        let mut parser = Parser::default();
        let mut invalid_offsets = Vec::new();
        let mut spans = Vec::new();
        parser.push(b"noise {bad text [true, null] tail", |event| match event {
            Event::Invalid(error) => invalid_offsets.push(error.offset),
            Event::CandidateEnd { span, .. } => spans.push(span),
            _ => {}
        });
        assert_eq!(invalid_offsets, vec![7]);
        assert_eq!(spans, vec![16..28]);
    }

    #[test]
    fn conflicting_opener_starts_a_new_candidate() {
        let mut parser = Parser::default();
        let mut starts = Vec::new();
        let mut completed = Vec::new();
        parser.push(br#"{"broken" {"valid":true}"#, |event| match event {
            Event::CandidateStart { offset, .. } => starts.push(offset),
            Event::CandidateEnd { span, .. } => completed.push(span),
            _ => {}
        });
        assert_eq!(starts, vec![0, 10]);
        assert_eq!(completed, vec![10..24]);
    }

    #[test]
    fn rejects_trailing_commas() {
        for input in [b"{\"a\":1,}".as_slice(), b"[1,]".as_slice()] {
            let mut parser = Parser::default();
            let mut invalid = 0;
            parser.push(input, |event| {
                if matches!(event, Event::Invalid(_)) {
                    invalid += 1
                }
            });
            assert_eq!(invalid, 1);
        }
    }

    #[test]
    fn single_mode_rejects_trailing_data() {
        let mut parser = Parser::new(Mode::Single);
        let mut errors = Vec::new();
        parser.push(b"{} []", |event| {
            if let Event::Invalid(error) = event {
                errors.push(error.kind);
            }
        });
        assert_eq!(errors, vec![ErrorKind::TrailingData]);
    }

    #[test]
    fn finish_reports_an_incomplete_candidate() {
        let mut parser = Parser::default();
        parser.push(b"prefix {\"a\":", |_| {});
        let mut errors = Vec::new();
        parser.finish(|event| {
            if let Event::Invalid(error) = event {
                errors.push(error.kind);
            }
        });
        assert_eq!(errors, vec![ErrorKind::UnexpectedEnd]);
    }
}

#[cfg(test)]
mod contract_tests;
