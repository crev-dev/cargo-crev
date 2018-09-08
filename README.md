<p align="center">
<!-- 
  <a href="https://travis-ci.org/dpc/crev">
      <img src="https://img.shields.io/travis/dpc/crev/master.svg?style=flat-square" alt="Travis CI Build Status">
  </a>
  <a href="https://crates.io/crates/crev">
      <img src="http://meritbadge.herokuapp.com/crev?style=flat-square" alt="crates.io">
  </a>
-->
  <a href="https://gitter.im/dpc/crev">
      <img src="https://img.shields.io/badge/GITTER-join%20chat-green.svg?style=flat-square" alt="Gitter Chat">
  </a>
  <br>
</p>



# `crev` -  Code REView tool that we desperately need

You're ultimately responsible for vetting your dependencies.

But in a world of NPM/PIP/Cargo/RubyGems - how do you do that? Can
you keep up with ever-changing ecosystem?

`crev` is a real "code review" system as opposed to typical "code-change review" system.

`crev` is scalable, distributed and social.

`crev` records review metadata: who, when, how did the review and 
stores this information in a verifiable way along with the code.

`crev` starts at a file granularity, builds on top of it upwards, allowing
spontaneous coordination and trust building on a personal, project, and global
scale.

`crev` is a tool that we desperately need.

#### Status

Very early, looking for help.

Name, idea, implementation - all are work in progress. I'm slowly
working on it, but have little free time.

If you like the idea, please consider contributing. Nothing here is difficult - it's
all just about writing the necessary code, testing and refining the basic idea.

