# Creating Trust Proof

A Trust Proof records your trust in motivations, abilities and standards
of another entity in the `crev` system.

All trust levels (except distrust) are positive. The `none` level is
considered a default between two users that don't know each other.
Anything above is "more than nothing".

## On Distrust

Distrust is the only punitive trust level. The purpose of it is to mark
malicious or otherwise harmful entries and warn other users about them.
It should not be used lightly, and always include comment explaining the
reason for trying to exclude them from WoT of others, possibly including
links to discussion about it.

Before creating distrust proof, it is suggested to try to contact the user
to correct their wrong-doing, and/or get a second opinion about it.

Example reasons to use `distrust` level:

* User possibly knowingly creates positive reviews for malicious/broken/poor
  quality crates.
* User carelessly inflates their `thoroughness` and `understanding` level
  which might mislead others.
* User possibly knowingly holds other offending users in high trust
  and refuses to lower/remove them.
* User creates unwaranted distrust proofs.

Example reasons *NOT* to use distrust level:

* User creates low `thoroughness` and `understanding` reviews. As long as
  self-reported `thoroughness` and `understanding` levels are truthful,
  such reviews are still beneficial to the community and it's up to other
  users to filter them out with `--thoroughness X` and `--understanding X`
  flags if they don't want ot use them.
* Users review criteria don't match my higher quality standards. Again,
  within reason that does not endanger the community, it is a
  reasponsibility of other users to assign lower trust levels to parties
  that either can't be trusted to do a good job of reviewing code or judge
  quality and standards of other users. Lower trust level of the such party,
  and any other party that might transitively introduce them in your WoT
  with higher level. Use `--level X` flag to filter out reviews from parties
  below certain trust level.

## Responsibility

While `crev` does not directly expose you to any harm from entities you trust,
adding untrustworthy entities into your Web of Trust, might lower your overall
security and/or reputation.

On the other hand, the more trustworthy entities in your Web of Trust, the
broader the reach of it and more data it can find.

Your Proofs are cryptographically signed and will circulate in the ecosystem.
While there is no explicit or implicit legal responsibility attached to using
`crev` system, other people will most probably use it to judge you, your other
work, etc.

By creating and publishing proofs, you implicitly agree to other people freely
using them.

## Data fields

- `trust` - trust level; possible values:
  - `high` - "for most practical purposes, I trust this user to do as good or
     better work than myself" e.g. "my dayjob ID", "known and reputable expert",
    "employee within the same team"
  - `medium` - "I trust this user, but not as much as I would trust myself; their
    review standards might be OK, but lower standard than mine"
  - `low` - "I have some reservations about trusting this entity, but I still
    consider their decisions as somewhat valuable"
  - `none` - "I don't know or trust this entity"; use to revoke trust (or
    distrust) from a previously issued Trust Proof and/or just advertise user's
    existence;
  - `distrust` - "I think this user is malicious/harmful";
- `comment` - human-readable information about this trust relationship, (e.g.
  who are these entities, why do you trust them)
- `override` - list of Ids from which to override (ignore) trust for target Id(s)

# Other information

More recent proofs overwrite older ones.

## Further reading

See <https://github.com/crev-dev/cargo-crev/wiki/Howto:-Create-Review-Proofs>
wiki page for more information and Frequently Asked Questions, or join
<https://gitter.im/dpc/crev> discussion channel.
