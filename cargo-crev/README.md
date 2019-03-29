<p align="center">
  <a href="https://travis-ci.org/dpc/crev">
      <img src="https://img.shields.io/travis/dpc/cargo-crev/master.svg?style=flat-square" alt="Travis CI Build Status">
  </a>
  <a href="https://crates.io/crates/cargo-crev">
      <img src="http://meritbadge.herokuapp.com/cargo-crev?style=flat-square" alt="crates.io">
  </a>
  <a href="https://gitter.im/dpc/crev">
      <img src="https://img.shields.io/badge/GITTER-join%20chat-green.svg?style=flat-square" alt="Gitter Chat">
  </a>
  <br>
</p>

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

[Crev](https://github.com/dpc/crev/) is a language and ecosystem agnostic,
social Code REView system.

`cargo-crev` is an implementation/frontend of Crev integrated with `cargo` and
for Rust/crates.io ecosystem.

See it in action:

[![asciicast](https://asciinema.org/a/216695.png)](https://asciinema.org/a/216695?speed=3)

## Changelog

Changelog can be found here: https://github.com/dpc/crev/blob/master/cargo-crev/CHANGELOG.md

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



If you wish to use latest release:

```
cargo install cargo-crev
```

### Installing from github

We try to release often, but new features are added at fast pace. If
you want to try the git version:

```
cargo install --git https://github.com/dpc/crev/ cargo-crev
```

## Usage

First **create an empty github repository with name: `crev-proofs`**.

```
cd <your-project>
cargo crev new id --github-username <username>          # generate your id
cargo crev fetch url https://github.com/dpc/crev-proofs # fetch proofs from dpc
cargo crev fetch all                                    # fetch proofs from all known ids
cargo crev verify                                       # verify your depedencies
cargo crev query id all                                 # show all known ids
cargo crev query review                                 # show all reviews
cargo crev query review <package>                       # show all reviews of a package
cargo crev trust <id>                                   # trust someone

# for Vim/CLI-heavy users
cargo crev goto <crate>                                 # jump/cd to crate to review it

# for IDE users
cargo crev open safemem --cmd "code --wait -n" --cmd-save # open crate in VSCode and use VSCode by default in the future

cargo crev review                                       # review a crate (after goto)
cargo crev review <crate>                               # review a dependency
cargo crev review --independent <crate> <version>       # review a crate that is not a dependency
cargo crev commit                                       # commit new proofs (reviews, trust)
cargo crev push                                         # push proofs to your public github repository
cargo crev help                                         # see what other things you can do
```

Join [crev gitter channel](https://gitter.im/dpc/crev) to share your ID with us,
and find IDs of other Rustaceans!
