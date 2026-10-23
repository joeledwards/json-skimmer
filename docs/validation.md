# 1.0 validation

Unit tests are deterministic, bounded and isolated. Run them with the workspace
wrapper, or ordinary Cargo outside this workspace:

```bash
../../scripts/with-rust cargo test --offline --locked --lib
../../scripts/with-rust cargo test --offline --locked --examples
../../scripts/with-rust cargo clippy --offline --locked --lib --examples -- -D warnings
```

## Independent JSON reference

The development-only `reference_probe` accepts bounded hex lines, checks
normalized events across whole/every two-chunk split/one-byte input and EOF
lifecycle in both modes, and reports strict acceptance and recovered spans.
`tools/reference_check.py` compares acceptance and every recovered candidate
with Python's standard JSON parser. Python receives explicit UTF-8, rejects
non-JSON NaN/Infinity, and preserves number strings so huge exponents and exact
integers do not become float-range restrictions. Both raw parsers accept
escaped surrogate code units without requiring a decoded Unicode scalar.

The fixed seed 42 corpus contains valid trees, escapes, Unicode, raw invalid
UTF-8, mutations, truncations, surrounding junk and arbitrary bytes. Probe
input is limited to 4 KiB per case and subprocess runtime to 120 seconds.
The developer ran this comparison successfully on 2026-10-23: 655 inputs and
401 recovered candidates passed strict acceptance, normalized chunk events and
EOF lifecycle checks. This bounded corpus does not establish correctness for
all inputs. Further independent reference runs require authorization under
the workspace rules.

```bash
../../scripts/with-rust cargo build --offline --locked --example reference_probe
python3 tools/reference_check.py
```

## Fuzz target

`fuzz/fuzz_targets/stream.rs` compares normalized events and consumed offsets
for whole, one-byte and input-selected chunks in scan/single modes, with
input-selected depth/candidate limits, arbitrary input and appended recovery
sentinels. It verifies terminal EOF/no post-finish events. Inputs are capped at
4 KiB and depth at 32. Counters near u64::MAX are separately seeded in unit tests.

The separate fuzz package is excluded from the published crate. Fuzz dependency
retrieval, cargo-fuzz/nightly installation and execution need developer
authorization. The developer installed cargo-fuzz 0.13.2 and nightly
1.101.0 (2026-10-22); the AddressSanitizer build failed before execution because
Zig rejected Rust's `-Wl,<ASan archive>` linker argument. A subsequent
native-C++-linker ASan run passed: 220,123 executions in 61 seconds, exit 0,
seed 126905715, final cov 426 / features 2129, corpus 578 entries / 36 KiB,
reported RSS 472 MiB. It began from 434 saved corpus files. No crash, invariant
failure or sanitizer error was reported. The recommended dictionary is normal
libFuzzer output, not a failure. RSS includes the harness/ASan and does not
establish production parser memory usage. The configured maximum input length
was 4,096 bytes; the adaptive length limit reached 461 during this campaign,
so this run does not establish coverage of full 4 KiB inputs.
From the repository root, in an authorized developer environment:

```bash
cargo install cargo-fuzz --locked
rustup toolchain install nightly --profile minimal
cargo +nightly fuzz run stream -- -max_total_time=60 -max_len=4096 -rss_limit_mb=512
```

On a developer host with a native C++ driver, prefer overriding only the Rust
linker after the workspace wrapper sets its environment. This retains ASan:

```bash
command -v c++ && \
../../scripts/with-rust env \
  CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=c++ \
  cargo +nightly fuzz run stream -- \
  -max_total_time=60 -max_len=4096 -rss_limit_mb=512
```

The developer verified this native-linker route on 2026-10-23, retaining ASan.
The agent container lacks native C++ tooling, so the run remains developer-run
evidence. No workspace linker script was changed. `fuzz-run.log`, the evolving
corpus and generated artifacts are ignored locally; preserve logs separately
if needed as release evidence. The summarized results above are tracked.

Retain a failing input and convert it into a focused public-API regression test
before fixing a discovered bug. A bounded successful campaign is evidence, not
proof of correctness. Longer campaigns and sanitizer/toolchain metadata should
be recorded separately before release.

## Remaining release evidence

The manifest declares Rust 1.87. The developer ran all 36 library and two
example tests successfully on Rust 1.87.0 on 2026-10-23, including the published
json-volcano dependency. Earlier formatting/lint/docs/package checks used
Rust 1.99.1. The developer also passed FLON's 45 library, 41 binary and
six compiled CLI tests on Rust 1.87.0 against this local parser.

Parser/FLON throughput, allocation and RSS measurements, long-lived streams,
deep paths and large tokens remain developer-run. See the [parser measurement guide](performance.md) and FLON's performance guide.
Publication follows the release guide after validation gaps are resolved.
