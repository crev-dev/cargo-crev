# cargo-crev

Cryptographically verifiable code review system for the Rust/Cargo ecosystem.
Implements the Crev protocol: Ed25519-signed code reviews + a distributed Web of Trust.

## Workspace Structure (strict layering, low → high)

1. **crev-common** — Shared utilities: blake2b256 hashing, filesystem helpers, YAML I/O
2. **crev-data** — Core data types: proofs, identities, trust levels, cryptographic signing (ed25519-dalek)
3. **crev-wot** — Web of Trust engine: trust set computation, review aggregation
4. **crev-lib** — Library API (like libgit2 for Crev): local proof store, identity management, git-backed repos
5. **cargo-crev** — CLI binary: Cargo subcommand, dependency analysis, crates.io integration

Each layer may only depend on layers below it. The `crevette` crate is excluded from the workspace.

## Building & Testing

```sh
cargo build                    # build all crates
cargo test                     # run all tests
cargo clippy --workspace       # lint
cargo fmt --all                # format
```

Nix environment: `nix develop` (sets up toolchain, rust-analyzer, git hooks).

## Key Dependencies

- `ed25519-dalek` — cryptographic signing
- `git2` — proof repository storage (git-backed)
- `cargo` (0.91) — Cargo internals for dependency resolution (cargo-crev only)
- `crates_io_api` — crates.io metadata queries
- `serde_yaml` / `serde_cbor` — proof serialization
- `structopt` — CLI argument parsing
- `petgraph` — dependency graph analysis
- `tokei` — lines-of-code statistics
- `geiger` — unsafe code detection (optional feature)

## Code Conventions

- Rust edition 2021, MSRV 1.77
- Triple-licensed: MPL-2.0 OR MIT OR Apache-2.0
- `rustfmt.toml` in each crate (edition = "2021" at root)
- Clippy with `--deny warnings` in CI
- Pre-commit hooks: rustfmt, shellcheck, nixpkgs-fmt, trailing-newline check

## CI

- GitHub Actions: test matrix across stable/beta/nightly, x86_64/i686/ARM/musl, Linux/macOS
- Nix CI: build, test, clippy, doc checks
- Release workflow triggered by `v*` tags; builds cross-platform binaries

## Project Layout

```
cargo-crev/src/main.rs    — CLI entry point + main command dispatch (~44KB)
cargo-crev/src/opts.rs    — CLI option definitions (structopt)
cargo-crev/src/deps/      — Dependency analysis logic
cargo-crev/src/review.rs  — Review creation workflow
crev-data/src/proof/      — Proof types and serialization
crev-wot/src/trust_set.rs — Core WoT algorithm
crev-lib/src/local.rs     — Local proof store management
ci/                        — CI helper scripts
misc/git-hooks/            — Pre-commit hook scripts
design/purpose.md          — Design rationale document
```
