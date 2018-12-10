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

Still early, but there's a lot of working code and ironed-out ideas.

The current focus is on a first concrete implementation of Crev:
[`cargo-crev`](https://github.com/dpc/crev/tree/master/cargo-crev) -
a tool for reviewing Rust language crates published on [crates.io](https://crates.io).
It is available in an alpha version.

Consider joining [crev gitter channel](https://gitter.im/dpc/crev). Thank you!

## Overview

Using `crev` you can generate cryptographically signed artifacts reviewing whole
releases, parts of the code, or specifying trust (or mistrust) into reviews of other people.

Eg. *Package Review Proofs* that review a whole package (aka. library, crate, etc.):

```
-----BEGIN CREV PACKAGE REVIEW-----
version: -99999
date: "2018-12-09T23:54:12.681862766-08:00"
from:
  id-type: crev
  id: _xQgkbDAQx3nSV5SMfdEeQBSYiPwSI32wnMxnjExk24=
  url: "https://github.com/dpc/crev-proofs"
package:
  source: "https://crates.io"
  name: either
  version: 1.5.0
  digest: 1E88e0ya8wOX1jYLjl5OAEtl1EVzhWpji86dEQ-V720=
review:
  thoroughness: medium
  understanding: high
  rating: positive
comment: "Simple `Either` type"
-----BEGIN CREV PACKAGE REVIEW SIGNATURE-----
AiS2AKPLuIoStFuX3h9KRln3TGv-TArLExY6P3VaI46CL23_1HbB2Nf1o8MvW-_jl6pzXamiYCnRhXHXEOK5DQ==
-----END CREV PACKAGE REVIEW-----
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

