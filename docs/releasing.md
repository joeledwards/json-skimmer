# Releasing json-skimmer 0.1.0

Publish this initial pre-1.0 parser before FLON 0.1.0. The raw streaming core is
implemented; API stabilization and the unchecked 1.0 roadmap items remain work
for a later release. json-volcano 1.0.0 is a crates.io development dependency
with default features disabled; no sibling generator patch is needed.

## Local checks

From the json-skimmer repository, using the workspace-local toolchain:

```bash
../../scripts/with-rust cargo fmt --check
../../scripts/with-rust cargo test --offline --locked --lib
../../scripts/with-rust cargo test --offline --locked --doc
../../scripts/with-rust cargo clippy --offline --locked --lib -- -D warnings
../../scripts/with-rust cargo doc --offline --locked --no-deps
../../scripts/with-rust cargo package --offline --locked --allow-dirty --list
../../scripts/with-rust cargo package --offline --locked --allow-dirty
```

`--allow-dirty` permits review of prepared working-tree files without an agent
changing Git state. Review and commit the intended release before publishing.
Inspect the package for LICENSE, README, docs and tests. Check the lockfile's
json-volcano entry has a registry source and checksum. If the dependency is not
cached, developer-authorized `cargo fetch` is needed before offline checks.

## Publishing (developer-run)

Use a private publishing credential context, not the workspace-local Cargo home:

```bash
cargo publish --dry-run --locked
cargo publish --locked
```

After publication, change FLON's parser dependency to the published 0.1.0,
refresh its lockfile, and follow FLON's release guide. A clean local package
check is not proof of successful registry upload or name ownership.

## Pre-1.0 limitations

No MSRV is established. Fragment coalescing, offset-overflow policy, independent
reference-parser comparisons, fuzzing and the remaining 1.0 grammar/API/release
checks are not yet complete. Existing raw fragments remain speculative until
CandidateEnd; scan mode discovers container roots with forward-only recovery.
See ROADMAP.md and docs/test-coverage.md for the full contract and gaps.
