# Release roadmap

## Release sequence and scope

Publish initial **0.1.0 releases** to establish the crate names and make the
implemented streaming core available. Publish json-skimmer first, then FLON
against json-skimmer 0.1.0. json-volcano 1.0.0 is already published and is a
registry development dependency in both consumers, with CLI features disabled.
See [release instructions](docs/releasing.md) for checks, publication order and
known pre-1.0 limitations. External checks and publication remain developer-run.

The 1.0 checklists below remain the stability target, rather than a claim that
0.1 has passed every 1.0 acceptance gate. Parser decoding/path convenience
adapters are later work, not prerequisites for either initial release. FLON
already consumes only public raw parser events. Checked items represent
implemented/verified work; pending tests and audits stay unchecked.

## 1.0.0 — core parser

- [x] Dependency-free incremental `push` API with borrowed raw fragments
- [x] Discover root objects/arrays and emit ordered container and scalar events
- [x] Preserve raw JSON escapes and exact number spelling
- [x] Validate UTF-8 incrementally across chunk boundaries
- [x] Scan mode with documented forward-only discard recovery
- [x] Strict single-container mode
- [x] Candidate lifecycle, structured errors, and absolute byte offsets
- [x] Configurable depth limits; default 1,000 containers
- [x] Final/idempotent EOF, terminal errors, and frozen-offset semantics
- [ ] Coalesce contiguous string and number fragments within each input chunk;
      preserve semantics without promising particular fragment sizes
- [x] Add configurable candidate-span limits with documented counting,
      boundary, and recovery behavior
- [ ] Audit offset/counter overflow and define explicit failure behavior
- [ ] Finish systematic grammar coverage for escapes, number states, literals,
      separators, mismatched delimiters, and truncation across chunk boundaries
- [x] Add deterministic generated trees, mutations, truncations, arbitrary bytes,
      and chunk/lifecycle properties
- [ ] Compare accepted containers with an independent reference JSON parser
- [ ] Add and exercise fuzz targets for parsing, chunk division, and recovery
- [ ] Add runnable public API examples, including a synchronous `Read` loop,
      speculative-event handling, recovery, limits, and EOF

## 1.0.0 — acceptance and release gates

- [x] Demonstrate a FLON consumer using only public parser APIs
- [x] Port distinct active Node fixtures and extended Rust compatibility cases
      in FLON; add UTF-8, recovery, precision, and chunking fixtures
- [ ] Reverify FLON compatibility against the final release API
- [ ] Benchmark throughput, allocations, nesting, long-lived streams, and large
      tokens; demonstrate memory proportional to depth in the parser and active
      keys/configured output storage in FLON
- [ ] Review public API stability and unsafe-code usage; check that later
      adapters can be added without changing the core event contract
- [ ] Establish and test a minimum supported Rust version
- [x] Complete metadata, licensing files, and initial crate documentation
- [ ] Run final unit, documentation, formatting, lint, and authorized release checks
- [ ] Publish `json-skimmer` 1.0.0, then allow FLON to use the published dependency

## 1.x — additive convenience features (scope reviewed per release)

These are candidates for minor releases, not commitments to specific minor
version numbers. Design each around demonstrated consumer needs and explicit
memory limits. Review semver compatibility before assigning a release.

- [ ] Add decoded key/string adapters with correct escaped-Unicode handling
      and configurable token limits; consider decoded streaming fragments
- [ ] Add reusable path tracking with active keys, array indexes, and path-aware
      container openings, including empty containers
- [ ] Add a synchronous `Read` convenience adapter beyond the 1.0 example
- [ ] Consider explicit recovery-policy configuration when another policy exists
- [ ] Add bounded complete-candidate raw-byte buffering/emission
- [ ] Design a lossless owned value preserving order, duplicate keys, and number
      spelling; add bounded complete-container emission
- [ ] Add optional Serde conversion and typed deserialization
- [ ] Evaluate async adapters without coupling the parser core to a runtime
- [ ] Design bounded replay recovery, including overlapping/speculative events,
      retention exhaustion, cleanup, and nested-candidate recovery tests

## 2.0 and later — changes requiring a breaking-contract review

A feature does not require 2.0 merely because it is substantial. Use a major
release when implementation would change an established public contract.

- [ ] Review changes to event shapes/lifetimes, error types, lifecycle, offsets,
      default limits, or recovery semantics for compatibility before scheduling
- [ ] Consider opt-in root scalar discovery, selective path matching, and
      `no_std` plus `alloc`; assign to 1.x only if fully additive
- [ ] Move any convenience proposal that cannot preserve the 1.x contract into
      an explicitly designed major release
