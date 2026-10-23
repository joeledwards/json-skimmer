# FLON requirements for 1.0.0

## Goal

The first stable `json-skimmer` release must be sufficient to rebuild the
reference `node-flon` project in Rust while preserving its defining property:
JSON is converted to grep-friendly, flattened assignments without loading the
entire input structure into memory.

This is behavioral compatibility, not bug-for-bug compatibility. The Rust
implementation should produce valid, unambiguous output for cases where the
Node implementation has incomplete escaping or recovery behavior.

## Current implementation and remaining requirements

The active `repos/flon` crate demonstrates formatting through public parser
events without whole-document retention in live mode. It owns path tracking,
limits on raw keys, streaming scalar rendering, `Read`/`Write` integration,
recovery policy, confirmed-output buffering, CLI, and color. Its unit suite
ports the distinct active Node fixtures and adds Rust-specific cases.

`json-skimmer` currently provides raw speculative events, validated UTF-8 and
JSON escapes, original number text, structured errors, strict/scan modes, and a
configurable depth limit (default 1,000). Candidate-span limits are implemented;
broader release validation remains open. Completion is final and idempotent, as documented
in the README and public API. See the release roadmap and coverage matrices.

## Coordinated release boundary

The initial releases are coordinated 0.1.0 versions: publish `json-skimmer`
first, then FLON using that published dependency. These pre-1.0 releases ship
the implemented core; the stability and validation gates below remain open
for 1.0. Parser 1.0 stabilizes raw events, strict/scan
modes, discard recovery, resource limits, lifecycle/offset/error contracts, and
usable documentation. Runnable examples, independent correctness comparisons,
fuzzing, benchmarks, and API/package review are release gates.

Decoded strings, reusable paths, replay recovery, owned values, Serde, and async
adapters are later features, not 1.0 requirements. Review them for additive 1.x
releases; use 2.0 only when an established contract must change. FLON does not
wait for those adapters. The per-repository roadmaps are the release checklists.

## Library capabilities required from json-skimmer

### Correct incremental parsing

- Accept byte chunks of arbitrary size from a long-lived input.
- Parse objects, arrays, strings, numbers, booleans, and null across any chunk
  boundary.
- Validate JSON string escapes and UTF-8.
- Preserve the original number spelling rather than converting through `f64`.
- Detect candidate completion and malformed candidates.
- Continue scanning after malformed input using discard recovery.
- Apply configurable limits without recursive descent on the process stack.

### Paths and values supplied by FLON

FLON retains raw active object keys, tracks zero-based array indexes, and emits
ordered container declarations and scalar assignments. Its memory is
proportional to depth, active keys, and configured output storage. Strings and
numbers stream as validated raw fragments; typed events represent booleans and
null. No reusable parser path adapter or whole scalar allocation is needed.

Future decoding/path adapters must separately define escaped-Unicode handling,
ownership, token limits, and memory behavior before being assigned a release.

### Stream integration

- Provide a straightforward synchronous integration with `std::io::Read` or a
  documented read-loop example.
- Allow events or path/value records to be written incrementally to an
  arbitrary `std::io::Write` destination.
- Treat a downstream closed pipe as an application concern rather than a parser
  error.

### Stable public contract

- Document which events are speculative until `CandidateEnd`.
- Document ownership and callback lifetimes for borrowed fragments.
- Expose absolute byte offsets and structured error kinds.
- Define behavior at EOF and after terminal errors.
- Include compatibility fixtures derived from the `node-flon` tests.

## FLON application behavior

The Rust FLON project, rather than `json-skimmer`, owns:

- rendering `json = {};` or `json = [];` for the root;
- rendering identifier-safe keys with dot notation and other keys with bracket
  notation;
- escaping keys and string values for valid assignment output;
- emitting parent container declarations in encounter order;
- optional terminal colors for paths, indexes, strings, numbers, booleans, and
  null;
- reading stdin or a selected file;
- stdin integration with external URL-fetching tools;
- buffered versus immediate output;
- elapsed-time summaries;
- CLI arguments, environment configuration, diagnostics, and broken-pipe
  handling.

The initial compatibility suite should cover all current `node-flon` fixtures:
objects, arrays, empty containers, nested mixed containers, unusual keys,
multiple consecutive documents, all scalar types, and junk between valid
candidates. It should add cases for backslashes, control characters, Unicode,
large and precise numbers, malformed candidates, and arbitrary chunking.

## Additional Rust libraries

No additional custom foundational library is currently required to implement
FLON. The formatter and path-to-assignment policy should initially live in a
Rust FLON crate that exposes both a library API and a binary. Splitting it out
before a second consumer appears would create an artificial abstraction.

Application concerns can use the Rust standard library or established crates:

- `BufWriter` for output buffering and `Instant` for timing;
- a maintained argument parser for the CLI;
- a maintained ANSI styling crate for terminal color and TTY detection; and
- external tools such as curl for URL fetching; built-in HTTP is out of scope.

A Rust equivalent of `@buzuli/color` may eventually be worthwhile as a separate
utility if multiple Rust applications need the same palette and environment
contract. FLON alone does not justify making it a prerequisite for
`json-skimmer` or blocking `json-skimmer` `1.0.0` on it.

## 1.0.0 acceptance test

The active FLON formatter already uses only public parser APIs. Before
publishing `json-skimmer` `1.0.0`, verify that it continues to pass the
compatibility fixtures while processing incrementally with memory proportional
to active depth, active keys, and configured output buffering. Raw scalar
fragments must remain streamable without a complete-token allocation. Future
decoding adapters must separately demonstrate their configured token limits.
