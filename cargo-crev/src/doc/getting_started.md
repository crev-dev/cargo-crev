# Getting Started Guide

## Introduction

The goal of this guide is to introduce you to the [`crev`](https://github.com/crev-dev/crev)
review system, the [`cargo crev`](https://github.com/crev-dev/cargo-crev/tree/master/cargo-crev) command,
ideas behind them and describe the basic workflows that will allow you to start using them.

Please remember that `crev` project is still largely a work in progress,
and this documentation might be incorrect or stale. In case of any problems
please don't hesitate to [join crev's gitter channel](https://gitter.im/dpc/crev)
and ask for help or open a GitHub issue.

Any help in improving this documentation is greatly appreciated.

## `crev` vs `cargo-crev`

`crev` is a general system of preparing cryptographically signed
documents (*proofs*) describing results of code reviews and circulating
them between developers to coordinate a distributed ecosystem of code review.

While `crev` itself is generic and abstract, to be a practical tool it requires integration
with the given ecosystem of each programming language. `cargo-crev` is an implementation of `crev` for
Rust programming language, tightly integrated with its package manager: [`cargo`](https://doc.rust-lang.org/book/ch01-03-hello-cargo.html). The goal of `cargo-crev` is helping Rust community verify and review all the dependencies published
on http://crates.io and used by Rust developers.

`cargo-crev` is a command line tool, similar in nature to tools like `git`. Integration
with IDEs and text editors are possible, but not implemented at the moment.

## Installing

`cargo-crev` is written in Rust, and until binaries for various operating systems are
available, the recommended way to install it is installing from source.

### Using static binaries

Static binaries build by CI pipeline are available on [crev's releases](https://github.com/crev-dev/cargo-crev/releases) GitHub page.

### Building from source

#### Dependencies

Currently `cargo-crev` requires a non-Rust dependency to compile, as OpenSSL
is required for TLS support.

Though OpenSSL is popular and readily available, it's virtually impossible to cover installing
it on all the available operating systems. We list some examples below. They should have matching commands and similar package names in the Unix-like OS of your choice.

In case of problems, don't hesitate to ask for help. 

##### Debian and Ubuntu 

The following should work on Debian and Debian based distributions such as Ubuntu:

```text
sudo apt-get install openssl libssl-dev
```

##### Arch Linux

On Arch and Arch based distributions such as Manjaro make sure the latest OpenSSL is installed:

```text
sudo pacman -Syu openssl
```

##### RedHat

On RedHat and its derivates Fedora and CentOS the following should work:

```text
sudo yum install openssl openssl-devel
```

##### SuSE

On SuSE Linux the following should work:

```text
sudo zypper install openssl libopenssl-devel
```

#### Compiling

To compile and install latest `cargo-crev` release use `cargo`:

```text
cargo install cargo-crev
```

In case you'd like to try latest features from the master branch, try:

```text
cargo install --git https://github.com/crev-dev/cargo-crev/ cargo-crev
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
cargo-crev 0.17.0

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

As you can see, by default `cargo crev` displays the built in help. Try it and
scan briefly over `SUBCOMMANDS` section. It should give you a good overview
of the available functionality.

## Using Subcommands

A subcommand determines the action taken by `cargo-crev`. Some subcommand of `cargo-crev` offer 
an additional level of subcommands. Some of these cascaded subcommands are provided by shortcuts, 
such as `verify` for `crate  verify`. For specific help regarding a subcommand use the `-h` flag.

Note: You can abbreviate most `cargo-crev` subcommands. For example: `cargo crev c v`.

```text
$ cargo crev config -h
cargo-crev-config 0.17.0
Local configuration

USAGE:
    cargo crev config <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    completions    Completions
    dir            Print the dir containing config files
    edit           Edit the config file
    help           Prints this message or the help of the given subcommand(s)
```

## Verifying

As a user, your typical goal of using `cargo crev` is verifying that all the dependencies of the current
crate are trustworthy and free of serious bugs and flaws.

The list of dependencies and their current trustworthiness status is available
through `cargo crev crate verify` command. This is one of the most important and commonly used sub-command.

Let's take a look:

```text
$ cargo crev crate verify --show-all --skip-indirect
digest                                      status reviews     downloads       owner issues    loc geiger flgs lpidx crate                version         latest_t       
dVQxGEaHcfAqvwThsn36nLQiSqOR_qvZYTf0tgMX98s none     0   0  3008623   6631360  0   1  0   0    211      0 ____  7339 adler32              1.0.4           
1nHDTyWmT6V19umoimQWYccblq1AHG9SNDWfqOb8D8U none     0   0  1770811  10749683  1   1  0   0   1295    206 CB__  1419 arrayvec             0.4.12          
svZHC251mVIE-ep8Gwc7VhQ60aHyQ50uc3ZTVaHOq7Y none     0   0    93405    886797  0   2  0   0   1245      0 ____   119 ammonia              2.1.2           
5RqRXABIJly1ZZlqphYktM2CeAOUC8ry8eeHbjiUmqs none     0   0   368513    429505  0   2  0   0   6166     19 ____     3 actix-net            0.2.6           
-                                           local    0   0        ?         ?  ?   ?  0   0      3      0 ____     0 cargo-crev-demo      0.1.0*
```

The actual output is using color to make the data more accessible.

The meaning of each column, and all the available options are
described in the output of `cargo crev crate verify --help` command.

Right now we will discuss just the most important columns.

On the right side `crate` and `version` indicate for which crate (in a given version)
values in other columns are calculated and displayed for.

The `status` column displays the verification status for each crate. A `pass` value
indicates that it has been reviewed by a sufficient number of trusted peers.

Verification of dependencies is considered as successful only if all the values
in `status` column contain `pass` value.

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
> cargo crev repo fetch url https://github.com/dpc/crev-proofs
Fetching https://github.com/dpc/crev-proofs... OK
Found proofs from:
      70 FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE
```

This command does a `git fetch` from a publicly available *proof repository* of a git
user, and stores it in a local cache for future use. A *proof repository* is just a
git repository containing *proofs*.

Go ahead and re-run `cargo crev crate verify`. Chances are you're using crates
that dpc have already reviewed. The `reviews` column will contain values bigger than zero.

## Building *trust proofs*

Right now none of your crates is considered trusted yet, despite the fact that dpc might
have reviewed them already. The reason is: you don't trust this user.

For most projects it is not possible to review all dependencies by yourself. You will have
to trust some people. Let's crate a *trust proof* for dpc. You can always revoke this trust
later if you wish.

```text
$ cargo crev id trust FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE
Error: User config not-initialized. Use `crev id new` to generate CrevID.
```

Oops. That's right. You can't sign a *proof* until you have your own identity.

## Creating a `CrevID`

To create a `CrevID` you'll first need a github repository to serve
as your public *proof repository*. Customarily the repository should be called `crev-proofs`.

* GitHub users can just [fork a template](https://github.com/crev-dev/crev-proofs/fork).
* Other users can do it manually. **Note**: `cargo-crev` requires the master branch to already exist, so the repository you have created
has to contain at least one existing commit.

Then run `cargo crev id new` like this:

```text
$ cargo crev id new --url https://github.com/YOUR-USERNAME/crev-proofs
https://github.com/YOUR-USERNAME/crev-proofs cloned to /home/YOUR-USERNAME/.config/crev/proofs/Sp87YXeDKUyh4jImm23bCp1Gr-6eNkMoQogWbftNobQ
CrevID will be protected by a passphrase.
There's no way to recover your CrevID if you forget your passphrase.
Enter new passphrase:
```


The command will ask you to encrypt your identity, and print out some encrypted data to back up. Please
copy that data and store it somewhere reliable.

You can generate and use multiple IDs, but one is generally enough. Check your current `CrevID` like this:

```text
$ cargo crev id current
2CxdPgo2cbKpAfaPmEjMXJnXa7pdQGBBeGsgXjBJHzA https://github.com/YOUR-USERNAME/crev-proofs
```

Now, back to creating a *trust proof* for `dpc`.


```text
$  cargo crev id trust FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE
Enter passphrase to unlock:
```

After you unlock your ID you'll be put into a text editor to create a *proof*:


```text
# Trust for FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE https://github.com/dpc/crev-proofs
trust: medium
comment: ""


# # Creating Trust Proof
# 
# A Trust Proof records your trust in abilities and standards of another
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
# See https://github.com/crev-dev/cargo-crev/wiki/Howto:-Create-Review-Proofs wiki
# page for more information and Frequently Asked Questions, or join
# https://gitter.im/dpc/crev discussion channel.
```

Editing the proof is modeled after editing a commit message through `git commit`.
As you can see 
[helpful documentation](https://github.com/crev-dev/cargo-crev/blob/master/crev-lib/rc/doc/editing-trust.md) 
is available in the editor. Don't forget to read it at some point.

When creating a *trust proof* you have to decide on the trust level,
and optionally add a comment about the nature of this trust relationship.

## Transitive effective trust

When you are done, have saved the proof and closed the editor, you should be able to query
all the ids you trust.

```text
$ cargo crev id query trusted
FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE medium https://github.com/dpc/crev-proofs
YWfa4SGgcW87fIT88uCkkrsRgIbWiGOOYmBbA1AtnKA low    https://github.com/oherrala/crev-proofs
2CxdPgo2cbKpAfaPmEjMXJnXa7pdQGBBeGsgXjBJHzA high   https://github.com/YOUR-USERNAME/crev-proofs
```

That might be a little surprising. Not only are you trusting `FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE`
which you have just signed the *trust proof* for, but also some other user.

That's because [user `dpc` already trusted user `oherrala`](https://github.com/dpc/crev-proofs/blob/2d250e26bed95927a76551c7969cd108ebb1946c/FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE/trust/2019-04.proof.crev#L1). Trust in `crev` is transitive. If you trust user `b`, and user `b` trusts user `c`, you're implicitly trusting user `c`. That is what your personal *Web of Trust* really means in `crev`.

For distrustful people, it seems scary at first, but it should not.

We are trying to achieve the "impossible" here. We're not going to get much done if we are not reusing work of other people.
And we should use any help we can get.


If it still makes you worry, just be aware that `cargo crev` provides a lot of ways to configure the effective trust calculation, including
control over depth of the *Web of Trust* and redundancy level required. Also, the effective transitive trust level of `c` is always lower
or equal to the direct trust level of `b`.

## Fetching updates

Now that your *Web of Trust* (*WoT*) is built, you can fetch *proofs* from all the new and existing trusted users with:

```text
$ cargo crev repo fetch trusted
Fetching https://github.com/oherrala/crev-proofs... OK
Fetching https://github.com/dpc/crev-proofs... OK
```

You can also consider fetching *proofs* from all the users `crev` is aware of - even ones that
are not part of your *WoT*. Use `cargo crev repo fetch all` for that.

## Reviewing code

Try `cargo crev crate verify` again.

If you are moderately lucky, at least some of the dependencies are now passing the verification.

But ultimately someone has to do the review, and at least sometimes you will have to do it yourself.

Scan the output of `cargo crev crate verify` and pick a crate with low lines of code (`loc`) count. For your first
review you want to start small and easy.

At the moment of writing this `cargo crev` provides two methods of reviewing crate source code:

* for people preferring the command line and text editors like Vim, there's a `cargo crev crate goto` command
* for IDE users `cargo crev crate open`

### Reviewing code using `cargo crev crate goto`

If you want to review a crate called `default`, you run:

```text
$ cargo crev crate goto default
Opening shell in: /home/YOUR-USERNAME/.cargo/registry/src/github.com-1ecc6299db9ec823/default-0.1.2
Use `exit` or Ctrl-D to return to the original project.
Use `review` and `flag` without any arguments to review this crate.
```

As the output explains: `cargo crev crate goto` works by opening a new shell with current working directory
set to a copy of the crate source code stored by `cargo` itself.

You're now free to use `Vim` or any other commands and text editors to investigate the content of the crate.
`tree -alh` or `ls` are a typical starting commands, followed by `vi <path_to_rs_file>`.

Also consider using [`cargo tree`](https://crates.io/crates/cargo-tree) which is part of `cargo` as of `cargo 1.44.0`, 
[`cargo-audit`](https://crates.io/crates/cargo-audit) and 
[`cargo-outdated`](https://crates.io/crates/cargo-outdated).

Now go ahead and review! It might be a novel experience, but it is the core of `crev` - we can not build
trust if no one ever actually reviews any code. Try to be thorough, but at the same time: do not push
yourself too much or let the fear make you not review at all.

When you are done with the actual review, it is time to actually create and sign the *review proof*.

You either call `cargo crev crate review` (or `cargo crev flag` if results of your review were negative), or exit the
temporary review-shell and use `cargo crev review <cratename>`.

### Reviewing code using `cargo crev open`

If you are an IDE user you can make `crev` open the crate source code in the IDE of your choice.

Example. VSCode users can run:

```text
$ cargo crev open <crate> --cmd "code --wait -n" --cmd-save
```

`--cmd-save` will make `crev` remember the `--cmd` paramter in the future, so it does not have to be
repeated every time. The exact `--cmd` to use for each IDE can vary, and you can ask for help in figuring it out
on the `crev`'s gitter channel. 

After reviewing the code use the standard `cargo crev crate review <cratename>` to create the *review proof*.

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

Again, a helpful comment section documents the basic guidelines of *review proof*, read it [here](https://github.com/crev-dev/cargo-crev/blob/master/crev-lib/rc/doc/editing-package-review.md).

The most important part is: just be truthful.

Before you finish and save the *proof*, let us look at [an existing, signed *review proof*](https://github.com/dpc/crev-proofs/blob/2d250e26bed95927a76551c7969cd108ebb1946c/FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE/reviews/2018-12-packages-Ua7DxQ.proof.crev#L84)

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

`crev` proofs are Yaml documents, wrapped in GPG-like separators, and signed using
the private key generated during `cargo crev id new`.

Yaml is a popular serialization format. It is easy to read and easy to parse. It also
makes the document format easily extendable in the future.


Time to save the document and exit the editor.

You should now be able to see your proof in the output of `cargo crev repo query review <cratename>`:


```text
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

Congratulations!

## Publishing your *proofs*

Every time you create a *proof* `crev` records it in a local copy of your *proof repository* associated with
your current `CrevID`.

You can access this repository using `cargo crev git` command.


```text
$ cargo crev repo git log
commit a308421882822bd2256574b6e966a114dd4bfc6e (HEAD -> master)
Author: You <your_email@example.org>
Date:   Wed Jun 19 23:44:20 2019 -0700

    Add review for default v0.1.2
(...)
```

When you are ready, you can push your recent *proofs* to your public repository with `cargo crev repo publish`.

Now that your work is public, the only thing left is to help other people find it. Until someone creates
a *trust proof* for your `CrevId` (even with `trust: none` settings), your *proof repository* is not
easily discoverable.

You can ask other people to include your `CrevID` in their *WoT* by publishing a blog-post, sending a tweet, sending message on
[`crev's` gitter channel](https://gitter.im/dpc/crev) or adding it to the
[official bootstrapping wiki-page list of crev *proof repositories*](https://github.com/crev-dev/cargo-crev/wiki/List-of-Proof-Repositories)

You can also use these places to find more *proof repositories* of other people.


## Follow-up

This short guide is just meant to get you started.

There's already more functionality implemented in `cargo crev`,
and even more will be continuously added in the future. Notably:

* If you plan to share a `CrevId` between many computers, make sure to try `export` and `import` commands.
* Differential reviews are available, where instead of reviewing a whole crate, you can review a diff between already trusted and current version (`diff` and `review --diff` commands).
* Security and serious flaws can be reported with `review --advisory` and are visible in the `issues` output of `verify`.
