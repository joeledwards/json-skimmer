# json-skimmer design

## Purpose

`json-skimmer` parses JSON incrementally from streams that may be unbounded and
may contain non-JSON or malformed data. Its primary job is to discover
top-level objects and arrays, emit events while walking their contents, report
invalid candidates, and continue looking for later candidates.

The parser must also support stricter and more convenient behavior through
layers built on the same event stream. It is not intended to require a complete
document in memory unless a caller explicitly selects a buffering adapter.

Version `1.0.0` must provide enough stable functionality to implement a Rust
version of FLON without replacing or bypassing the parser. The concrete
acceptance requirements are recorded in
[`flon-requirements.md`](flon-requirements.md).

## Root values

Object (`{...}`) and array (`[...]`) containers are the supported top-level
candidates. Strings, numbers, booleans, and null are valid inside containers,
but do not independently start candidates.

Supporting every JSON value as a top-level candidate may be added later. It is
not a near-term requirement because scalar values are ambiguous when searching
arbitrary text.

## Layers

The design separates parsing from policies that require retained data:

```text
input byte chunks
    -> incremental lexical and structural parser
    -> candidate lifecycle events
    -> optional adapters
        |-- path tracking
        |-- complete container emission
        |-- replay recovery
        |-- strict single-container parsing
        `-- serde conversion/deserialization
```

The core parser owns lexical state and a structural stack. It does not own a
complete candidate. Adapters may retain keys, paths, tokens, or candidate bytes
according to their contracts.

The FLON use case is served by the path-tracking layer, not by the
complete-container layer. It must be possible to format a value as soon as that
value is complete while retaining only the active path and current token.

## Candidate lifecycle

Scanning begins outside a candidate. In scan mode, bytes are ignored until an
opening `{` or `[` is found. That byte starts a candidate and establishes its
root container kind.

A candidate produces `CandidateStart`, lexical and structural events, and then
exactly one of:

- `CandidateEnd` when its root container closes successfully; or
- `Invalid` when its syntax is invalid or a configured resource limit is
  exceeded.

Events emitted before `CandidateEnd` are speculative. Consumers that cannot act
on an invalid candidate must buffer those events or their effects until the
candidate is confirmed.

## Parsing modes

### Raw event mode

Raw mode emits candidate, container, key, string, number, boolean, null, and
error events. Large strings and numbers are represented as fragments so memory
use does not depend on token length.

### Complete-container mode

This adapter buffers a candidate and emits an owned object or array only after
`CandidateEnd`. Buffering is explicit and configurable. An invalid candidate
is discarded rather than emitted.

The initial owned representation should preserve object entry order, duplicate
keys, and the original representation of numbers. Conversion to lossy or typed
representations belongs in separate adapters.

### Single-container mode

Single mode accepts exactly one object or array surrounded by JSON whitespace.
Leading non-whitespace, invalid syntax, trailing non-whitespace, or end of input
before the root closes is terminal for that parser instance.

## Recovery policies

### Discard

Discard is the default scanning policy. After `Invalid`, all state for the
candidate is dropped and scanning resumes forward. Previously consumed opening
delimiters are not reconsidered. If the byte exposing the conflict is itself
`{` or `[`, it may immediately begin the next candidate.

Discard recovery requires no candidate buffer and is the primary policy for
long-lived diagnostic tools.

### Replay

Replay is optional. It retains candidate bytes or bounded checkpoints so an
opening delimiter consumed inside a failed candidate can be reconsidered as a
possible candidate start.

Replay must require an explicit retention limit. Reaching that limit produces
an invalid or limit event and falls back to documented behavior; it must never
allow unbounded memory growth.

## State and memory model

The core parser retains only:

- the lexical state needed across chunk boundaries;
- the current absolute byte offset;
- candidate lifecycle state;
- one structural frame per active object or array; and
- any explicitly configured limits or counters.

Core memory is therefore proportional to nesting depth, excluding allocator
capacity. A path adapter additionally retains active object keys and array
indexes. A complete-container or replay adapter additionally retains bounded or
caller-approved candidate data.

No input chunk boundary may have semantic significance. UTF-8 code points,
escape sequences, numbers, literals, keys, and containers may all cross chunk
boundaries.

## Validity and offsets

Accepted candidates must follow the JSON grammar, including UTF-8 validity,
string escaping, number syntax, separators, and nesting. Errors carry absolute
byte offsets. Candidate completion carries the half-open byte span of the root
container.

Raw fragments borrow from the input passed to the current parser call. A
consumer must copy a fragment if it needs the bytes after the callback returns.

Lossless raw fragments alone are insufficient for every consumer. The `1.0.0`
API must also provide a bounded way to obtain decoded object keys and string
values. This may be a token accumulator with explicit limits, a decoding
adapter, or decoded fragments with correct handling of escaped Unicode. Number
text must remain available without conversion through a floating-point type.

## Resource limits

The public configuration is expected to support limits for at least:

- nesting depth;
- accumulated key or token size in adapters;
- buffered candidate size;
- replay checkpoints or replay bytes; and
- optionally, the maximum candidate span.

In scanning mode a limit violation invalidates the current candidate and
recovery continues. In single mode it is terminal. Defaults must be documented
and must not accidentally make raw parsing proportional to document size.

## Safety and resilience

The parser must not panic on arbitrary input. Malformed input is normal in scan
mode and is represented by events. Integer overflow in offsets or counters must
be handled explicitly. Deeply nested input must be constrained by configuration
rather than process stack recursion.

The parser core should remain dependency-free unless a dependency provides a
clear correctness or maintenance advantage. Optional ecosystem integrations
must be feature-gated.

## Non-goals for the initial release

- Repairing malformed JSON or guessing the author's intent.
- Treating uncontained scalar text as a discoverable document.
- Silently converting numbers to a fixed numeric type.
- Hiding buffering behind an API that appears bounded-memory.
- Guaranteeing that speculative raw events belong to a candidate that will
  eventually be valid.
