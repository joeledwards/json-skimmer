# Release roadmap

## Release sequence and scope

The 0.1.0 releases are published and their GitHub repositories are public.
The next coordinated release is **1.0.0**: finish the core and validation gates,
publish json-skimmer first, then FLON against that registry dependency.
json-volcano 1.0.0 remains a library-only development dependency.

Parser decoding/path convenience adapters remain later work. FLON consumes
public raw parser events. Checked items represent implemented/verified work;
pending validation and publication remain unchecked. See
[API stability](docs/api-stability.md) and the coverage matrix for contracts.

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
- [x] Coalesce contiguous string and number fragments within each input chunk;
      preserve semantics without promising particular fragment sizes
- [x] Add configurable candidate-span limits with documented counting,
      boundary, and recovery behavior
- [x] Audit offset/counter overflow and define explicit failure behavior
- [x] Finish systematic grammar coverage for escapes, number states, literals,
      separators, mismatched delimiters, and truncation across chunk boundaries
- [x] Add deterministic generated trees, mutations, truncations, arbitrary bytes,
      and chunk/lifecycle properties
- [x] Compare accepted containers with an independent reference JSON parser
- [x] Add and exercise fuzz targets for parsing, chunk division, and recovery
      (bounded 60-second ASan campaign; see docs/validation.md)
- [x] Add runnable public API examples, including a synchronous `Read` loop,
      speculative-event handling, recovery, limits, and EOF

## 1.0.0 — acceptance and release gates

- [x] Demonstrate a FLON consumer using only public parser APIs
- [x] Port distinct active Node fixtures and extended Rust compatibility cases
      in FLON; add UTF-8, recovery, precision, and chunking fixtures
- [x] Reverify FLON compatibility against the local 1.0 release-candidate API
- [x] Measure parser throughput, allocations, nesting, large tokens and bounded
      repeated-candidate stability; see docs/performance-results.md (250 MiB/1 GiB
      throughput and 66 resource checks; not an hours-long varied-input soak)
- [x] Review public API stability and unsafe-code usage; check that later
      adapters can be added without changing the core event contract
- [x] Establish and test a minimum supported Rust version (1.87; library/examples verified by developer)
- [x] Complete metadata, licensing files, and initial crate documentation
- [x] Run final offline unit/example/doc, formatting, lint, documentation and
      package verification checks (Rust 1.87 tests/package; stable lint/format)
- [ ] Publish `json-skimmer` 1.0.0, then allow FLON to use the published dependency

## Downstream release tracking

FLON still owns its remaining active-key/output-storage performance evidence,
reference-output comparison and final registry-dependency verification. Existing
FLON compatibility tests passed against this parser's public API. Those FLON
release tasks do not require additional parser features or block uploading the
validated parser first. Extended varied-input soak testing remains an explicit
limitation of the bounded parser campaign, not completed evidence.

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
