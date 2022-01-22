<p align="center">
  <a href="https://github.com/crev-dev/cargo-crev/discussions">
    <img src="https://img.shields.io/badge/commmunity-discussion-blue?style=flat-square" alt="community discussion">
  </a>
  <a href="https://github.com/crev-dev/cargo-crev/actions/workflows/ci.yml">
      <img src="https://github.com/crev-dev/cargo-crev/workflows/ci/badge.svg" alt="Github Actions CI Build Status">
  </a>
  <a href="https://crates.io/crates/cargo-crev">
      <img src="https://img.shields.io/crates/v/cargo-crev.svg?style=flat-square" alt="crates.io">
  </a>
  <br>
  
</p>


<p align="center">
  <img src="https://i.imgflip.com/5b8fqd.jpg" alt="jesus, that's a lot of dependencies" />
  <br/>
  <a href="https://jakelikesonions.com/post/158707858999/the-future-more-of-the-present">image credit</a>
</p>


# cargo-crev

> A cryptographically verifiable **c**ode **rev**iew system for the cargo (Rust)
> package manager.

## Introduction

[Crev](https://github.com/crev-dev/crev/) is a language and ecosystem agnostic,
distributed **c**ode **rev**iew system.

`cargo-crev` is an implementation of Crev as a command line tool integrated with
`cargo`. This tool helps Rust users evaluate the quality and trustworthiness of
their package dependencies.

## Features

`cargo-crev` can already:

- warn you about untrustworthy crates and security vulnerabilities,
- display useful metrics about your dependencies,
- help you identify dependency-bloat,
- allow you to review most suspicious dependencies and publish your findings,
- use reviews produced by other users,
- increase trustworthiness of your own code,
- build a web of trust of other reputable users to help verify the code you use,

and many other things with many more to come.

## Getting started

Static binaries are available from the [releases
page](https://github.com/crev-dev/cargo-crev/releases).

Follow the [`cargo-crev` - Getting Started
Guide](https://github.com/crev-dev/cargo-crev/blob/master/cargo-crev/src/doc/getting_started.md)
(more documentation available on [docs.rs](https://docs.rs/cargo-crev)).

`cargo-crev` is a work in progress, but it should be usable at all times.
Use [discussions](https://github.com/crev-dev/cargo-crev/discussions)
to get help, more information and report feedback. Thank you\!

## Raise awareness

If you're supportive of the cause, we would appreciate helping to raise
awareness of the project. Consider putting the below note in the README of your
Rust
    projects:

    It is recommended to always use [cargo-crev](https://github.com/crev-dev/cargo-crev)
    to verify the trustworthiness of each of your dependencies, including this one.

Thank you\!

## Changelog

Changelog can be found here:
<https://github.com/crev-dev/cargo-crev/blob/master/cargo-crev/CHANGELOG.md>
