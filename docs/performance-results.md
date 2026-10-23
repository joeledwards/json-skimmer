# Parser performance results

## 2026-10-23 — initial 1.0.0 baseline on hulk

Developer-run release builds through the workspace measurement harness. All
16 configurations per size completed three repetitions and passed input,
token-byte and candidate count consistency checks. These are throughput and
shallow-input RSS observations, not completion of all release validation.

| Fixture | Records: range of configuration medians | Strings: range of configuration medians | Artifacts |
| --- | --- | --- | --- |
| 250 MiB | 151.82–157.01 MiB/s | 145.24–148.54 MiB/s | [Summary](performance-results/2026-10-23-250mib/summary.json) |
| 1 GiB | 152.20–156.69 MiB/s | 145.75–149.02 MiB/s | [Summary](performance-results/2026-10-23-1024mib/summary.json) |

Each artifact directory retains method/host/compiler/build identity, local diff,
commands, per-repetition JSONL, input and executable hashes, and GNU time logs.
Generated inputs are excluded. The captured revision alone does not describe
the uncommitted measurement example; consult the saved diff and this commit's
example. No per-byte output hashing or FLON formatting is included.

All corresponding configuration medians changed by less than 3% between sizes.
The 250 MiB run had less than 5% min/max throughput spread within every
configuration. Its streaming process peak RSS was 1964–2048 KiB; preloaded
processes held the fixture and used roughly 251 MiB. RSS is process-level,
not parser-only allocation accounting. See raw logs for 1 GiB process RSS.

Use 512 MiB for future routine comparisons; this size is a practical choice,
not an additional measured result. Keep three repetitions and compare matching
shapes, candidate counts, chunks and modes. Cache was not flushed and background
load was uncontrolled. There is no timing pass/fail threshold.

Allocation/depth/repeated-candidate evidence follows below. Existing
unit/reference/fuzz evidence is separate from these valid-input measurements.


## 2026-10-23 — parser resource checks on hulk

Developer-run `scripts/measure-json-skimmer-resources`: all **66 checks passed**
(11 cases, two chunk sizes, three independent process repetitions).
[Raw results](performance-results/2026-10-23-resources/results.json), method,
compiler/host/build identity, commands, source snapshots and executable hash are
retained in the same directory. No generated fixture or executable is included.
The build used local uncommitted resource examples on top of parser commit
81a7f82; exact example inputs are retained as source snapshots.

| Workload | Successful allocation/reallocation calls | Peak live requested heap |
| --- | ---: | ---: |
| Depth 1 | 1 | 8 B |
| Depth 16 | 3 | 32 B |
| Depth 128 | 6 | 256 B |
| Depth 1,000 | 9 | 2,048 B |
| String sizes 16 B / 64 KiB / 1 MiB / 8 MiB, depth 1 | 1 in every case | 8 B in every case |
| 1 / 1,000 / 100,000 successive candidates, depth 16 | 3 in every case | 32 B in every case |

Heap metrics were identical across chunk sizes and repetitions. No allocation
occurred after the first equal-depth candidate. Retained requested heap was
unchanged as completed-candidate counts increased. All tracked allocations
were released on parser drop. The depth results are consistent with stack
capacity growing with depth; exact sizes are observations, not API promises.

Accounting excludes input fixtures and reporting, and measures requested
current-thread heap bytes, not process RSS, allocator overhead, fragmentation
or transient realloc-copy peaks. Fixture payloads contain UTF-8 and raw escapes.
Repeated candidates reuse one fixture on one parser, covering lifecycle
accumulation rather than hours-long elapsed runtime or diverse production
traffic. This is bounded evidence for depth/scalar-size/candidate-count scaling;
extended varied-duration soak testing remains outside this campaign.
