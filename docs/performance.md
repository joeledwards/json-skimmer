# Parser measurements

Run the workspace `scripts/measure-json-skimmer` harness manually. It builds the
`measure` example offline in release mode and prepares seeded valid record and
giant-string fixtures, each with one and 1,024 candidates. Defaults are 64 MiB,
three repetitions, and 8 KiB/64 KiB input chunks. It installs nothing.

From the workspace root:

```bash
python3 scripts/measure-json-skimmer \
  --toolchain-bin "$PWD/.tools/rust/rustup-home/toolchains/stable-x86_64-unknown-linux-gnu/bin"
```

Use `--mib 16` for a shorter exploratory run or `--mib 1024` for an explicit
scale check. Keep repetitions at three or more. Do not run multiple campaigns
concurrently when comparing performance.

The example measures public scan-mode parser events with counters for input
bytes, raw string/number bytes (including keys), valid candidates and invalid
candidates. It does no FLON formatting, destination output or per-byte output
hashing. Counters prevent discarded computation and check repeat/chunk/mode
consistency, but do not constitute an independent correctness oracle. Invalid
input or no valid containers fails the measurement. Token bytes include
speculative fragments; these runs require valid input.

Memory mode preloads the fixture before timing, then reads slices into a chunk
buffer; it includes those copies. Stream mode reads from a newly opened file.
Internal monotonic timing includes parser/chunk-buffer setup, reads, event
counting, EOF and parser destruction; excludes file open/preload, printing and
generation. Memory mode is a parser-plus-buffer-copy baseline, not a pure
zero-copy microbenchmark. No cache flushing or warmup policy is applied.

Artifacts under `target/perf-parser-*` retain command lines, raw JSONL elapsed
nanoseconds/counts, GNU time process RSS/CPU logs, host/compiler/revision/diff,
binary and input hashes, generator version, flags and summary.json with median
and min/max throughput. RSS is per process across repetitions and includes the
input fixture in memory mode; it must not be called parser-only memory usage.
Successful fixtures are removed; failed fixtures remain for inspection.

There is no timing pass/fail threshold. Establish repeatable baselines and
machine variation before choosing regression tolerances. Smaller throughput
fixtures do not replace depth and resource-boundary tests. Allocation counts,
deep nesting, malformed recovery, sustained long-lived streams and scaling
across depths remain separate release evidence gaps. This harness does not
complete the roadmap's performance gate. Record host load, storage/cache
conditions and results alongside artifacts before drawing conclusions.

The example can also be built/run directly outside this workspace:

```bash
cargo build --offline --locked --release --example measure
./target/release/examples/measure stream fixture.json 8192 3
```

Developer-run baselines are retained in [performance results](performance-results.md).


## Allocation, nesting and repeated-candidate checks

From the workspace root, run the separate bounded resource campaign:

```bash
python3 scripts/measure-json-skimmer-resources \
  --toolchain-bin "$PWD/.tools/rust/rustup-home/toolchains/stable-x86_64-unknown-linux-gnu/bin"
```

This builds the development-only `resources` example offline. It does not use
json-volcano, files of generated input, output hashes or FLON. Each case has
three independent process repetitions, at 8 KiB and 64 KiB chunk sizes:

| Axis | Cases | Purpose |
| --- | --- | --- |
| Nesting | 1, 16, 128, 1000 nested arrays | Observe requested heap as depth increases |
| Scalar size | 16 B, 64 KiB, 1 MiB, 8 MiB string payloads at depth 1 | Check heap use is independent of raw string length |
| Repeated candidates | 1, 1000, 100000 equal-depth candidates, depth 16, 64 B payload | Check allocation/retained heap plateaus after first candidate |

Inputs are nested arrays around one string, with repeated UTF-8 and raw JSON
escapes plus an ASCII tail. One fixture is prepared before accounting begins
and reused for each successive candidate on the **same parser**. No complete
stream is buffered. Each case processes at most 10 MiB; this checks candidate
lifecycle accumulation, not hours-long runtime or OS allocator RSS stability.

The example's custom allocator delegates to Rust's System allocator and counts
successful current-thread allocation/reallocation requests. Counting starts
before parser construction and ends after parser destruction; fixture creation,
argument parsing, report serialization and subprocess setup are excluded. The
consumer is allocation-free and passes borrowed slices directly to public
`push`/`finish`. There are no I/O buffers inside the accounting interval.
This instrumentation is not linked into production json-skimmer.

`allocation_calls` includes reallocations; `requested_bytes` sums full requested
sizes (including the new full size on realloc). `peak_live_bytes` tracks logical
live requested bytes, treating successful realloc as an instantaneous resize;
it excludes allocator metadata, fragmentation and transient realloc copies.
`retained_after_first` and `retained_at_end` are snapshots with the parser alive.
A retained stack allocation after a candidate is permitted and can be reused.
`live_after_drop` must be zero. These metrics are **not process RSS** or timing
benchmarks. Only allocations created and freed within the accounting interval
are valid; the example flags accounting underflow instead of accepting it.

The runner checks exact candidate/input counts, no invalid events, no additional
allocations or heap growth after the first equal-depth candidate, complete
release on drop, equal heap metrics across token sizes, and equal heap metrics
across stream lengths. It retains raw JSON results, source snapshots, commands,
build identity, compiler and host under `target/perf-resources-*`. Review the
depth rows to assess scaling; no allocator-specific exact byte sizes are part
of the parser contract. A growth check failure is a finding to investigate,
not a reason to relax the result silently.

Unit tests cover small valid strings, UTF-8/escapes, varied chunk sizes,
invalid/truncated input cleanup and the allocator's allocate/reallocate/drop
accounting. Larger-scale cases above remain developer-run. Resource-boundary,
recovery and exact event correctness are also covered by the main parser suite.
These synthetic checks do not replace diverse long-running production inputs.
