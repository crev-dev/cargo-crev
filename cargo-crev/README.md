# `cargo-crev` - Cargo Code REView!


`cargo-crev` is a tool helping Rust users review crates they use,
and share it with the community.

## How it works

* Review crates: judge their safety, quality and find problems.
* Publish verifiable reviews in a public git repository.
* People download your reviews, you download reviews of others!
* We build a web of trust veting whole Rust ecosystem.
* Noone ever again is bitten by running unreviewed and untrusted code.

## More info

Crev is is a language and ecosystem agnostic, social Code REView system.

`cargo-crev` is an implementation of Crev for `cargo` and Rust.

## Getting started

`cargo-crev` is a work in progress, but it should be usable at all times.
Join [crev gitter channel](https://gitter.im/dpc/crev), get help,
report problems and feedback. Thank you!

### Dependencies

`cargo-crev` has a couple of non-Rust dependencies:

```
# openssl
sudo apt-get install openssl libssl-dev

# argonautica build system
sudo apt-get install clang llvm-dev libclang-dev
```

Soon you should be able to to just `cargo install cargo-crev`, but until then,
you need to install `cargo-crev` directly from github directory.

```
git clone https://github.com/dpc/crev
cd crev
cargo install -f --path cargo-crev
```

Afterwards you can use `cargo crev` command.

### Usage

```
cd <your-project>
cargo crev new id                                       # generate your id
cargo crev fetch url https://github.com/dpc/crev-proofs # fetch proofs from dpc
cargo crev verify                                       # verify your depedencies
cargo crev query id all                                 # show all known ids
cargo crev query reviews <package>                      # show all reviews of a package
cargo crev trust <id>                                   # trust someone
cargo crev fetch trusted                                # fetch proofs from all trusted ids
cargo crev review <crate>                               # review a dependency
cargo crev commit                                       # commit new proofs (reviews, trust)
cargo crev push                                         # push proofs to your public github repository
cargo crev help                                         # see what other things you can do
```

Join [crev gitter channel](https://gitter.im/dpc/crev) to share your ID with us,
and find IDs of other Rustaceans!
