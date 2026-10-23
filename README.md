# json-skimmer

`json-skimmer` is an incremental, event-driven JSON parser designed for
long-lived streams. It discovers top-level objects and arrays, reports malformed
candidates, and can continue scanning for later JSON instead of terminating.

The crate is in early development. Its core parser is dependency-free and does
not buffer complete documents.

The default maximum nesting depth is 1,000 containers, counting the root as
depth one. Use `Parser::with_max_depth(mode, NonZeroUsize)` to select a positive
custom limit. Exceeding it emits `DepthLimitExceeded` before the rejected
container opens.

## Parser lifecycle

Call `push()` for each input chunk, then call `finish()` once at EOF. `finish()`
emits any final token/error events and permanently closes the parser, whether
input was valid, empty, or incomplete. It is idempotent: repeated `finish()`
calls emit nothing. After finishing or encountering a terminal error, `push()`
ignores input, emits nothing, and leaves `offset()` unchanged. Create a new
parser for another stream.

`offset()` counts consumed bytes. A terminal error consumes the offending byte
but stops before the rest of that chunk. For example, single mode given
`{} []` reports trailing data at byte offset 3 and freezes `offset()` at 4,
regardless of chunk divisions. EOF errors report the EOF offset without
consuming another byte.

A `CandidateEnd` event alone does not close the parser: scan mode may discover
more candidates, and single mode still accepts trailing whitespace or reports
trailing data. Scan-mode recoverable errors also leave the parser open until
`finish()` is called.

## Project documents

- [Design](docs/design.md) describes the intended behavior, architecture, and
  memory model.
- [FLON requirements](docs/flon-requirements.md) defines the minimum `1.0.0`
  capabilities needed to rebuild FLON in Rust.
- [Roadmap](ROADMAP.md) distinguishes the current implementation from planned
  work.

- [Test coverage](docs/test-coverage.md) records tested contracts and remaining gaps.
