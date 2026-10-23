# Test coverage

Update this matrix when public behavior changes. Tests live in `src/lib.rs`,
`src/contract_tests.rs`, `src/generated_tests.rs`, and `src/volcano_tests.rs` and use public parser
events and methods.

| Behavior | Coverage | Remaining gaps |
| --- | --- | --- |
| Measurement consumer | `examples/measure.rs`: valid/invalid/truncated inputs, UTF-8/escapes/numbers, whole/every split/one-byte normalized counters and read failures; workspace Python summary tests | Actual throughput/RSS developer-run; allocation/deep/long-lived measurements remain open |
| Containers, literals, strings, escapes, number spellings | Valid grammar corpus; completion spans; generated trees with independent semantic-event traversal | Decoded adapters (outside core scope) |
| Invalid grammar and truncation | Invalid grammar corpus; exact error kinds, candidate starts, offsets; every invalid escape byte and hex position, incomplete lexical/structural states, number/separator/mismatched-delimiter cases across chunks | Expand cases with each bug fix; external reference-parser comparison |
| Fragment delivery | Borrowed contiguous runs within each chunk; valid/invalid/truncated strings, UTF-8, escapes and numbers; flush-before-error/recovery and every split; normalized whole/split/one-byte corpus | Performance measurements |
| Chunk independence | Whole input, every two-chunk split, one-byte chunks, eight deterministic random partitions; normalized fragments | More complex generated documents |
| Recovery | Malformed candidates, conflicting opener, arbitrary leading byte; absolute spans and offsets; depth-limit recovery consumes the rejected opener and resets for later candidates across chunk divisions | Broader generated recovery corpus |
| Strict mode | Missing roots, scalar roots, trailing data, frozen terminal offsets independent of chunk divisions | Broader generated strict-mode corpus |
| Completion | Final/idempotent finish on valid, empty, incomplete, and invalid streams in both modes; no post-close callbacks or offset changes; EOF event ordering; terminal UTF-8, syntax, and depth errors | Broader generated lifecycle sequences |
| UTF-8 | Boundary/common-text generators; all 256 replacements at each continuation position compared with Rust UTF-8 validation; every incomplete prefix at quote/EOF; mixed scripts, emoji, combining characters, escapes, raw fragment preservation, repeated recovery; keys and values | Broader arbitrary-byte/property corpus; exhaustive Unicode scalar coverage |
| Candidate spans | Exact and one-byte-over boundaries, every truncation prefix of UTF-8/escape/number/literal/nested inputs, invalid input, strict/scan recovery, absolute offsets and frozen lifecycle, per-candidate reset, offending-opener consumption, size/depth precedence; whole/every two-chunk split/one-byte input with normalized events | Offset exhaustion covered separately |
| Resource bounds and resilience | Generated object/array/mixed-depth boundaries, default 1000, raised/custom limits, no rejected opening event, sibling/scalar accounting, strict/scan recovery across chunk divisions | External reference-parser comparison and fuzzing |

Run lightweight unit tests from this repository:

```bash
../../scripts/with-rust cargo test --offline --lib
```


UTF-8 generators use Rust character encoding and `std::str::from_utf8` as
independent references. Generated mutation cases run as whole input and one-byte
chunks; focused text, truncation, and recovery cases also run every two-chunk
split and deterministic random partitions. Failures identify the source
character, mutated byte position, replacement, and chunk division.

## Bounded generated properties

`src/generated_tests.rs` adds dependency-free deterministic generators:

- 128 valid trees with up to four container levels and three children per
  generated container. A separate tree traversal supplies expected semantic
  events, including raw strings/numbers, object order, and duplicate keys.
- Every proper prefix and every single-byte `0xff` replacement of 32 generated
  documents. Strict mode must reject them; scan mode must discover a sentinel
  appended after a NUL recovery boundary. Intermediate nested candidates are
  permitted by discard recovery.
