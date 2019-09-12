<p align="center">
  <a href="https://travis-ci.org/crev-dev/cargo-crev">
      <img src="https://img.shields.io/travis/crev-dev/cargo-crev/master.svg?style=flat-square" alt="Travis CI Build Status">
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

# cargo-crev

> A cryptographically verifiable **c**ode **rev**iew system for the cargo (Rust) package manager.

## Introduction

[Crev](https://github.com/dpc/crev/) is a language and ecosystem agnostic, distributed **c**ode **rev**iew system.

`cargo-crev` is an implementation of Crev as a command line tool integrated with `cargo`. This tool helps Rust users evaluate the quality and trustworthiness of their package dependencies.

## Features

`cargo-crev` can already:

* warn you about untrustworthy crates and security vulnerabilities,
* display useful metrics about your dependencies,
* help you identify dependency-bloat,
* allow you to review most suspicious dependencies and publish your findings,
* use reviews produced by other users,
* increase trustworthiness of your own code,
* build a web of trust of other reputable users to help verify the code you use,

and many other things with many more to come.

## Getting started

Static binaries are available from the [releases page](https://github.com/crev-dev/cargo-crev/releases).

Follow the [`cargo-crev` - Getting Started Guide](https://github.com/crev-dev/cargo-crev/blob/master/cargo-crev/src/doc/getting_started.md)
(more documentation available on [docs.rs](https://docs.rs/cargo-crev)).

`cargo-crev` is a work in progress, but it should be usable at all times.
Join our [matrix](https://matrix.to/#/!uBhYhtcoNlyEbzfYAW:matrix.org) or [gitter](https://gitter.im/crev-dev/cargo-crev) channel, get help,
report problems and feedback. Thank you!

## Changelog

Changelog can be found here: https://github.com/crev-dev/cargo-crev/blob/master/cargo-crev/CHANGELOG.md
