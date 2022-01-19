# Creating Package Review Proof

A Package Review Proof records results of your review of a version/release of a
software package.

## Responsibility

It is important that your review is truthful. At the very least, make sure to
adjust the `thoroughness` and `understanding` correctly.

Other users might use information you provide, to judge software quality and
trustworthiness.

Your Proofs are cryptographically signed and will circulate in the ecosystem.
While there is no explicit or implicit legal responsibility attached to using
`crev` system, other people will most probably use it to judge you, your other
work, etc.

By creating and publishing proofs, you implicitly agree to other people freely
using them.

## Data fields

- `review` - review of particular version of the crate; all fields set to `none`
  mean that no review took place; whole section can be deleted for the same
  effect;
  - `digest` - recursive digest of the whole project content
  - `thoroughness` - time and effort spent on the review
    - `high` - long, deep, focused review - possibly as a part of a formal
      security review; "hour or more per file"
    - `medium` - a standard, focused code review of a decent depth; "~15 minutes
      per file"
    - `low` - low intensity review: "~2 minutes per file"
    - `none` - no review, or just skimming; "seconds per file"; still useful for
      a trusted or reputable project or when when proof is created to warn about
      problems
  - `understanding`
    - `high` - complete understanding
    - `medium` - good understanding
    - `low` - some parts are unclear
    - `none` - lack of understanding
  - `rating`
    - `strong` - secure and good in all respects, for all applications
    - `positive` - secure and ok to use; possibly minor issues
    - `neutral` - secure but with flaws
    - `negative` - severe flaws and not ok for production usage
    - `dangerous` - unsafe to use; severe flaws and/or possibly malicious
- `advisories` - advisories mark package versions containing an important fix
  (list)
  - `ids` - list of IDs identifying the issue being fixed
  - `range` - versions are potentially affected
    - `all` - all previous versions
    - `major` - all previous versions within the same major release version
    - `minor` - all previous versions within the same minor release version
  - `severity`
    - `high` - critical issue (often with security implications)
    - `medium` - important
    - `low` - low severity
- `issues` - issues report a problem in a release (list)
  - `id` - an ID of an issue
  - `severity` - same as in the `advisories` section
- `alternatives` - potential alternatives, similar or better; elements of the
  list with an empty `name` will be automatically ignored and removed
- `flags` - additional flags
  - `unmaintained` - package is not maintained or abandoned; **NOTE**: this flag
    applies to the whole package, not only current version, like in most other
    data fields
- `comment` - human-readable information about this review (e.g. why it was
  done, how, and `rating` explanation)
- `override` - list of Ids from which to override (ignore) reviews for target
   package

# Other information

More recent proofs overwrite older ones.

## Further reading

See <https://github.com/crev-dev/cargo-crev/wiki/Howto:-Create-Review-Proofs>
wiki page for more information and Frequently Asked Questions, or join
<https://gitter.im/dpc/crev> discussion channel.
