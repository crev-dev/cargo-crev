

### Structure

`Crev` is split into layered crates (lowest layer to highest):

* `crev-common` is a common utility code
* `crev-data` contains core data types, crypto, and serialization code.
* `crev-lib` implements basic concepts (think `libgit2`)
* binary crates - the actual utilities that users will call


The rule is that any given crate can only depend on the lower layer.

### Misc.

Other than that there's not that much structure yet, and everything is still fluid
and not neccessarily properly done.

Seek help on [crev gitter channel](https://gitter.im/dpc/crev) before you start hacking
on the code.

The imediate goal is to get `cargo` binary to be usable. See
[cargo-trust: Concept](https://github.com/dpc/crev/wiki/cargo-trust:-Concept)
for more information about the vision.



