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

Allocation counts, depth scaling and long-lived streams remain open. Existing
unit/reference/fuzz evidence is separate from these valid-input measurements.
