# 1.0 API stability review

The supported core is `Parser`, `Mode`, `Event`, `ParseError`, `ErrorKind`,
`ContainerKind`, `StringKind` and `DEFAULT_MAX_DEPTH`. The core has no runtime
dependencies and uses no unsafe code. This review covers repository-owned code,
not an audit of the Rust standard library or development dependencies.

## Contracts to preserve

- Default scan mode discovers objects/arrays; single mode requires exactly one
  container with JSON whitespace around it. Root scalars remain unsupported.
- Root depth is one; default maximum depth is 1,000. Candidate bytes are unlimited
  unless explicitly bounded. Limits use raw input bytes, not decoded text.
- Borrowed raw string and number fragments preserve input spelling. Fragment
  size/count is unspecified; fragments can split UTF-8 and escapes. A caller
  needing bytes beyond the input lifetime must copy them. Escaped surrogate
  code units are syntax-checked and preserved, not decoded or pair-validated.
- Events preserve input order. Effects before CandidateEnd are speculative.
  Invalid closes that candidate; scan recovery discards forward and may reuse a
  conflicting opener except for consumed depth/candidate-limit offenders.
- Offsets count consumed bytes and spans are half-open. EOF emits final events
  once and permanently closes the parser. Terminal syntax errors consume their
  offender. OffsetOverflow is the exception: no representable next consumed
  offset exists, so its byte is unconsumed and both modes become terminal.
- `finish` takes nonborrowing events (`Event<'static>`); an explicitly typed
  closure `|event: Event<'_>| ...` can be reused for pushes and EOF. Callback
  panic recovery is not guaranteed. Allocation failure follows Rust allocator
  behavior; no fallible-allocation API is promised.

## Extension review

Decoding, paths, Read/async, owned values and Serde adapters can consume these
public events without adding storage to Parser or changing its lifetimes.
Their separate types/configurations must define their own limits. They do not
require core event changes and remain outside 1.0 scope.

Public enums are exhaustive and configuration is constructor-based. Adding
variants, changing fields/types/lifetimes, defaults, errors, recovery, EOF or
root semantics requires a breaking-change review. New adapters/functions may
be additive. Callback count and exact fragment sizes are deliberately excluded
from compatibility guarantees.

Release validation, reference comparisons, fuzzing, benchmarks and the tested
minimum compiler are tracked separately in ROADMAP.md; this review does not
claim those checks have run.
