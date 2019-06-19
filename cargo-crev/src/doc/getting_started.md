# `cargo-crev` - User Getting Started Guide

## Introduction

The goal of this guide is to introduce user to `crev`, `cargo crev` command,
ideas behind it and typical workflows.

Please remember that `crev` project is still a work in progress,
and this documentation might be wrong or stale. In case of any problems
please don't hestitate to [join crev's gitter channel](https://gitter.im/dpc/crev)
and ask for help or open a github issue.

Any help in improving this documentation is greatly appreciated.

## `crev` vs `cargo-crev`

`crev` is a language and ecosystem agnostic system of preparing cryptografically signed
documents (*proofs*) describing results of code reviews and circulating them between developers.

While `crev` itself is generic and abstract, to be a practical tool it requires integration
with given ecosystem and programming language. `cargo-crev` is an implementation of `crev` for
Rust programming language, tightly integrated with its package manager: `cargo`. The goal
of `cargo-crev` is helping Rust community verify and review all the dependencies published
on http://crates.io and used by Rust devlopers.

`cargo-crev` is a command line tool, similiar in nature to tools like `git`. Integrations
for IDEs and text editors are possible, but not implemented at the moment.

## Installing

`cargo-crev` is written in Rust, and until binaries for different operating systems are
available, the recommended way to install is using `cargo install` command:


```
cargo install cargo-crev
```

If you need help installing Rust compiler & `cargo`, consider using [rustup.rs page](https://rustup.rs/)


## Running

In a similiar way that `git` is typically used within a context of a local git repository,
`cargo crev` is supposed to be used inside Rust `cargo` project. Before using `cargo crev`
`cd <some_rust_project_path>`.

## Using build-in help

When installed `cargo-crev` can be run like this:

```
$ cargo crev
cargo-crev 0.8.0
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
...
```

As you can see by default `cargo crev` displays the built in help. Try it and
scan briefly over `SUBCOMMANDS` section. It should give you some good overview
of available functionality.


## Verifing

The "goal" of using `cargo crev` is verifing that all the dependencies of the current
crate has been reviewed by you, or people the you trust.

The list of dependencies and their current trustworthiness status is available
through `cargo crev verify deps`. This is one of the most important and commonly used sub-command.

Let's take a look:

```
$ cargo crev verify deps
trust reviews     downloads    own. advisr lines  geiger flgs crate                version         latest_t       
pass   1  1   177100    177100 1/1    0/0      7       0      rustc-workspace-hack 1.0.0           =1.0.0         
none   0  0  3531833   4186680 0/2    0/0    194       0      scopeguard           0.3.3                          
none   0  0  5076485   5902626 0/1    0/0   2685       0 CB   kernel32-sys         0.2.2                          
none   0  5   998838   4988096 0/3    0/0   1886     338      smallvec             0.6.9           ↓0.6.7         
none   0  0  1965880   3816784 1/1    0/0   1523       2      ucd-util             0.1.3                          
none   0  0  1350964   2711723 0/1    0/0    898       8      humantime            1.2.0                          
none   0  0   296354   1425866 0/2    0/0    900      10      sha2                 0.8.0                          
none   0  0  1315721   2838420 1/2    0/0   1316     119      failure              0.1.5                          
none   0  0  1390826   3132921 0/1    0/0   1100      29      owning_ref           0.4.0                          
none   0  0  1114710   6900283 0/1    0/0    289      56      num_cpus             1.10.0                         
pass   1  1  6046519   6101572 0/1    0/0     12       0      winapi-build         0.1.1           =0.1.1         
none   0  1  1358256  12401057 1/2    0/0    270       9      lazy_static          1.3.0           ↓1.2.0         
none   0  0   169811    195767 0/1    0/0    368       8      numtoa               0.1.0                          
none   0  0   592730   2222111 0/1    0/0   1365      64      tokio-reactor        0.1.9                          
none   0  0   600326   1866516 1/1    0/0   1789      36 CB   native-tls           0.2.2                          
none   0  0  1202678   1203095 0/1    0/0    183      20      redox_termios        0.1.1                          
(...)
```

The actual output is using color codes to make the output more accessible.

The explanation of meaning of each column, and all the available options are
described in the output of `cargo crev verify deps --help` command.

Right now we will discuss just the main columns.

On the right side `crate` and `version` indicate the which crate version other
columns were calculated for.

A `pass` value in the `trust` column indicates that this crate, in this version
has been reviewed by enough trusted people to consider it trustworthy.

Verification of dependencies is considered as successful only if all the values
in `trust` column contain `pass`.

If you just started using `crev`, your Rust project probably has more than 100
dependencies, and all of them are not passing the verification. That's the reason 
why `crev` was created - your software is implicitily trusting 100 or more libraries,
created by strangers from the Internet, containing code that you've never looked at.

It might seem like an impossible problem to solve, but the goal of `crev` is to actually
make it doable.

## Reviewing
