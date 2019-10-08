# Cargo specific features

`crev` is a language and ecosystem agnostic system for reviewing code.
While being quite generic it does not forbit or prevent integrating
with particular features and data available in each ecosystem. Quite the
opposite - part of the vision of `crev` is to build well integrated
ecosystem-specific tools. `cargo-crev` is exactly such a tool for
Rust language and `cargo` package manager.

For this reason `cargo-crev` implements multiple features to help
`cargo` users.

## Known Owners

While in a perfect world everyone would just review the code they
are using and/or rely on other reputable reviewers,
this will be a difficult target until a critical mass of adoption is
reached.

To address this problem, `cargo-crev` allows reasoning about trustworthinnes
of crates by the reputation of their autors.

Every crev identity can create and maintain a "known owners" list. Use
`cargo crev config edit known` command to edit it. Each line is crates.io
username or group name that will be considered somewhat trustwothy.

During dependency verification a `--skip-known-owners` argument can be used
to skip crates that have at least one known owner.

It's important to consider the security implications. crates.io or the personal
accounts of reputable crate authors could get compromised. And just because
the crate owner is on a list of authors does not mean other co-authors
are neccessarily trustworthy.

So this feature is definitely a compromise. But it is very useful for
filtering out dependencies that are most probably OK, and can be reviewed
after code from less reputable sources is reviewed first.


## Download counters

`cargo crev crate verify` will display download counts for both specific crate version
and total crate downloads, as a quick estimate of crate popularity. Crates and versions
with particularily low download count at higher risk of introducing serious bugs
or malicious code.

## Geiger count

[`geiger`](https://crates.io/crates/geiger) is a binary and a library calculating
number of `unsafe` lines of code. `cargo-crev` uses it to display the *geiger count*
for each dependency. `unsafe` code can introduce memory safety issues, and non-zero
geiger count is a good reason to prioritze reviewing the code.

## Lines of code

`cargo-crev` uses [`tokei`](https://crates.io/crates/tokei) to calculate the total
number of Rust code each dependency introduces. Small crates are a good candidate
for imediate review (because it will be quick). Bigger ones can often be replaced
with smaller alternatives.

