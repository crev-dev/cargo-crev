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

`crev` is a scalable, distributed, social, true-"code review" system. 
(as opposed to more common "code-change review" system).

It's a tool that we desperately need.

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
-----BEGIN CODE REVIEW PROOF-----
date: 2018-08-01 22:43:39-07:00 
from: Dawid Ciężarkiewicz <dpc@dpc.pw>
from-id: RfMbyUrBBK6JNcoF2kaCUnevQU82zRvyMTkW/U/EcWQ=RfMbyUrBBK6JNcoF2kaCUnevQU82zRvyMTkW/U/EcWQ=
project: libcrev
files:
   - digest: 2cff6162e5784b263c6755b6d8b5a7933064956701008060fb47c24b06d630ee
     path: src/index.rs
   - digest: 701008060fb47c24b06d6303cff6162e5784b263c6755b6d8b5a7933064956ee
     path: src/main.rs
   - digest: b263c6755b6d8b5a7933064956701008060fb47c24b06d630ee3cff6162e5784
     path: src/test.rs
revision: bd049182c8a02c11b4bde3c55335c8653bae7e2e
thoroughness: good
understanding: good
trust: some
comment: LGTM
-----BEGIN CODE REVIEW PROOF SIGNATURE-----
5V1c1P5a8dqDVMPhwqnDF39ZrHpaw7jhetEgHyPUkjM8tYvugPzDJ3xyhD9WdJQ4AjwYkN2XdWhnTB3GTRMJuAEd
-----END CODE REVIEW PROOF-----
```

and include it in your source code, submit a PR to the original project, a even
some 3rd party code-review gathering repository.

Code review contains, an ID, information about reviewed parts and is cryptographically signed.

While your own reviews are very valuable, `crev` allows reviewing identities of other
people to establish trust.

```
-----BEGIN CODE REVIEW TRUST PROOF-----
date: 2018-08-11 12:23:31-07:00 
from: Dawid Ciężarkiewicz <dpc@dpc.pw>
from-id: RfMbyUrBBK6JNcoF2kaCUnevQU82zRvyMTkW/U/EcWQ=RfMbyUrBBK6JNcoF2kaCUnevQU82zRvyMTkW/U/EcWQ=
ids:
   - name: Adam Smith <adam@smith.com>
     id: U/EcWQ=RfMbyUrBBK6JNcoF2kaCUnevQU82zRvRfMbyUrBBK6JNcMTkW/yMTkW/U/EcWQoF2kaCUnevQU82zRvy=
   - name: @codeninja
     id: W/yMTkW/U/EcWQoF2kaCUnevQU82zRvyU/EcWQ=RfMbyUrBBK6JNcoF2kaCUnevQU82zRvRfMbyUrBBK6JNcMTk=
trust: good
-----BEGIN CODE REVIEW TRUST SIGNATURE-----
rHpaw7jhetEgHyPUkjM8tYvugPzDJ3xyhD9WdJQ4AjwYkN2XdWhnTB3GTRMJuAEd5V1c1P5a8dqDVMPhwqnDF39Z
-----END CODE REVIEW PROOF-----
```

Similarity to Code Review Proofs, Code Review Trust Proofs are stored along the code: in personal,
per-project, per-community, etc repositories.

`crev` allows collecting both of them, and builds a personalized web of trust. This allows answering 
queries like:

* Which of my dependencies don't have a sufficient (arbitrary) level of code review/trust?
* What were the changes in a project X, since I last reviewed it?

There's more, but these two artifacts are the core of `crev`.

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