- 256 byte streams of at most 96 bytes, alternating unrestricted bytes with a
  JSON-like alphabet, exercised in both modes.

Each generated input is checked whole, at a midpoint split, in one-byte chunks,
and with a seeded variable-size partition. Observations normalize fragments,
verify fragment bytes against source offsets, check candidate lifecycle and span
bounds, and assert final/idempotent completion. Assertion diagnostics include
seed, input, mutation position where applicable, and chunk cuts. The suite is
bounded and does not establish correctness for all arbitrary inputs or replace
fuzzing or comparison with an external JSON parser.

Run just these properties from this repository:

```bash
../../scripts/with-rust cargo test --offline --lib generated_tests
```

## Companion generator

Generation contracts and tests belong to the independently published
[json-volcano project](https://crates.io/crates/json-volcano), which exposes
both a library and optional CLI. Its library is a registry development
dependency with default features disabled; no sibling checkout is required.
Generator tests belong to that project, not this parser's Cargo workspace.

Published json-volcano 1.0 fixtures are tested in `src/volcano_tests.rs`: both
shapes, valid/invalid/mixed/truncated lifecycles, candidate counts, EOF errors,
Unicode/escapes/numbers/depth and deterministic seeds. Events and tokens are
normalized across whole input, every two-chunk split and one-byte chunks.
Independent tree-generated semantic oracles remain in `src/generated_tests.rs`.

Generator consumer tests also cover object-only/array-only/mixed roots, bounded
infinite prefixes, parser-enforced maximum depths 1/4/128 and deterministic
depth-bias ordering. Value-size tests observe string payload bounds and ordered
size-bias means. Finite variation scenarios run whole/every split/one-byte.
Gaps: signal handling is manual; number-magnitude and record byte-size
distributions and broader randomized tree topology are not implemented.

## Offset exhaustion and API examples

Seeded-counter tests exercise public push/finish behavior near `u64::MAX` in
both modes: terminal overflow without consumption across chunk divisions,
frozen lifecycle, and valid literal/string/number/candidate spans ending exactly
at the maximum. A natural exabyte-scale run is not required. The counter audit
and stable defaults are recorded in [API stability](api-stability.md).

`examples/scan.rs` demonstrates synchronous Read, interrupted-read retry,
resource limits, speculative statistics, recovery and mandatory EOF. Its two
isolated tests exercise mixed/truncated input and input error propagation:

```bash
../../scripts/with-rust cargo test --offline --locked --example scan
```

## Prepared reference and fuzz validation

The Python differential harness passed the developer-run 655-input corpus
with 401 recovered candidates. The developer-run bounded ASan fuzz campaign
passed 220,123 executions in 61 seconds using the native C++ linker, with no
reported crash or invariant/sanitizer failure. Configured maximum input length
was 4 KiB; the adaptive length limit reached 461. Rust 1.87.0 library/example
tests passed. See
[validation instructions](validation.md) for commands, bounds and known MSRV
constraints and remaining validation gaps.


## Development resource probe

`examples/resources.rs` and `examples/support/allocation.rs` test public parser
candidate counts and requested heap behavior for depth 1/8/64, payloads 0/9/4096,
and chunks 1/7/8192, repeated candidates, malformed/truncated cleanup and
allocator growth/drop accounting. Instrumentation is current-thread and limited
to the parser lifetime; fixture generation/reporting is excluded. Workspace
Python tests validate reporting, bounded workloads and cross-case growth checks.
See [performance](performance.md) for the developer-run larger resource matrix.
Gaps: sustained hours-long workloads, allocator RSS/fragmentation and more varied
candidate sequences; finite synthetic success does not prove universal bounds.

Developer-run resource campaign passed all 66 checks. The retained
[results](performance-results.md) cover depth through 1,000, strings through
8 MiB and 100,000 equal-depth successive candidates, two chunk sizes and
three process repetitions. No post-first-candidate allocation or post-drop
tracked allocation remained. Extended varied-input soak and allocator RSS
remain outside this bounded campaign.
