<p align="center">
  <a href="https://travis-ci.org/dpc/crev">
      <img src="https://img.shields.io/travis/dpc/crev/master.svg?style=flat-square" alt="Travis CI Build Status">
  </a>
  <a href="https://gitter.im/dpc/crev">
      <img src="https://img.shields.io/badge/GITTER-join%20chat-green.svg?style=flat-square" alt="Gitter Chat">
  </a>
  <br>
</p>



# `crev` -  Code REView tool that we desperately need

You're ultimately responsible for vetting your dependencies.

But in a world of NPM/PIP/Cargo/RubyGems - how do you do that? Can
you keep up with ever-changing ecosystem?

`crev` is an actual *code review* system as opposed to typical *code-change review* system.

`crev` is scalable, distributed and social.

`crev` records review metadata: who, when, how did the review.

`crev` allows building web of trust and reusing reviews of trusted parites.

`crev` is a tool that we desperately need.

#### Status

Very early, looking for help.

The current focus is on a first conret implementation of Crev:
[`cargo-crev`](https://github.com/dpc/crev/tree/master/cargo-crev) -
a tool for reviewing Rust language crates published on [crates.io](https://crates.io).

Consider joining [crev gitter channel](https://gitter.im/dpc/crev). Thank you!

## Overview

Using `crev` you can generate cryptographically signed artifacts reviewing whole
releases, parts of the code, or specifying trust (or mistrust) into reviews of other people.

Eg. *Project Review Proofs* that review a whole project (aka. package, crate, etc.):

```
-----BEGIN CREV PROJECT REVIEW-----
date: "2018-12-08T20:33:22.144618385-08:00"
from:
  id: SnfdW4LwLh7yHBRNvyKGa4je0bzfeEo4_H4Zs7mgDuc=
  url: "https://github.com/test/crev-db"
project:
  source: "https://crates.io"
  name: toml
  version: 0.4.9
  digest: r0ex2BqsdMk5eK8Zo7dn1lJiC5hv9YXZ3otT7zoVnCc=
review:
  thoroughness: low
  understanding: medium
  trust: medium
comment: Just testing
-----BEGIN CREV PROJECT REVIEW SIGNATURE-----
r4kLN4tmAhDac3f5GdEEuA0ghq23tpvspX_TEy1CVA3OH5szA2BFtG8Uzl_lQiUr_ZYoHMj8LKJsjzaVsus7Dw==
-----END CREV PROJECT REVIEW-----
```

When useful, it is possible to review particular files (*Code Review Proof*).

While your own reviews are very valuable, `crev` allows reviewing identities of other
people to establish trust.

*Proofs* are stored and published in personal repositories for other people to use.

They can be also included in a relevant source code itself through submiting
a PR to the original project.

`crev` collects *Proofs* from different sources, and builds a personalized web of trust.
This allows answering queries like:

* Which of my dependencies don't have a sufficient (arbitrary) level of code review/trust?
* What were the changes in a project X, since I last reviewed it?
* and more!

## Fundamental ideas behind `crev`:

* Not many people can review all their dependencies, but if every user
  at least skimmed through a couple of them, and shared that information with
  others, we would be in much better situation.
* Trust is fundamentally about people and community, not automatic-scans,
  arbitrary metrics, process or bureaucracy. People have to judge both: code
  (code coverage, testing, quality, etc.) and trustworthiness of other
  people (whos reviews do you trust, and how much).
* Code review tool should be language and ecosystem agnostic. Code is code, and should be reviewed.
* Trust should be spread between many people, so one compromised or malicious
  actor can't abuse the system.
* Code Review should be stored along the source code itself. Just like tests,
  documentation, or even [design decisions](https://github.com/vitiral/artifact).
* Web of Trust is personal and subjective: islands of Trust emerge spontaneously
  and overlap.

## Links

* [Crev FAQ](https://github.com/dpc/crev/wiki/FAQ)
* ["cargo-trust: Concept"](https://github.com/dpc/crev/wiki/cargo-trust:-Concept)

