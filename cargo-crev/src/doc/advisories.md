# Reporting Advisories (and Issues)

`crev` (and so `cargo-crev`) comes with support for issue/advisory reporting.

Both issues and advisories are an optional section of a *package review proof*.
For this reason they are always associated with a specific crate version, which
makes them work in a bit peculiar way

## Issues

`cargo crev report` can be used to report an issue in a given release of a
crate.

Each issue is marked with an ID. It can be any string. Identifiers like
`CVE-xxxx-xxxx`, `RUSTSEC-xxxx-xxxx` are recommended, but when not available an
URL can be used instead.

Issues associated with a crate version, are only stating that this particular
release is affected. The do not imply that this is neccesarily the first or only
version being affected.

Also, issues are treated as an open from the first version reported. `crev` will
consider all the later versions to be affected as well, until a corresponding
*advisory* is found with a matching `id`.

**It is generaly better to report *advisories* instead of issues**. Issues are
most useful when the fixed release is not yet available, so it's impossible to
create an advisory associated with a version that does not yet exist.

## Advisories

`cargo crev advise` can be used to create *package review proof* including an
advisory.

Advisory should be reported on the first version fixing a problem for a given
range of previously affected versions.

For simplicity a single `range` field is used to specify the range of affected
versions.

For example: If a problem affects all releases from 1.4.0 and the fix was
released in the version 1.4.5, a `range: minor` since the whole minor release
was affected (all versions matching 1.4.x, before release containing the
advisory).

This simplifies specifing the range, but is not always precise. Had the issue
been first introduced in version 1.4.1, the version 1.4.0 would be incorrectly
affected as well. This is however rare and overshooting is not a problem.

In some cases the same advisory might need to be created for multiple versions.
I.e. when the patched versions was provided for multiple affected major
versions.
