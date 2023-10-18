# Getting Started Guide

## TL;DR

``` bash
# setup
cargo install cargo-crev
cargo crev trust --level high https://github.com/dpc/crev-proofs
cargo crev repo fetch all

# verify
cargo crev verify --show-all

# review
cargo crev open $crate_name
cargo crev review $crate_name

# share reviews
# Fork this: https://github.com/crev-dev/crev-proofs/fork
cargo crev id set-url https://github.com/$your_github_username/crev-proofs
cargo crev publish

# get more reviews
cargo crev id query all
cargo crev trust # insert other people's URLs or Ids here

# review just the parts that changed since
cargo crev crate diff $crate_name | less
cargo crev review --diff $previous_version -- $crate_name
```

## Introduction

[Crev](https://github.com/crev-dev/crev) is a system for verifying security and
reliability of dependencies based on collaborative code reviews. Crev users
review source code of packages/libraries/crates, and share their findings with
others. Crev then uses Web of Trust select trusted
[reviews](https://web.crev.dev/rust-reviews/crates/) and judge reputation of
projects' dependencies.

Crev is
[language-independent](https://github.com/crev-dev/crev/#implementations), but
the primary implementation is [`cargo
crev`](https://github.com/crev-dev/cargo-crev/tree/main/cargo-crev) for
Rust/[Cargo](https://doc.rust-lang.org/book/ch01-03-hello-cargo.html) crates.

> This project and documentation is a work in progress. If anything is missing,
> incorrect or stale, let us know. You can [join crev's gitter
> channel](https://gitter.im/dpc/crev) and ask for help or open a GitHub issue.
> Any help in improving this documentation is greatly appreciated.

## Installing

`cargo-crev` is a command-line tool written in Rust. You can [download pre-built
binaries from Releases page](https://github.com/crev-dev/cargo-crev/releases),
or install from source:

``` bash
cargo install cargo-crev
```

In case of compilation issues, [check build instructions](compiling.md) for more
information and troubleshooting.

## Running

In a similar way that `git` is typically used within a context of a local git
repository, `cargo crev` is supposed to be used inside Rust `cargo` project.
Before using `cargo crev` make sure to change current directory to a Rust
project.

## Using Subcommands

``` text
$ cargo crev
cargo-crev 0.18.0

USAGE:
    cargo crev <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    config     Local configuration
    crate      Crate related operations (review, verify...)
    id         Id (own and of other users)
    proof      Find a proof in the proof repo
    repo       Proof Repository
    trust      Add a Trust proof by an Id or a URL
    goto       Shortcut for `crate goto`
    open       Shortcut for `crate open`
    publish    Shortcut for `repo publish`
    review     Shortcut for `crate review`
    update     Shortcut for `repo update`
    verify     Shortcut for `crate verify`
```

A subcommand determines the action taken by `cargo-crev`. Some subcommand of
`cargo-crev` offer an additional level of subcommands. Some of these cascaded
subcommands are provided by shortcuts, such as `verify` for `crate verify`. For
specific help regarding a subcommand use the `-h` flag.

Note: You can abbreviate most `cargo-crev` subcommands. For example: `cargo crev
cr v`.

## Verifying

As a user, your typical goal of using `cargo crev` is verifying that all the
dependencies of the current crate are trustworthy and free of serious bugs and
flaws.

The list of dependencies and their current trustworthiness status is available
through `cargo crev crate verify` command. This is one of the most important and
commonly used sub-command.

Let's take a look:

``` text
$ cargo crev verify --show-all
status reviews issues owner      downloads    loc lpidx geiger flgs crate                version      latest_t
none     0   2  0   0  0  3   197K   5129K    179    84      0 CB__ wasm-bindgen-shared  0.2.70
none     0   1  0   0  0  1   497K   1603K    268    29      0 ____ bytesize             1.0.1
none     0   1  0   0  0  1  3459K   7936K    238    94      1 ____ subtle               1.0.0
none     0   0  0   0  0  3   120K    854K    167    50      0 ____ signature            1.3.0
none     2   2  0   0  0  1     9K      9K      3    15      0 ____ default              0.1.2
```

### Columns

On the right side `crate` and `version` indicate for which crate (in a given
version) values in other columns are calculated and displayed for.

The `status` column displays the verification status for each crate. A `pass`
value indicates that it has been reviewed by a sufficient number of trusted
peers. `none` for lacking reviews, `flagged` or `dangerous` for crates with
problem reports. Verification of dependencies is considered as successful only
if all the values in `status` column contain `pass` value.

If you just started using `crev`, your Rust project probably has more than 100
dependencies, and all of them are not passing the verification. That's the
reason why `crev` was created - your software is implicitly trusting 100 or more
libraries, created by strangers from the Internet, containing code that you've
never looked at. It might seem like an impossible problem to solve, but the goal
of `crev` is to actually make it doable.

`cargo crev verify --help`:

- reviews - Number of reviews for the specific version and for all available
  versions (total)
- issues - Number of issues repored (from trusted sources/all)
- owner
  - In non-recursive mode: Owner counts from crates.io (known/all)
  - In recursive mode:
    - Total number of owners from crates.io
    - Total number of owner groups ignoring subsets
- downloads - Download counts from crates.io for the specific version and all
  versions
- loc - Lines of Rust code
- lpidx - "left-pad" index (ratio of downloads to lines of code)
- geiger - Geiger score: number of `unsafe` lines
- flgs - Flags for specific types of packages
  - CB - Custom Build (runs arbitrary code at build time)
  - UM - Unmaintained crate
- name - Crate name
- version - Crate version
- latest\_t - Latest trusted version

## Fetching reviews from other users

Reviews are stored in public git repositories of crev users. `cargo crev update`
or `cargo crev repo fetch trusted` will automatically update known repositories.
It's also possible to fetch them individually. Let's fetch all the *proofs* from
the author of `crev`:

``` text
> cargo crev repo fetch url https://github.com/dpc/crev-proofs
Fetching https://github.com/dpc/crev-proofs... OK
Found proofs from:
      70 FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE
```

This command does a `git fetch` from a publicly available *proof repository* of
a git user, and stores it in a local cache for future use. A *proof repository*
is just a git repository containing *proofs*.

## Building *trust proofs*

It's possible that some crates have reviews, but the crates aren't trusted. This
happens when you don't trust the reviewers.

For most projects it is not possible to review all dependencies by yourself. You
will have to trust some people. The fact of trusting a crev user is publicly
recorded as a *trust proof*. This allows building a public network of trusted
reviewers.

You can trust a user specifically by their CrevID. This is the most secure
option. To trust `dpc`, run:

``` text
$ cargo crev id trust FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE
```

After you unlock your ID you'll be put into a text editor to create a
*proof*:

``` text
# Trust for FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE https://github.com/dpc/crev-proofs
trust: medium
comment: ""
```

You can edit it to customize the relationship. Editing the proof is modeled
after editing a commit message through `git commit`. As you can see [helpful
documentation](https://github.com/crev-dev/cargo-crev/blob/main/crev-lib/rc/doc/editing-trust.md)
is available in the editor.

## Creating a `CrevID`

You can also be a reviewer, and other people will be able to use your reviews.
You will need a public git repository to serve as your *proof repository*.
Customarily the repository should be called `crev-proofs`.

- GitHub users can just [fork a
  template](https://github.com/crev-dev/crev-proofs/fork) (same [for
  GitLab](https://gitlab.com/crev-dev/crev-proofs)).
- Other users can do it manually. **Note**: `cargo-crev` requires the master
  branch to already exist, so the repository you create has to contain at least
  one existing commit.

Then run `cargo crev id new` like this:

``` text
$ cargo crev id new --url https://github.com/YOUR-USERNAME/crev-proofs
https://github.com/YOUR-USERNAME/crev-proofs cloned to /home/YOUR-USERNAME/.config/crev/proofs/Sp87YXeDKUyh4jImm23bCp1Gr-6eNkMoQogWbftNobQ
CrevID will be protected by a passphrase.
There's no way to recover your CrevID if you forget your passphrase.
Enter new passphrase:
```

The command will ask you to encrypt your identity, and print out some encrypted
data to back up. Please copy that data and store it somewhere reliable.

You can generate and use multiple IDs, but one is generally enough. Check your
current `CrevID` like this:

``` text
$ cargo crev id current
2CxdPgo2cbKpAfaPmEjMXJnXa7pdQGBBeGsgXjBJHzA https://github.com/YOUR-USERNAME/crev-proofs
```

To push your changes (reviews, trust proofs) run:

``` bash
cargo crev publish
```

## Transitive effective trust

When you are done, have saved the proof and closed the editor, you should be
able to query all the ids you trust.

``` text
$ cargo crev id query trusted
FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE medium https://github.com/dpc/crev-proofs
YWfa4SGgcW87fIT88uCkkrsRgIbWiGOOYmBbA1AtnKA low    https://github.com/oherrala/crev-proofs
2CxdPgo2cbKpAfaPmEjMXJnXa7pdQGBBeGsgXjBJHzA high   https://github.com/YOUR-USERNAME/crev-proofs
```

That might be a little surprising. Not only are you trusting
`FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE` which you have just signed the
*trust proof* for, but also some other user.

That's because [user `dpc` already trusted user
`oherrala`](https://github.com/dpc/crev-proofs/blob/2d250e26bed95927a76551c7969cd108ebb1946c/FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE/trust/2019-04.proof.crev#L1).
Trust in `crev` is transitive. If you trust user `b`, and user `b` trusts user
`c`, you're implicitly trusting user `c`. That is what your personal *Web of
Trust* really means in `crev`.

For distrustful people, it seems scary at first, but it should not.

We are trying to achieve the "impossible" here. We're not going to get much done
if we are not reusing work of other people. And we should use any help we can
get.

If it still makes you worry, just be aware that `cargo crev` provides a lot of
ways to configure the effective trust calculation, including control over depth
of the *Web of Trust* and redundancy level required. Also, the effective
transitive trust level of `c` is always lower or equal to the direct trust level
of `b`.

## Reviewing code

Try `cargo crev crate verify` again.

If you are moderately lucky, at least some of the dependencies are now passing
the verification.

But ultimately someone has to do the review, and at least sometimes you will
have to do it yourself.

Scan the output of `cargo crev crate verify` and pick a crate with low lines of
code (`loc`) count. For your first review you want to start small and easy.

At the moment of writing this `cargo crev` provides two methods of reviewing
crate source code:

- for people preferring the command line and text editors like Vim, there's a
  `cargo crev crate goto` command
- for IDE users `cargo crev crate open`

### Reviewing code using `cargo crev crate goto`

If you want to review a crate called `default`, you run:

``` text
$ cargo crev crate goto default
Opening shell in: /home/YOUR-USERNAME/.cargo/registry/src/github.com-1ecc6299db9ec823/default-0.1.2
Use `exit` or Ctrl-D to return to the original project.
Use `review` and `flag` without any arguments to review this crate.
```

As the output explains: `cargo crev crate goto` works by opening a new shell
with current working directory set to a copy of the crate source code stored by
`cargo` itself.

You're now free to use `Vim` or any other commands and text editors to
investigate the content of the crate. `tree -alh` or `ls` are a typical starting
commands, followed by `vi <path_to_rs_file>`.

Also consider using [`cargo tree`](https://crates.io/crates/cargo-tree) which is
part of `cargo` as of `cargo 1.44.0`,
[`cargo-audit`](https://crates.io/crates/cargo-audit) and
[`cargo-outdated`](https://crates.io/crates/cargo-outdated).

Now go ahead and review\! It might be a novel experience, but it is the core of
`crev` - we can not build trust if no one ever actually reviews any code. Try to
be thorough, but at the same time: do not push yourself too much or let the fear
make you not review at all.

When you are done with the actual review, it is time to actually create and sign
the *review proof*.

You either call `cargo crev crate review` (or `cargo crev flag` if results of
your review were negative), or exit the temporary review-shell and use `cargo
crev review <cratename>`.

### Reviewing code using `cargo crev open`

If you are an IDE user you can make `crev` open the crate source code in the IDE
of your choice.

Example. VSCode users can run:

``` text
$ cargo crev open <crate> --cmd "code --wait -n" --cmd-save
```

`--cmd-save` will make `crev` remember the `--cmd` paramter in the future, so it
does not have to be repeated every time. The exact `--cmd` to use for each IDE
can vary, and you can ask for help in figuring it out on the `crev`'s gitter
channel. You can change the command later with `cargo crev config edit`.

### Creating a review

After reviewing the code, create the *review proof*:

``` bash
cargo crev crate review <cratename>
```

Similarly to editing *trust proof*, you have to edit the *review proof* document
when you create a review.

``` text
# Package Review of default 0.1.2
review:
  thoroughness: low
  understanding: medium
  rating: positive
comment: ""


# # Creating Package Review Proof
#
# A Package Review Proof records results of your review of a version/release
# of a software package.
#
# ## Responsibility
#
# It is important that your review is truthful. At very least, make sure
# to adjust the `thoroughness` and `understanding` correctly.
#
# Other users might use information you provide, to judge software quality
# and trustworthiness.
#
# Your Proofs are cryptographically signed and will circulate in the ecosystem.
# While there is no explicit or implicity legal responsibiltity attached to
# using `crev` system, other people will most probably use it to judge you,
# your other work, etc.
#
#
# ## Data fields
#
(...)
```

Again, a helpful comment section documents the basic guidelines of *review
proof*, read it
[here](https://github.com/crev-dev/cargo-crev/blob/main/crev-lib/rc/doc/editing-package-review.md).

The most important part is: just be truthful.

Before you finish and save the *proof*, let us look at [an existing, signed
*review
proof*](https://github.com/dpc/crev-proofs/blob/2d250e26bed95927a76551c7969cd108ebb1946c/FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE/reviews/2018-12-packages-Ua7DxQ.proof.crev#L84)

``` text
-----BEGIN CREV PACKAGE REVIEW-----
version: -1
date: "2018-12-19T22:00:24.644210896-08:00"
from:
  id-type: crev
  id: FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE
  url: "https://github.com/dpc/crev-proofs"
package:
  source: "https://crates.io"
  name: either
  version: 1.5.0
  digest: uBbgCVotv_8z4SEOjremFmvMG4JPhUROC19OLjPPLNE
review:
  thoroughness: medium
  understanding: high
  rating: strong
comment: "Simple `Either` type."
-----BEGIN CREV PACKAGE REVIEW SIGNATURE-----
IBPz20fpI6x3nWJJ1pRsHqGVq3b6yQxyYppIlVPUEZIL3h9AYrV-u7UJMPu5sqCWski91mX8qOE5D3_2bgksDQ
-----END CREV PACKAGE REVIEW-----
```

As you might have already noticed, the document you are editing is not a
complete *review proof*. A lot of details will be filled automatically by `cargo
crev`.

`crev` proofs are Yaml documents, wrapped in GPG-like separators, and signed
using the private key generated during `cargo crev id new`.

Yaml is a popular serialization format. It is easy to read. It also makes the
document format easily extendable in the future.

Time to save the document and exit the editor.

You should now be able to see your proof in the output of `cargo crev repo query
review <cratename>`:

``` text
$ cargo crev repo query review default
version: -1
date: "2019-06-19T23:32:13.683894969-07:00"
from:
  id-type: crev
  id: 2CxdPgo2cbKpAfaPmEjMXJnXa7pdQGBBeGsgXjBJHzA
  url: "https://github.com/YOUR-USERNAME/crev-proofs"
package:
  source: "https://crates.io"
  name: default
  version: 0.1.2
  revision: 583039a6a4233b6aa64dcba7a23f5ae4419a9a72
  digest: YuxzyXhCHZYMi4__Hj_hCzkQyxRLrZjDqL8usLqA4QY
review:
  thoroughness: low
  understanding: medium
  rating: positive
```

Congratulations\!

## Publishing your *proofs*

Every time you create a *proof* `crev` records it in a local copy of your *proof
repository* associated with your current `CrevID`.

You can access this repository using `cargo crev repo git` command.

``` text
$ cargo crev repo git log
commit a308421882822bd2256574b6e966a114dd4bfc6e (HEAD -> master)
Author: You <your_email@example.org>
Date:   Wed Jun 19 23:44:20 2019 -0700

    Add review for default v0.1.2
(...)
```

When you are ready, you can push your recent *proofs* to your public repository
with `cargo crev repo publish`.

Now that your work is public, the only thing left is to help other people find
it. Until someone creates a *trust proof* for your `CrevId` (even with `trust:
none` settings), your *proof repository* is not easily discoverable.

You can ask other people to include your `CrevID` in their *WoT* by publishing a
blog-post, sending a tweet, sending message on [`crev's` gitter
channel](https://gitter.im/dpc/crev) or adding it to the [official bootstrapping
wiki-page list of crev *proof
repositories*](https://github.com/crev-dev/cargo-crev/wiki/List-of-Proof-Repositories)

You can also use these places to find more *proof repositories* of other people.

## Follow-up

This short guide is just meant to get you started.

There's already more functionality implemented in `cargo crev`, and even more
will be continuously added in the future. Notably:

- If you plan to share a `CrevId` between many computers, make sure to try
  `export` and `import` commands.
- Differential reviews are available, where instead of reviewing a whole crate,
  you can review a diff between already trusted and current version (`diff` and
  `review --diff` commands).
- Security and serious flaws can be reported with `review --advisory` and are
  visible in the `issues` output of `verify`.
