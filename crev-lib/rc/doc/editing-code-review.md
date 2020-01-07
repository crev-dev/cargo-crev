# Creating Code Review Proof

A Code Review Proof records results of your file-by-file review of a software project.

## Responsibility

It is important that your review is truthful. At the very least, make sure
to adjust the `thoroughness` and `understanding` correctly.

Other users might use information you provide, to judge software quality
and trustworthiness.

Your Proofs are cryptographically signed and will circulate in the ecosystem.
While there is no explicit or implicity legal responsibility attached to
using `crev` system, other people will most probably use it to judge you,
your other work, etc.


## Data fields

* `date` - proof timestamp
* `from` - proof author
* `review` - review details
  * `revision` - revision-system checksum at the time of the review
  * `digest` - recursive digest of the whole project content
  * `thoroughness` - time and effort spent on the review
    * `high` - long, deep, focused review - possibly as a part of a formal
               security review; "hour or more per file"
    * `medium` - a standard, focused code review of a decent depth;
                 "~15 minutes per file"
    * `low` - low intensity review: "~2 minutes per file"
    * `none` - no review, or just skimming; "seconds per file";
               still useful for a trusted or reputable project
               or when when proof is created to warn about problems
  * `understanding`
    * `high` - complete understanding
    * `medium` - good understanding
    * `low` - some parts are unclear
    * `none` - lack of understanding
  * `rating`
    * `strong` - secure and good in all respects, for all applications
    * `positive` - secure and ok to use; possibly minor issues
    * `neutral` - secure but with flaws
    * `negative` - severe flaws and not ok for production usage
    * `dangerous` - unsafe to use; severe flaws and/or possibly malicious
* `comment` - human-readable information about this review
              (e.g. why it was done, how, and `rating` explanation)
* `files` - list of reviewed files

## Further reading

See https://github.com/crev-dev/cargo-crev/wiki/Howto:-Create-Review-Proofs wiki
page for more information and Frequently Asked Questions, or join
https://gitter.im/dpc/crev discussion channel.
