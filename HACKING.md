# Hacking

## Structure

`Crev` is split into layered crates (lowest layer to highest):

* `crev-common` is a common utility code
* `crev-data` contains core data types, crypto, and serialization code.
* `crev-lib` implements basic concepts (think `libgit2`)
* binary crates - the actual utilities that users will call
    * `crev-bin` - generic tool, currently not meant to be used
    * `cargo-crev` - frontend integrated with Cargo for Rust
* auxiliary tools:
    * `recursive-digest` - library implementing a recursive digest
      over a directory content
    * `rblake2sum` - a binary on top of `recursive-digest`

For core crates, the rule is that any given crate can only depend on the lower layer.

## Misc

Other than that there's not that much structure yet, and everything is still fluid
and not necessarily properly done.

Seek help on [crev gitter channel](https://gitter.im/dpc/crev) before you start hacking
on the code.

The immediate goal is to get `cargo-crev` binary to be usable.
