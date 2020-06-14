# Creating Trust Proof

A Trust Proof records your trust in abilities and standards of another
entity using `crev` system.

## Responsibility

While `crev` does not directly expose you to any harm from
entities you trust, adding untrustworthy entities into your
Web of Trust, might lower your overall security and/or reputation.

On the other hand, the more trustworthy entities in your Web of Trust,
the broader the reach of it and more data it can find.

Your Proofs are cryptographically signed and will circulate in the ecosystem.
While there is no explicit or implicit legal responsibility attached to
using `crev` system, other people will most probably use it to judge you,
your other work, etc.

By creating and publishing proofs, you implicitly agree to other people freely using them.

## Data fields

* `trust` - trust level; possible values:
  * `high` - "for most practical purposes, I trust this ID as much or more
             than myself" e.g. "my dayjob ID", "known and reputable expert",
             "employee within my team"
  * `medium` - typical, normal level of trust
  * `low` - "I have some reservations about trusting this entity"
  * `none` - "I don't actually trust this entity"; use to revoke trust
             (or distrust) from a previously issued Trust Proof
             and/or just advertise user's existence
  * `distrust` - "I distrust this person and so should you"
* `comment` - human-readable information about this trust relationship,
             (e.g. who are these entities, why do you trust them)

# Other information

More recent proofs overwrite older ones.

## Further reading

See https://github.com/crev-dev/cargo-crev/wiki/Howto:-Create-Review-Proofs wiki
page for more information and Frequently Asked Questions, or join
https://gitter.im/dpc/crev discussion channel.
