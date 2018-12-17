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

`crev` is an actual *code review* system as opposed to typicaly practiced *code-change review* system.

`crev` is scalable, distributed and social. Users publish and circulate results of their reviews: potentially warning about problems, malicious code, or just encuraging high quality by peer review.

`crev` allows building a personal web of trust in people and code.

`crev` [is a][f] [tool][e] [we][d] [desperately][c] [need][b] [yesterday][a]. It protects against compromised dev accounts, intentional malicious code, typesquating, compromised package registries, or just plain poor quality.

[a]: https://www.csoonline.com/article/3214624/security/malicious-code-in-the-node-js-npm-registry-shakes-open-source-trust-model.html

[b]: https://thenewstack.io/npm-attackers-sneak-a-backdoor-into-node-js-deployments-through-dependencies/

[c]: https://news.ycombinator.com/item?id=17513709

[c]: https://www.theregister.co.uk/2018/11/26/npm_repo_bitcoin_stealer/

[d]: https://www.zdnet.com/article/twelve-malicious-python-libraries-found-and-removed-from-pypi/

[e]: https://www.itnews.com.au/news/rubygems-in-recovery-mode-after-site-hack-330819

[f]: https://users.rust-lang.org/t/security-advisory-for-crates-io-2017-09-19/12960

## Vision

We would like Crev to become a general, language and ecosystem agnostic 
system for estabilishing trust in Open Source code. We would like to have
frontends integrated with all major Open Source package managers and ecosystems.

Consider joining [crev gitter channel](https://gitter.im/dpc/crev). Thank you!

## Status

Still early, but there's a team already, a lot of working code and ironed-out ideas.

The current focus is on a first concrete implementation of Crev:
[`cargo-crev`](https://github.com/dpc/crev/tree/master/cargo-crev) -
a tool for reviewing Rust language crates published on [crates.io](https://crates.io).
It is available in an alpha version.

See it in action:

[![asciicast](https://asciinema.org/a/216695.png)](https://asciinema.org/a/216695)


## Overview

Using `crev` you can generate cryptographically signed artifacts (*Proofs*). Proofs can share details of code reviews (whole releases, parts of the code), or specifying trust (or mistrust) into reviews of other people.

Example of *Package Review Proofs* that review a whole package (aka. library, crate, etc.):

```
-----BEGIN CREV PACKAGE REVIEW-----
version: -1
date: "2018-12-16T00:09:27.905713993-08:00"
from:
  id-type: crev
  id: 8iUv_SPgsAQ4paabLfs1D9tIptMnuSRZ344_M-6m9RE
  url: "https://github.com/dpc/crev-proofs"
package:
  source: "https://crates.io"
  name: default
  version: 0.1.2
  digest: RtL75KvBdj_Zk42wp2vzNChkT1RDUdLxbWovRvEm1yA
review:
  thoroughness: high
  understanding: high
  rating: positive
comment: "I'm the author, and this crate is trivial"
-----BEGIN CREV PACKAGE REVIEW SIGNATURE-----
QpigffpvOnK7KNdDzQSNRt8bkOFYP_LOLE-vOZ2lu6Je5jvF3t4VZddZDDnPhxaY9zEQurozqTiYAHX8nXz5CQ
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

