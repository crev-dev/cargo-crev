# `cargo-crev` - Cargo Code REView!


`cargo-crev` is a tool helping Rust users review crates they use,
and share it with the community. It works as a recomendation system,
helps identify poor quality, protects against many attack
vectors, and aims at driving the quality of Rust ecosystem even higher,
by encouraging continous peer review culture.

All of this neatly integrated with the `cargo` itself!

## How it works

* Identify your dependencies: list in many useful ways.
* Review crates: judge their safety, quality and document problems.
* Publish verifiable reviews in a public git repository.
* People download your reviews, you download reviews of others.
* Build a web of trust veting whole Rust ecosystem.
* Gain reputation and trust. Maybe even monetize it, by reving code for money.
* Implement it in your company and/or team to stay ahead! 
* Never again get bitten by unreviewed and untrusted code.

## More info

[Crev](https://github.com/dpc/crev/) is is a language and ecosystem agnostic,
social Code REView system.

`cargo-crev` is an implementation/frontend of Crev integrated with `cargo` and
for Rust/crates.io ecosystem.

See it in action:

[![asciicast](https://asciinema.org/a/216695.png)](https://asciinema.org/a/216695?speed=3)

## Getting started

`cargo-crev` is a work in progress, but it should be usable at all times.
Join [crev gitter channel](https://gitter.im/dpc/crev), get help,
report problems and feedback. Thank you!

### Dependencies
`cargo-crev` has a couple of non-Rust dependencies:

#### Unix

```
# openssl
sudo apt-get install openssl libssl-dev

# argonautica build system
sudo apt-get install clang llvm-dev libclang-dev
```

#### Windows

Make sure you have
[LLVM](http://releases.llvm.org/download.html) installed and added to your
path.

### Installing from crates.io

```
cargo install cargo-crev
```

and you're all set.

### Installing from github

If you want to live on the edge, you can install `cargo-crev` directly from github, too:

```
git clone https://github.com/dpc/crev
cd crev
cargo install -f --path cargo-crev
```

Afterwards you can use `cargo crev` command.


`cargo-crev` has a couple op non-Rust dependencies. 

Soon you should be able to to just `cargo install cargo-crev`, but until then,
you need to install `cargo-crev` directly from github directory.

```
git clone https://github.com/dpc/crev
cd crev
cargo install -f --path cargo-crev
```

Afterwards you can use `cargo crev` command.

## Usage

First **create an empty github repository with name: `crev-proofs`**.

```
cd <your-project>
cargo crev new id --github-username <username>          # generate your id
cargo crev fetch url https://github.com/dpc/crev-proofs # fetch proofs from dpc
cargo crev fetch all                                    # fetch proofs from all known ids
cargo crev verify                                       # verify your depedencies
cargo crev query id all                                 # show all known ids
cargo crev query reviews                                # show all reviews
cargo crev query reviews <package>                      # show all reviews of a package
cargo crev trust <id>                                   # trust someone
cargo crev review <crate>                               # review a dependency
cargo crev commit                                       # commit new proofs (reviews, trust)
cargo crev push                                         # push proofs to your public github repository
cargo crev help                                         # see what other things you can do
```

Join [crev gitter channel](https://gitter.im/dpc/crev) to share your ID with us,
and find IDs of other Rustaceans!
