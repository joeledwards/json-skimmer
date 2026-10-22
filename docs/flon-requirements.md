# FLON requirements for 1.0.0

## Goal

The first stable `json-skimmer` release must be sufficient to rebuild the
reference `node-flon` project in Rust while preserving its defining property:
JSON is converted to grep-friendly, flattened assignments without loading the
entire input structure into memory.

This is behavioral compatibility, not bug-for-bug compatibility. The Rust
implementation should produce valid, unambiguous output for cases where the
Node implementation has incomplete escaping or recovery behavior.

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

### Path tracking

A supplied adapter must track the path to each value:

- decoded object keys;
- zero-based array indexes;
- object versus array container kind; and
- container open/close and scalar completion.

Its retained state should be proportional to nesting depth and the active keys,
not the total number of values already processed. The adapter must correctly
advance array indexes after scalar, object, and array elements.

### Values suitable for formatting

The consumer must be able to write each FLON assignment when its value becomes
complete:

- strings as decoded text that can be escaped for the output notation;
- numbers as their validated original text;
- booleans and null as typed events; and
- empty and non-empty container declarations with their paths.

Because a FLON assignment cannot be emitted until its key and scalar value are
known, the adapter may accumulate the current key or scalar token. Such storage
must have configurable limits and must not imply buffering the enclosing
document.

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

The future Rust FLON project, rather than `json-skimmer`, owns:

- rendering `json = {};` or `json = [];` for the root;
- rendering identifier-safe keys with dot notation and other keys with bracket
  notation;
- escaping keys and string values for valid assignment output;
- emitting parent container declarations in encounter order;
- optional terminal colors for paths, indexes, strings, numbers, booleans, and
  null;
- reading stdin or a selected file;
- optional URL input and TLS policy;
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
- a maintained HTTP client if URL input remains a requirement.

A Rust equivalent of `@buzuli/color` may eventually be worthwhile as a separate
utility if multiple Rust applications need the same palette and environment
contract. FLON alone does not justify making it a prerequisite for
`json-skimmer` or blocking `json-skimmer` `1.0.0` on it.

## 1.0.0 acceptance test

Before publishing `json-skimmer` `1.0.0`, a small reference FLON formatter must
be implementable using only its public API. That formatter must pass the
compatibility fixtures while processing incrementally with memory proportional
to active depth, active keys, and the configured maximum scalar size.
