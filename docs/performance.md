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
