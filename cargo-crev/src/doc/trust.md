# Trust and Web of Trust

The goal of this document is to help users understand trust in `crev` (and
`cargo-crev`).

## Web of Trust

### Trust proofs

Any identity can generate and sign a *trust proof* to express direct trust in
another identity.

Example.

``` text
-----BEGIN CREV TRUST -----
version: -1
date: "2019-04-28T22:05:05.147481998-07:00"
from:
  id-type: crev
  id: FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE
  url: "https://github.com/dpc/crev-proofs"
ids:
  - id-type: crev
    id: YWfa4SGgcW87fIT88uCkkrsRgIbWiGOOYmBbA1AtnKA
    url: "https://github.com/oherrala/crev-proofs"
trust: low
-----BEGIN CREV TRUST SIGNATURE-----
02BF0i1K0O7uR8T5UHzymqTo65P9R7JDuvfowZuHb3ubW8kd2-Fbl4jSv0n08ZdSU9P_E2YLWvEJrVQDYfjVCg
-----END CREV TRUST-----
```

Identity `FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE` trusts identity
`YWfa4SGgcW87fIT88uCkkrsRgIbWiGOOYmBbA1AtnKA`. Notably, *trust proofs* include
trust level information (`trust` field).

`cargo-crev` builds WoT from from all the available *trust proofs*, and
calculates a personal *trust set* from it.

When calculating the *trust set*, `crev` recursively traverses the graph from
the given root identity. This makes *trust* transitive.

The root identity is typically the current identity of the user, but can be
specified arbitrarily with the `--for-id` argument.

### *effective trust level*

While traversing the graph `crev` keeps track of an *effective trust level* of
each trusted identity. In simple terms: if R is the root identity, and R trust X
with a low *trust level*, and X trusts Y with high *trust level*, R will have a
low *effective trust level* for Y, because *effective trust level* for Y can't
exceed the *effective trust level* in X.

More precisely: *effective trust level* of R for Y is equal to:

- maximum of:
  - direct *trust level* of R for Y (if available), or
  - for any already trusted identity Xi that also trusts Y, the maximum value of
    the lowest of:
    - direct trust level of Xi for Y
    - the *effective trust level* of R for Y

Or in other words: for R to have a given *effective trust* for Y, there has to
exist at least on path from R and Y, where every previous node directly trusts
the next one at the level at least as high.

That's because any *effective trust level* can only be as high as the highest
*effective trust level*

### Depth of the WoT

While traversing the trust graph to calculate the WoT, `cargo-crev` keeps track
of the distance from the root ID. The exact details how far from it it will
reach can be controlled by the following command line options:

``` text
--depth <depth>
--high-cost <high_cost>
--medium-cost <medium_cost>
--low-cost <low_cost>
```

This allows flexible control over transitive trust. For example:

``` text
--high-cost 1 --medium-cost 1 --low-cost 1 --depth 1
```

would effectively make `cargo-crev` use only directly trusted identities.

### Filtering reviews

In addition to control over how the WoT is calculated, it is possible to filter
package reviews used by other criteria.

`--trust` options allows verification of packages only by reviews created by
identities of a given trust level (or higher).

The following options:

``` text
--thoroughness <thoroughness>
--understanding <understanding>
```

control filtering of the reviews by their qualities.

Finally:

``` text
--redundancy <redundancy>          Number of reviews required [default: 1]
```

control how many trusted reviews is required to consider each package as
trustworthy.
