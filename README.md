# json-skimmer

`json-skimmer` is an incremental, event-driven JSON parser designed for
long-lived streams. It discovers top-level objects and arrays, reports malformed
candidates, and can continue scanning for later JSON instead of terminating.

The crate is in early development. Its core parser is dependency-free and does
not buffer complete documents.

## Project documents

- [Design](docs/design.md) describes the intended behavior, architecture, and
  memory model.
- [FLON requirements](docs/flon-requirements.md) defines the minimum `1.0.0`
  capabilities needed to rebuild FLON in Rust.
- [Roadmap](ROADMAP.md) distinguishes the current implementation from planned
  work.
