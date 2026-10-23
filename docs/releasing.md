# Releasing json-skimmer 1.0.0

The 0.1.0 release is published. Prepare and publish parser 1.0.0 before FLON
1.0.0. Core features and API review are implemented; open validation gates in
ROADMAP.md must be resolved before calling this release ready.

## Validation and local checks

First complete the independent reference/fuzz/MSRV checks in
[validation](validation.md), record performance evidence, and reverify FLON
against the local final parser. The workspace rules require explicit permission
for integration, fuzzing, reference runs, benchmarks and downloads.

From this repository, using the workspace-local toolchain:

```bash
../../scripts/with-rust cargo fmt --check
../../scripts/with-rust cargo test --offline --locked --lib
../../scripts/with-rust cargo test --offline --locked --examples
../../scripts/with-rust cargo test --offline --locked --doc
../../scripts/with-rust cargo clippy --offline --locked --lib --examples -- -D warnings
../../scripts/with-rust cargo doc --offline --locked --no-deps
../../scripts/with-rust cargo package --offline --locked --allow-dirty --list
../../scripts/with-rust cargo package --offline --locked --allow-dirty
```

Review license, README, docs, examples and tests in the package. The separate
fuzz package/build artifacts must be excluded. json-volcano 1.0.0 remains a
registry library-only development dependency; confirm its lock checksum.
`--allow-dirty` permits reviewing prepared files without changing Git state.
The developer reviews and commits the final release.

## Publishing (developer-run)

Use a private publishing credential context, not the workspace-local Cargo home:

```bash
cargo publish --dry-run --locked
cargo publish --locked
```

After upload succeeds, switch FLON from the local version+path dependency to
`json-skimmer = "1.0.0"`, fetch it, refresh the lockfile and verify FLON again
against the actual registry artifact. Package verification alone is not upload.

## Scope and stable contract

Raw speculative events, grammar/UTF-8 validation, resource limits, forward-only
recovery, terminal EOF and explicit offset exhaustion form the core. See
[API stability](api-stability.md). Decoding/path/owned/Serde/async adapters and
replay recovery remain later work. The declared minimum Rust version is 1.87; the developer verified library
and example tests on Rust 1.87.0.
