<p align="center">
  <a href="https://travis-ci.org/dpc/crev">
      <img src="https://img.shields.io/travis/dpc/crev/master.svg?style=flat-square" alt="Travis CI Build Status">
  </a>
<!-- 
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

`crev` allows building web of trust and reusing reviews of trusted parites.

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

Using `crev` you can generate cryptographically signed artifacts specifying trust (or mistrust)
into reviewed code or other reviewers.

Eg. `Project Review Proofs` that review a whole project (aka. package, crate, etc.):

```
-----BEGIN PROJECT REVIEW-----
date: "2018-09-23T22:46:21.051417282-07:00"
from:
  id: An9CIxHs1bLYW_VnrYOoy7jdBY105YCvr4AMeNxO_uE=
  url: "https://github.com/dpc/trust"
project:
  id: WgX255RZwk9qIBJtMz1vdmtIgX7ctnBe5hhw_oD93ds=
revision: e64be138f4b8ee0957e0065adc53389ddc856d1e
thoroughness: low
understanding: medium
trust: none
distrust: medium
digest: 48b775f16d7a345ffd0859c02ec66d4de7d7846bd700baf639529651ae4708b3c6d416b536a1e9ea068a81092371f3c133d3dba4a5a0e0d7c180ed4d254f85e2
-----BEGIN PROJECT REVIEW SIGNATURE-----
_KusMrDw8mU-nWDKIOu4DP75pazhAU3edK1YQmWYkGan7AV_qPjHmUhPmuqUpR4ugklxFLXsnDU3iwgEAzKZCQ==
-----END PROJECT REVIEW-----
```

When useful, it is possible to review particular files (`Code Review Proof`).

While your own reviews are very valuable, `crev` allows reviewing identities of other
people to establish trust.

Proofs like that are stored in personal repositories and published (eg. in
a dedicated git repository) for other people to use.

They can be also included in a relevant source code itself through submiting
a PR to the original project, a even some 3rd party code-review gathering repository.

```
-----BEGIN CODE REVIEW TRUST-----
date: "2018-08-27T22:44:49.855361810-07:00"
from:
  id: An9CIxHs1bLYW_VnrYOoy7jdBY105YCvr4AMeNxO_uE=
  url: "https://github.com/dpc/trust"
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