At very least, please please consider giving your feedback on the original
[forum thread](https://users.rust-lang.org/t/idea-for-scalable-code-review-trust-system)
or [crev gitter channel](https://gitter.im/dpc/crev). Thank you!

## Overview

Using `crev` you can generate Code Review Proofs, e.g.:

```
-----BEGIN CODE REVIEW-----
date: "2018-08-27T22:40:06.639220203-07:00"
from: "IkmxqWrukzjbxK9CM6UgAwMDF9AQdotoRHOIoR+zeNI="
"from-name": Dawid Ciężarkiewicz
project_urls:
  - "https://github.com/dpc/crev"
revision: 2267845bd1e397e9e41c3e87fea21441fc629ce8
"revision-type": git
comment: "I'm the author"
thoroughness: medium
understanding: high
trust: high
files:
  - path: README.md
    digest: 2a092866507c63b00022d233f36a7f3bd9f2b68fdcbdcab77ba3886319a08bdb2a33479dd05bd897d59c17cade18d10794c6e37acd933fd393d129a16ca51092
  - path: src/proof.rs
    digest: 56457bf6df215eb64fff035c28244951c509d77c6e46edfa66105a7a72382051d222bb6a6d66bad415fc325fdd80c50d27c2b076914315cbcb369d3c4f6857fb
  - path: src/main.rs
    digest: 012b46b4d10bdca817ae2638814d7d23c8909b1651fb85742f454fc868fbb82cb7937fb38591da1c01006fa60edc9da20ae4dcdb301c006060a0283cef6be247
  - path: src/index.rs
    digest: 786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce
-----BEGIN CODE REVIEW SIGNATURE-----
mZxQ60ol+MRGLQC863ITf+FjkEAWmD0N4CtJANl5ZRa4c7kFyWCRXljI63UWm23oyNA2ZngZ2S4ndanJIiOMBw==
-----END CODE REVIEW-----
```

and include it in your source code, submit a PR to the original project, a even
some 3rd party code-review gathering repository.

Code review contains, an ID, information about reviewed parts and is cryptographically signed.

While your own reviews are very valuable, `crev` allows reviewing identities of other
people to establish trust.

```
-----BEGIN CODE REVIEW TRUST-----
date: "2018-08-27T22:44:49.855361810-07:00"
from: "IkmxqWrukzjbxK9CM6UgAwMDF9AQdotoRHOIoR+zeNI="
from-name: Dawid Ciężarkiewicz
from-id-type: crev
from_urls:
  - http://github.com/dpc/crev-trust
trusted-ids:
  - "IkmxqWrukzjbxK9CM6UgAwMDF9AQdotoRHOIoR+zeNI="
trust: high
-----BEGIN CODE REVIEW TRUST SIGNATURE-----
zoykKIakR0Ao/Jt53/blblUfQ9+SGFUucEfRFfpaTT71e+0GAT2KagvbAkiKsaPredF3mHh6PwyTQzHkpFBwAg==
-----END CODE REVIEW TRUST-----
```

Similarity to Code Review Proofs, Code Review Trust Proofs are stored along the code: in personal,
per-project, per-community, etc repositories.

`crev` allows collecting both of them, and builds a personalized web of trust. This allows answering 
queries like:

* Which of my dependencies don't have a sufficient (arbitrary) level of code review/trust?
* What were the changes in a project X, since I last reviewed it?
* and more!

These two artifacts are the core of `crev`, and hopefully your can already extrapolate
all the other possibilities.

## Fundamental ideas behind `crev`:

* Not many people can review all of their dependencies, but if every user
  at least skimmed through a couple of them, and share that information with
  others, we would be in much better situation.
* Trust is fundamentally about people and community, not automatic-scans,
  arbitrary metrics, process or bureaucracy. People have to judge both: code
  (code coverage, testing, quality, etc.) and trustworthiness of other
  people (who's reviews do you trust, and how much).
* Code review tool should be as tool-agnostic as possible. No language, package manager,
  etc. limitations. Code is code, and can be reviewed.
* Trust should be spread between many people, so one compromised or malicious
  actor can't abuse the system.
* Code Review should be stored along the source code itself. Just like tests,
  documentation, or even [design decisions](https://github.com/vitiral/artifact).
* Web of Trust is personal and subjective, islands of Trust emerge spontaneously
  and overlap.

## MVP

* `crev` - command line tool, that works a bit like `git`, for personal and per-project use
  * generating, signing ids
  * creating code reviews
  * queries related to code-review coverage
* `libcrev` - a binary for easy building custom tools
* `cargo-crev` - Cargo command that assembles WoT from code and all it's dependencies

## FAQ

> There are certain imperfections...

While I'm open for any ideas for improvements, I won't let the perfect kill the good enough.
Any system of this kind would be a huge improvement over current situation.

> Verification should be on crate/library/project level.

Having code-review on a file level is much more useful:

* Not all projects are small enough to review in one go. Some people might be
  competent/have enough time/personally be interested in a subset of eg. cryptographic
  library. It is still better to collect their per-file review, than not.
* Having file hash/revision allows answering querries like: which files have changed
  since I last reviewed, so I can re-review only them.
* Working on a file-level makes the interface much more natural (similiar to git).

I still plan to support release-integrity reviews, that recursively hash all
files in subdirectories and collapse it to a single hash, that can be signed
and used to check integrity post-release.

> We should review packages, not repositories.

In `crev` you don't review "repositories". You review source code. The proof
of your code review becomes part of the source code itself, just like documentation
is usually a part of the source too. When the maintainers of the project release
their library on crates.io/NPM/etc. the "tarball" contains the review proofs
and the integrity of everything can still be verified.

If you were to review source code in central locations like NPM/crates.io you would
lock users with these centralized services, and render the whole thing much harder
to everybody else.

> What about negative reviews

This must and will be supported for both code review and identity review. User
will be able to express distrust, and thus warn other users.

To prevent malicious actors from silencing negative reviews, Code Review Proofs are
independent of the source code itself. It will be possible to have otherwise code-less
repositories containing only community maintained Code Review Proofs.

> What about other identity systems (PGP, Salty, etc.)

It is easiest for both the end user, and initial implementation to implement
it's own public key based IDs, signing and WoT.

Design is open for supporting PGP, Salty, Keybase, and whatever else in the future.

Note: Systems like that don't carry enough information. Just because you
verified that someones PGP really belong to them, doesn't mean you trust their
code review judgment. But the identity/singing system could be reused.

> I would like to help

[Join crev gitter channel](https://gitter.im/dpc/crev) and let's talk! Or feel free
to start hacking!

> I don't like the name

Well, I like it. It's a bit like "crew", it maps easily to "Code REView", it's short,
and doesn't have a lot of hits on Google yet. I like discoverable names.

Please suggest alternatives, though!
