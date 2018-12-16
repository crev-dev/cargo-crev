# Creating Trust Proof

Trust Proof records your trust in abilities and standards of another
entity using `crev` system.

## Responsibility

While `crev` does not directly expose you to any harm from
entities you trust, adding untrustworthy entities into your
Web of Trust, might lower your overal security and/or reputation.

On the other hand, the more trustworthy entites in your Web of Trust,
the broader the reach of it and more data it can find.

Your Proofs are cryptographically signed and will circulate in the ecosystem.
While there is no explicit or implicity legal responsibiltity attached to
using `crev` system, other people will most probably use it to judge you,
your other work, etc.

## Data fields

* `date` - proof timestamp
* `from` - proof author
* `ids` - objects of the trust relationship
* `trust` - trust level; possible values:
  * `high` - "for most practically purposes, I trust this ID as much or more
             than myself" eg. "my dayjob ID", "known and reputatable expert",
             "employee within my team"
  * `medium` - typical, normal level of trust
  * `low` - "I have some reservations about trusting this entity"
  * `none` - "I don't actually trust this entity"; use to overwrite trust from
             a previously issued Trust Proof
  * `distrust` - "I distrust this person and so should you"
* `comment` - human-readable information about this trust relationship,
             (eg. who are these entities, why do you trust them)
