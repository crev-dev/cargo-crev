# `cargo-crev` to `cargo-vet` converter

[Crev](https://lib.rs/cargo-crev) and [Vet](https://lib.rs/cargo-vet) are supply-chain security tools for auditing Rust/Cargo dependencies.

This tool ([`crevette`](https://lib.rs/crevette)) is a helper for `cargo-crev` users that exports Crev reviews as an `audits.toml` file for use with `cargo-vet`.

## Installation

You must have [`cargo-crev` alredy set up](https://github.com/crev-dev/cargo-crev/blob/master/cargo-crev/src/doc/getting_started.md), some [repos added as trusted](https://github.com/crev-dev/cargo-crev/wiki/List-of-Proof-Repositories) and reviews fetched (try `cargo crev repo fetch all`).

It requires the latest stable version of Rust. If your package manager has an outdated version of Rust, switch to [rustup](https://rustup.rs).

```bash
cargo install crevette
```

## Usage

In this initial release, the tool has no configuration. It uses your default `cargo crev` identity and configuration. It exports almost all reviews from all reviewers you (transitively) trust. Running `crevette` will print location of the `audits.toml` file. You may want to review it to ensure you agree with its contents.

To generate and upload the `audits.toml`:

```bash
crevette
cargo crev publish
```

Then on the `cargo vet` side, go to a Rust/Cargo project that you want to verify, and run:

```bash
# cargo vet init (if you haven't already)
cargo vet import 'https://raw.githubusercontent.com/<your github username>/crev-proofs/HEAD/audits.toml'
cargo vet
```

If you host your repositories elsewhere, adjust the HTTPS link accordingly.

Re-run `crevette` to generate an updated version of `audits.toml` whenever you add more Crev reviews.

## Important limitations

The tool estimates the `safe-to-run` and `safe-to-deploy` criteria based on a fuzzy combination of trust, rating, thoroughtness, and understanding attributes of crev code reviews. Currently negative reviews are not mapped to `vet`'s `violation` feature, and thefore do not have any effect!
