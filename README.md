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

Use `Parser::with_limits(mode, max_depth, Option<NonZeroU64>)` to also bound raw
input bytes per candidate. `None` means unlimited and is the default for existing
constructors. Counting includes the root delimiters, internal whitespace, raw
escapes, and UTF-8 bytes, excluding surrounding text. Exactly the configured
number of bytes is allowed. Before parsing the first excess byte, the parser
emits `CandidateLimitExceeded` at that byte's absolute offset, with the original
candidate start. That byte is consumed, including if it is an opener. Scan mode
resumes at the following byte; single mode becomes terminal. The bound resets
for every candidate. Size checks take precedence over syntax, UTF-8, and depth
checks on the excess byte. EOF consumes no byte, so an unfinished candidate
exactly at the limit retains its normal EOF error.

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

## Companion JSON generator

[json-volcano](https://crates.io/crates/json-volcano) is an independent sibling project
with a programmable generator and optional CLI for exact-size seeded fixtures.
It is not part of json-skimmer or its Cargo workspace. Its published 1.0 library is used as a development dependency for bounded
valid/invalid/mixed/truncated fixture tests, without adding runtime dependencies
to the parser. Existing independent tree-generated tests remain in place.

## Project documents

- [Design](docs/design.md) describes the intended behavior, architecture, and
  memory model.
- [FLON requirements](docs/flon-requirements.md) defines the minimum `1.0.0`
  capabilities needed to rebuild FLON in Rust.
- [Roadmap](ROADMAP.md) distinguishes the current implementation from planned
  work.

- [Test coverage](docs/test-coverage.md) records tested contracts and remaining gaps.

## Release plan

The target is a coordinated 1.0.0 release with FLON.
The parser publishes first, then FLON uses that published dependency. Core
features and release gates are tracked in [the release roadmap](ROADMAP.md);
convenience features are candidates for later 1.x releases, with breaking
changes reviewed for 2.0. No release dates are assigned.
