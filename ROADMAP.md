# Roadmap

This roadmap tracks intended work, not release promises. The design contract is
documented in [`docs/design.md`](docs/design.md).

The minimum feature set for the first stable release is defined by the
[`1.0.0` FLON requirements](docs/flon-requirements.md). Items not needed by
FLON may still be included in `1.0.0`, but they must not delay validation of the
FLON use case unless they are part of the promised public API.

## Current foundation

The following code exists but has not yet been compiled or run in the current
development environment because a Rust toolchain is unavailable:

- [x] Dependency-free crate scaffold
- [x] Incremental `push` interface with borrowed event fragments
- [x] Discovery of top-level objects and arrays
- [x] Object and array structural state
- [x] Strings, escapes, numbers, booleans, and null
- [x] Candidate start, completion, and invalid events with byte offsets
- [x] Forward-only discard recovery
- [x] Strict single-container mode
- [x] Initial chunk-boundary and recovery unit tests

## Milestone 1: correct bounded-memory event parser

- [ ] Compile the crate and resolve all diagnostics
- [ ] Apply `rustfmt` and pass `clippy`
- [ ] Validate UTF-8 incrementally across chunk boundaries
- [ ] Coalesce contiguous string and number fragments within each input chunk
- [ ] Add configurable nesting and candidate-span limits
- [ ] Make parser completion and post-completion behavior explicit
- [ ] Expand grammar tests for every string escape and number state
- [ ] Test every lexical token across every possible chunk boundary
- [ ] Test arbitrary non-JSON bytes before, between, and after candidates
- [ ] Test mismatched delimiters, trailing commas, truncated input, and overflow
- [ ] Ensure arbitrary input cannot panic
- [ ] Document the public API with runnable examples

## Milestone 2: FLON-enabling event adapters

- [ ] Expose discard recovery as an explicit configuration policy
- [ ] Implement a path-tracking adapter with active keys and array indexes
- [ ] Provide decoded keys and string values with configurable token limits
- [ ] Preserve validated number text without numeric conversion
- [ ] Emit path-aware object and array openings, including empty containers
- [ ] Add a synchronous `std::io::Read` adapter or complete integration example
- [ ] Port the `node-flon` fixtures as compatibility tests
- [ ] Add junk recovery, Unicode, escaping, precision, and chunking fixtures
- [ ] Build a small reference FLON formatter using only the public API
- [ ] Demonstrate memory proportional to depth, active keys, and token limits

## Milestone 3: additional promised parser modes

- [ ] Implement bounded replay recovery
- [ ] Specify behavior when replay retention limits are reached
- [ ] Implement configurable candidate buffering
- [ ] Emit complete owned objects and arrays after successful candidates
- [ ] Define a lossless owned value preserving order, duplicate keys, and number text
- [ ] Add raw-byte emission for complete candidates
- [ ] Test speculative-event handling and failed-candidate cleanup
- [ ] Test recovery from valid containers nested inside malformed candidates

## Milestone 4: 1.0.0 release readiness

- [ ] Evaluate async stream adapters without coupling the core to a runtime
- [ ] Add optional `serde` conversion and typed deserialization
- [ ] Add property tests comparing accepted containers with a reference parser
- [ ] Add fuzz targets for parsing, chunk division, and recovery
- [ ] Benchmark throughput, allocations, nesting, and large tokens
- [ ] Review crate metadata, licensing files, and crates.io documentation
- [ ] Establish a minimum supported Rust version
- [ ] Perform an API and unsafe-code review
- [ ] Verify the reference FLON formatter acceptance test
- [ ] Publish the initial crates.io release before creating the public GitHub repository

## Possible future work

- Discover top-level scalar JSON values when explicitly enabled
- Decoded string-fragment events in addition to lossless raw fragments
- Selective path matching similar to streaming JSON query tools
- `no_std` plus `alloc` support if the core architecture permits it
