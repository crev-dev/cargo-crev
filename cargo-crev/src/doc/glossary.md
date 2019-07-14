# Glossary

The following is glossary of terms commonly used in `crev` and `cargo-crev`


### *Advisory*

An optional part of a *Review*, announcing a significant problem
fixed in that package version, advising other users to upgrade.

Notably *Advisories* implicitly denote existance of such issue
in all previous versions in a certain *VersionRange*.

### *CrevId*

Default (and currently the only supported) identity type in `crev`.
A self-generated identity used to sign *proofs*.


Under the hood it's just
a keypair, with the public key used as a public ID, and the secret key
stored locally and encrypted with a passphrase.
### *Issue*

An optional part of a *Review*, announcing a significant problem present
in a given package version.

Very similiar to *Advisory*. It's generally better to prefer *Advisories*,
except for cases in which the problem does not have a solution available yet.

### *Level*

Level is commonly used in `crev` to qualify things. It can typically have one of 4 values:

* high
* medium
* low
* none

### *Proof*

A YAML document describing attesting a certain fact. Currently supported
*proof* types are:

* *Trust Proof* - attesting that the author of the proof considers another
  identity as trustworthy
* *Package Review Proof* - describing the results of code review of a specific
  package/library

More proof types can be introduced in the future.

### *Proof Repository*

A `git` repository used to store *Proofs*. A *Proof Repository* can be local,
or public, just like `git` repositories are.

### *Review*

Short for *Package Review Proof*. A document describing results of source code
review of a particular package.

### *Trust Set*

A result of traversing a *Web of Trust* from from a given root identity
to calculate a set of all the other identities that are directly or
transitiviely consider trustworthy.

See [Trust documentation module](../trust/index.html) for introduction.

### *Web of Trust* (*WoT*)

A graph composed of identities as nodes, and trust relations between them as edges.

Build on the fly from all the available *Trust Proofs* and used to calculate
a *Trust Set* for a given identity.

See [Trust documentation module](../trust/index.html) for introduction.


### *Version Range*

In `crev`, a method of simplified version range specification.

Can take the following values:

* `all` - all versions
* `major` - major release (eg. 2.x.y)
* `minor` - major release (eg. 2.1.x)

An *Advisory* reported in a *Review* of version 1.2.3, would
affect all versions between:

* `0.0.0` and `1.2.3` if reported with `all` *Version Range* value,
* `1.0.0` and `1.2.3` if reported with `major` *Version Range* value,
* `1.2.0` and `1.2.3` if reported with `minor` *Version Range* value
