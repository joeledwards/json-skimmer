# json-skimmer

`json-skimmer` is an incremental, event-driven JSON parser designed for
long-lived streams. It discovers top-level objects and arrays, reports malformed
candidates, and can continue scanning for later JSON instead of terminating.

The core parser is dependency-free and does
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

[json-volcano 1.0](https://crates.io/crates/json-volcano) provides programmable,
exact-size seeded fixtures. Its published library is a development dependency
with `default-features = false`, so Clap and generator code are not parser
runtime dependencies. Tests cover valid/invalid/mixed/truncated input, root
shapes, depths, sizes and chunk divisions. Independent tree-generated tests
remain in place. No sibling generator checkout or Cargo patch is required.

## Project documents

- [Design](docs/design.md) describes the intended behavior, architecture, and
  memory model.
- [FLON requirements](docs/flon-requirements.md) defines the minimum `1.0.0`
  capabilities needed to rebuild FLON in Rust.
- [Roadmap](ROADMAP.md) distinguishes the current implementation from planned
  work.

- [Test coverage](docs/test-coverage.md) records tested contracts and remaining gaps.

## Release plan

The initial 0.1.0 releases are published. The next coordinated releases are
**1.0.0**, json-skimmer first and FLON second. The core feature scope and
remaining validation gates are tracked in the [roadmap](ROADMAP.md).
See [release instructions](docs/releasing.md) for checks and publication order.
Convenience adapters remain later 1.x candidates; no release dates are assigned.

## Streaming example and stable contract

The [read-loop example](examples/scan.rs) handles interrupted reads, EOF,
resource limits, speculative statistics and recovery without retaining whole
candidates. Run it with `cargo run --example scan < input.json`.

Fragments borrow the current input chunk. Their boundaries are unspecified:
consume them as raw bytes, since UTF-8 and escapes can cross fragments. The
parser coalesces contiguous string/number bytes within a chunk without copying.
Escaped `\uXXXX` code units are preserved, including unpaired surrogates; no
Unicode decoding or surrogate-pair enforcement is performed on escapes.

Absolute offsets cannot wrap. After consuming `u64::MAX` bytes, another
nonempty push emits `OffsetOverflow` at that offset and permanently closes the
parser in both modes, leaving the next byte unconsumed. This resource error
takes precedence over candidate limits. Empty pushes and `finish()` at that
offset remain valid. Allocation failure remains subject to Rust's allocator
behavior; configured depth limits do not guarantee successful allocation.

The 1.0 API contract and extension policy are recorded in
[API stability](docs/api-stability.md).
