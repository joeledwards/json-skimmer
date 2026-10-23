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

## Project documents

- [Design](docs/design.md) describes the intended behavior, architecture, and
  memory model.
- [FLON requirements](docs/flon-requirements.md) defines the minimum `1.0.0`
  capabilities needed to rebuild FLON in Rust.
- [Roadmap](ROADMAP.md) distinguishes the current implementation from planned
  work.
