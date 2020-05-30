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

## Continuous Integration

To streamline development, this repository uses continuous integration (CI) to check that tests pass and that the code is correctly formatted.

These checks can be automatically executed locally using [Git hooks](https://git-scm.com/book/en/v2/Customizing-Git-Git-Hooks). They will be executed before running Git commands (such as commit). Git hooks for this repository can be trivially installed using the [pre-commit](https://pre-commit.com/) Python package:

```shell
pip install pre-commit
pre-commit install -t pre-commit -t pre-push
```

Hooks for this project are defined in `./.pre-commit-config.yaml`. Pre-commit allows a developer to chose which hooks to install.

## Misc

Other than that there's not that much structure yet, and everything is still fluid
and not necessarily properly done.

Seek help on [crev gitter channel](https://gitter.im/dpc/crev) before you start hacking
on the code.

The immediate goal is to get `cargo-crev` binary to be usable.
