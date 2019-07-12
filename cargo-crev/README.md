<p align="center">
  <a href="https://travis-ci.org/dpc/crev">
      <img src="https://img.shields.io/travis/dpc/crev/master.svg?style=flat-square" alt="Travis CI Build Status">
  </a>
  <a href="https://crates.io/crates/cargo-crev">
      <img src="http://meritbadge.herokuapp.com/cargo-crev?style=flat-square" alt="crates.io">
  </a>
  <a href="https://matrix.to/#/!uBhYhtcoNlyEbzfYAW:matrix.org">
    <img src="https://img.shields.io/matrix/crev:matrix.org.svg?server_fqdn=matrix.org&style=flat-square" alt="crev matrix channel">
  </a>
  <a href="https://gitter.im/dpc/crev">
    <img src="https://img.shields.io/gitter/room/dpc/crev.svg?style=flat-square" alt="crev gitter channel">
  </a>
  <br>
</p>

# `cargo-crev` - Cargo Code REView!

`cargo-crev` is a tool helping Rust users evalute quality and trustworthiness
of dependencies they use.

`cargo-crev` helps analyze data like:

* popularity,
* line-count,
* amount of `unsafe` statements,
* number of owners and their reputation,
* known advisories affecting it

On top of that, it comes with an implementation of distributed
Code REView and recomendation system (crev),
that protects against many attack vectors.

The general goal is driving the quality of Rust ecosystem even higher,
by helping indentify quality crates, and encouraging continous peer review culture.

All of this is neatly integrated with the `cargo` itself.

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
distributed Code REView system.

`cargo-crev` is an implementation/frontend of Crev integrated with `cargo` and targeting
Rust/crates.io ecosystem.



## Getting started

Static binaries are availble on [crev's releases](https://github.com/dpc/crev/releases) page.

Follow the [`cargo-crev` - Getting Started Guide](https://github.com/dpc/crev/blob/master/cargo-crev/src/doc/getting_started.md)
(more documentation available on [docs.rs](https://docs.rs/cargo-crev)).

`cargo-crev` is a work in progress, but it should be usable at all times.
Join [crev gitter channel](https://gitter.im/dpc/crev), get help,
report problems and feedback. Thank you!

## Changelog

Changelog can be found here: https://github.com/dpc/crev/blob/master/cargo-crev/CHANGELOG.md
