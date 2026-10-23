# Test coverage

Update this matrix when public behavior changes. Tests live in `src/lib.rs`
and `src/contract_tests.rs` and use public parser events and methods.

| Behavior | Coverage | Remaining gaps |
| --- | --- | --- |
| Containers, literals, strings, escapes, number spellings | Valid grammar corpus; completion spans | Exhaustive lexical transitions; decoded adapters |
| Invalid grammar and truncation | Invalid grammar corpus; exact error kinds, candidate starts, offsets | Expand cases with each bug fix |
| Chunk independence | Whole input, every two-chunk split, one-byte chunks, eight deterministic random partitions; normalized fragments | Broader generated/property corpus |
| Recovery | Malformed candidates, conflicting opener, arbitrary leading byte; absolute spans and offsets | Depth-limit recovery, including consuming the rejected opener |
| Strict mode | Missing roots, scalar roots, trailing data | Explicit post-finish contract |
| UTF-8 | Boundary/common-text generators; all 256 replacements at each continuation position compared with Rust UTF-8 validation; every incomplete prefix at quote/EOF; mixed scripts, emoji, combining characters, escapes, raw fragment preservation, repeated recovery; keys and values | Broader arbitrary-byte/property corpus; exhaustive Unicode scalar coverage |
| Resource bounds and resilience | Generated object/array/mixed-depth boundaries, default 1000, raised/custom limits, no rejected opening event, sibling/scalar accounting, strict/scan recovery across chunk divisions | Candidate-span limits; overflow policy; bounded property tests and fuzzing |

Run lightweight unit tests from this repository:

```bash
../../scripts/with-rust cargo test --offline --lib
```


UTF-8 generators use Rust character encoding and `std::str::from_utf8` as
independent references. Generated mutation cases run as whole input and one-byte
chunks; focused text, truncation, and recovery cases also run every two-chunk
split and deterministic random partitions. Failures identify the source
character, mutated byte position, replacement, and chunk division.
