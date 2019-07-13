# Getting Started Guide

## Introduction

The goal of this guide is to introduce you to the [`crev`](https://github.com/dpc/crev)
review system, the [`cargo crev`](https://github.com/dpc/crev/tree/master/cargo-crev) command,
ideas behind them and describe the basic workflows that will allow you to start using them.

Please remember that `crev` project is still largely a work in progress,
and this documentation might be incorrect or stale. In case of any problems
please don't hesitate to [join crev's gitter channel](https://gitter.im/dpc/crev)
and ask for help or open a github issue.

Any help in improving this documentation is greatly appreciated.

## `crev` vs `cargo-crev`

`crev` is a general system of preparing cryptographically signed
documents (*proofs*) describing results of code reviews and circulating
them between developers to coordinate a distributed ecosystem of code review.

While `crev` itself is generic and abstract, to be a practical tool it requires integration
with the given ecosystem of each programming language. `cargo-crev` is an implementation of `crev` for
Rust programming language, tightly integrated with its package manager: `cargo`. The goal
of `cargo-crev` is helping Rust community verify and review all the dependencies published
on http://crates.io and used by Rust developers.

`cargo-crev` is a command line tool, similar in nature to tools like `git`. Integration
with IDEs and text editors are possible, but not implemented at the moment.

## Installing

`cargo-crev` is written in Rust, and until binaries for various operating systems are
available, the recommended way to install it is installing from source.

### Using static binaries

Static binaries build by CI pipeline are available on [crev's releases](https://github.com/dpc/crev/releases) github page.

### Building from source

#### Dependencies

Regrettably `cargo-crev` requires couple of non-Rust dependencies to compile:

* `argonautica` crate requires LLVM to compile some C/C++ code,
* OpenSSL is required for TLS support.

Though these are popular and readily available, it's virtually impossible to cover installing
them on all the available Operating Systems. In case of problems, don't hesitate to ask for help.

##### Unix

The following should work on Ubuntu:

```text
# openssl
sudo apt-get install openssl libssl-dev

# argonautica build system
sudo apt-get install clang llvm-dev libclang-dev
```

and should have matching command in the Unix-like OS of your choice.

##### Windows

On Windows, make sure you have
[LLVM](http://releases.llvm.org/download.html) installed and added to your
system path.

#### Compiling

To compile and install latest `cargo-crev` release use `cargo`:

```text
cargo install cargo-crev
```

In case you'd like to try latest features from the master branch, try:

```text
cargo install --git https://github.com/dpc/crev/ cargo-crev
```

If you need help installing Rust compiler & `cargo`, consider using [rustup.rs page](https://rustup.rs/)

## Running

In a similar way that `git` is typically used within a context of a local git repository,
`cargo crev` is supposed to be used inside Rust `cargo` project. Before using `cargo crev`
make sure to change current directory to a Rust project.

## Using build-in help

When installed `cargo-crev` can be run like this:

```text
$ cargo crev
cargo-crev 0.7.0
Dawid Ciężarkiewicz <dpc@dpc.pw>
Scalable, social, Code REView system that we desperately need - Rust/cargo frontend

USAGE:
    cargo-crev crev <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    advise      Create advisory urging to upgrade to given package version
    clean       Clean a crate source code (eg. after review)
    diff        Diff between two versions of a package
(...)
```

As you can see, by default `cargo crev` displays the built in help. Try it and
scan briefly over `SUBCOMMANDS` section. It should give you a good overview
of the available functionality.


## Verifying

As a user, your goal of using `cargo crev` is verifying that all the dependencies of the current
crate are trustworthy and free of serious bugs and flaws.

The list of dependencies and their current trustworthiness status is available
through `cargo crev verify` command. This is one of the most important and commonly used sub-command.

Let's take a look:

```text
$ cargo crev verify
status reviews     downloads    own. issues lines  geiger flgs crate                version         latest_t       
none    0  0   354897   1504220 0/5    0/0   2249     504      core-foundation      0.5.1           
none    0  0   530853   1026015 0/1    0/0    429       2      scoped_threadpool    0.1.9           
none    0  0  1045209   2648161 1/1    0/0    403       3      same-file            1.0.4           
none    0  0   395480  11267511 1/3    0/0   9563       0 CB   serde                1.0.90          
(...)
```

The actual output is using color to make the data more accessible.

The of meaning of each column, and all the available options are
described in the output of `cargo crev verify --help` command.

Right now we will discuss just the most important columns.

On the right side `crate` and `version` indicate for which crate (in a given version)
values in other columns are calculated and displayed for.

The `status` column displays the verification status for each crate. A `pass` value
indicates it has been reviewed by enough trusted people to consider it trustworthy.

Verification of dependencies is considered as successful only if all the values
in `trust` column contain `pass` value.

If you just started using `crev`, your Rust project probably has more than 100
dependencies, and all of them are not passing the verification. That's the reason 
why `crev` was created - your software is implicitly trusting 100 or more libraries,
created by strangers from the Internet, containing code that you've never looked at.

It might seem like an impossible problem to solve, but the goal of `crev` is to actually
make it doable.

## Fetching reviews from other users

The easiest way to verify packages is to see if other people did that before.

Let's fetch all the *proofs* from the author of `crev`:

```text
> cargo crev fetch url https://github.com/dpc/crev-proofs
Fetching https://github.com/dpc/crev-proofs... OK
Found proofs from:
      70 FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE
```

This command does a `git fetch` from publicly available *proof repository* of github
user, and stores it in a local cache for future use. A *proof repository* is just a
github repository containing *proofs*.

Go ahead and re-run `cargo crev verify`. Chances are you're using crates
that dpc have already reviewed. The `reviews` column will contain values bigger than zero.

## Building *trust proofs*

Right now none of your crates is considered trusted yet, despite the fact that dpc might
have reviewed them already. The reason is: you don't trust this user.

For most project it is not possible to review all dependencies by yourself. You will have
to trust some people. Let's crate a *trust proof* for dpc. You can always revoke this trust
later if you wish.

```text
$ cargo crev trust FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE
Error: User config not-initialized. Use `crev new id` to generate CrevID.
```

Oops. That's right. You can't sign an *proof* until you have your own identity.

## Creating a `CrevID`

To create a `CrevID` you'll first need an empty github repository to serve
as your public *proof repository*. Go ahead, and create one on github, or whatever
else your typically host your code. Customarily the repository should be called `crev-proofs`.

Note: `cargo-crev` requires the master branch to already exist, so the repository you have created
has to contains at least one existing commit.

Then run `cargo crev new id` like this:

```text
$ cargo crev new id --url https://github.com/{user}/crev-proofs
https://github.com/{user}/crev-proofs cloned to /home/{user}/.config/crev/proofs/Sp87YXeDKUyh4jImm23bCp1Gr-6eNkMoQogWbftNobQ
CrevID will be protected by a passphrase.
There's no way to recover your CrevID if you forget your passphrase.
Enter new passphrase: 
```


The command will ask you to encrypt your identity, and print out some encrypted data to back up. Please
copy that data and store it somewhere reliable.

You can generate and use multiple IDs, but one is generally enough. Check your current `CrevID` like this:

```text
$ cargo crev query id current
2CxdPgo2cbKpAfaPmEjMXJnXa7pdQGBBeGsgXjBJHzA https://github.com/{user}/crev-proofs
```

Now, back to creating *trust proof* for `dpc`.


```text
$  cargo crev trust FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE
Enter passphrase to unlock: 
```

After you unlock your ID you'll be put into a text editor to create a *proof*:


```text
# Trust for FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE https://github.com/dpc/crev-proofs
trust: medium
comment: ""


# # Creating Trust Proof
# 
# Trust Proof records your trust in abilities and standards of another
# entity using `crev` system.
# 
# ## Responsibility
# 
# While `crev` does not directly expose you to any harm from
# entities you trust, adding untrustworthy entities into your
# Web of Trust, might lower your overal security and/or reputation.
# 
# On the other hand, the more trustworthy entites in your Web of Trust,
# the broader the reach of it and more data it can find.
# 
# Your Proofs are cryptographically signed and will circulate in the ecosystem.
# While there is no explicit or implicity legal responsibiltity attached to
# using `crev` system, other people will most probably use it to judge you,
# your other work, etc.
# 
# ## Data fields
# 
# * `date` - proof timestamp
# * `from` - proof author
# * `ids` - objects of the trust relationship
# * `trust` - trust level; possible values:
#   * `high` - "for most practically purposes, I trust this ID as much or more
#              than myself" eg. "my dayjob ID", "known and reputatable expert",
#              "employee within my team"
#   * `medium` - typical, normal level of trust
#   * `low` - "I have some reservations about trusting this entity"
#   * `none` - "I don't actually trust this entity"; use to overwrite trust from
#              a previously issued Trust Proof
#   * `distrust` - "I distrust this person and so should you"
# * `comment` - human-readable information about this trust relationship,
#              (eg. who are these entities, why do you trust them)
# 
# ## Further reading
# 
# See https://github.com/dpc/crev/wiki/Howto:-Create-Review-Proofs wiki
# page for more information and Frequently Asked Questions, or join
# https://gitter.im/dpc/crev discussion channel.
```

Editing the proof is modeled after editing a commit message through `git commit`.
As you can see helpful documentation is available in the editor. Don't forget
to read it at some point.

When creating a *trust proof* you have to decide on the trust level,
and optionally add a comment about the nature of this trust relationship.

## Transitive effective trust

When you are done, have saved the proof and closed the editor, you should be able query
all the ids you trust.

```text
$ cargo crev query id trusted
FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE medium https://github.com/dpc/crev-proofs
YWfa4SGgcW87fIT88uCkkrsRgIbWiGOOYmBbA1AtnKA low    https://github.com/oherrala/crev-proofs
2CxdPgo2cbKpAfaPmEjMXJnXa7pdQGBBeGsgXjBJHzA high   https://github.com/{user}/crev-proofs
```

That might be a little surprising. Not only are you trusting `FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE`
which you have just signed the *trust proof* for, but also some other user.

That's because [user `dpc` already trusted user `oherrala`](https://github.com/dpc/crev-proofs/blob/2d250e26bed95927a76551c7969cd108ebb1946c/FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE/trust/2019-04.proof.crev#L1). Trust in `crev` is transitive. If you trust user `a`, and user `b` trusts user `c`, you're implicitly trusting user `c`. That is what your personal *Web of Trust* really means in `crev`.

For distrustful people, it seem scary at first, but it should not.

We are trying to achieve an "impossible" here. We're not going to get much done if we are not reusing work of other people.
And we should use any help we can get.


If it still makes you worry, just be aware that `cargo crev` provides a lot of ways to configure the effective trust calculation, including
control over depth of the *Web of Trust* and redundancy level required. Also, the effective transitive trust level of `c` is always lower
or equal to the direct trust level of `b`.

## Fetching updates

Now that your *Web of Trust* (*WoT*) is built, you can fetch *proofs* from all the new and existing trusted users with:

```text
$ cargo crev fetch trusted
Fetching https://github.com/oherrala/crev-proofs... OK
Fetching https://github.com/dpc/crev-proofs... OK
```

You can also consider fetching *proofs* from all the users `crev` is aware of - even ones that
are not par of your *WoT*. Use `cargo crev fetch all` for that.



## Reviewing code


Try `cargo crev verify` again.

If you are moderately lucky, at least some of the dependencies are now passing the verification.

But ultimately someone has to do the review, and at least sometimes you will have to do it yourself.

Scan the output of `cargo crev verify` and pick a crate with low `lines` count. For your first
review you want to start small and easy.


At the moment of writting this `cargo crev` provides two methods of reviewing crate source code:

* for people prefering the command line and text editors like Vim, there's a `cargo crev goto` command
* for IDE users `cargo crev open`

### Reviewing code using `cargo crev goto`

If you want to review a crate called `default`, you run:

```text
$ cargo crev goto default
Opening shell in: /home/{user}/.cargo/registry/src/github.com-1ecc6299db9ec823/default-0.1.2
Use `exit` or Ctrl-D to return to the original project.
Use `review` and `flag` without any arguments to review this crate.
```

As the output explains: `cargo crev goto` works by opening a new shell with current working directory
set to a copy of the crate source code stored by `cargo` itself.

You're now free to use `Vim` or any other commands and text editors to investigate the content of the crate.
`tree -alh` or `ls` are a typical starting commands, followed by `vi <path_to_rs_file>`.

Now go ahead and review! It might be a novel experience, but it is the core of `crev` - we can not build
trust if no one ever actually reviews any code. Try to be thorough, but at the same time: do not push
yourself too much or let the fear make you not review at all.

When you are done with the actual review, it is time to actually create and sign the *review proof*.

You either call `cargo crev review` (or `cargo crev flag` if results of your review were negative), or exit the
temporary review-shell and use `cargo crev review <cratename>`.

### Reviewing code using `cargo crev open`

If you are an IDE users you can make `crev` open the crate source code in the IDE of your choice.

Example. VSCode users can run:

```text
$ cargo crev open <crate> --cmd "code --wait -n" --cmd-save
```


`--cmd-save` will make `crev` remember the `--cmd` paramter in the future, so it does not have to be
repeated every time. The exact `--cmd` to use for each IDE can vary, and you can ask for help in figuring it out
on the `crev`'s gitter channel. 

After reviewing the code use the standard `cargo crev review <cratename>` to create the *review proof*.

### Editing *review proof*

Similarly to editing *trust proof*, you have to edit the *review proof* document.


```text
# Package Review of default 0.1.2
review:
  thoroughness: low
  understanding: medium
  rating: positive
comment: ""


# # Creating Package Review Proof
# 
# Package Review Proof records results of your review of a version/release
# of a software package.
# 
# ## Responsibility
# 
# It is important that your review is truthfull. At very least, make sure
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

Again, a helpful comment section documents the basic guidelines of *review proof*.

The most important part is: just be truthful.

Before you finish, and save the *proof*. Let us look at [an existing, signed *review proof*](https://github.com/dpc/crev-proofs/blob/2d250e26bed95927a76551c7969cd108ebb1946c/FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE/reviews/2018-12-packages-Ua7DxQ.proof.crev#L84)

```text
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

As you might have already noticed, the document you are editing is not a complete
*review proof*. A lot of details will be filled automatically by `cargo crev`.

`crev` proofs are Yaml documents, wrapped in GPG-like separatos, and signed using
private key generated during `cargo crev id new`.

Yaml is a popular serialization format. It is easy to read and easy to parse. It also
makes the document format easily extendable in the future.


Time to save the document and exit the editor.

You should now be able to see your proof in the output of `cargo crev query review <cratename>`:


```text
$ cargo crev query review default
version: -1
date: "2019-06-19T23:32:13.683894969-07:00"
from:
  id-type: crev
  id: 2CxdPgo2cbKpAfaPmEjMXJnXa7pdQGBBeGsgXjBJHzA
  url: "https://github.com/{user}/crev-proofs"
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

Congratulations!

## Publishing your *proofs*

Every time you create a *proof* `crev` records it in a local copy of your *proof repository* associated with
your current `CrevID`.

You can access this repository using `cargo crev git` command.


```text
$ cargo crev git log
commit a308421882822bd2256574b6e966a114dd4bfc6e (HEAD -> master)
Author: {user} <{user}@users.noreply.github.com>
Date:   Wed Jun 19 23:44:20 2019 -0700

    Add review for default v0.1.2
(...)
```

When you are ready, you can push your recent *proofs* to your public repository with `cargo crev publish`.

Now that your work is public, the only thing left is to help other people find it. Until someone creates
a *trust proof* for your `CrevId` (even with `trust: none` settings), your *proof repository* is not
easily discoverable.

You can ask other people to include them in their *WoT* by publishing a blog-post, sending a tweet, sending message on
[`crev's` gitter channel](https://gitter.im/dpc/crev) or adding it to the
[official bootstraping wiki-page list of crev *proof repositories*](https://github.com/dpc/crev/wiki/List-of-Proof-Repositories)

You can also use these places to find more *proof repositories* of other people.


## Follow-up

This short guide is just meant to get you started.

There's already more functionality implemented in `cargo crev`,
and even more will be continuously added in the future. Notably:

* If you plan to share a `CrevId` between many computers, make sure to try `export` and `import` commands.
* Differential reviews are available, where instead of reviewing a whole crate, you can review a diff between already trusted and current version (`diff` and `review --diff` commands).
* Security and serious flaws can be reported with `advise` and are visible in the `advisr` output of `verify`. 
