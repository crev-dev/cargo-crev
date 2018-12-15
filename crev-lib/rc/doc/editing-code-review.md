# Creating Code Review Proof

Code Review Proof records results of your file-by-file review of a software project.

## Responsibility

It is important that your review is truthfull. At very least, make sure
to adjust the `thoroughness` and `understanding` correctly.

Other users might use information you provide, to judge software quality
and trustworthiness.

Your Proofs are cryptographically signed and will circulate in the ecosystem.
While there is no explicit or implicity legal responsibiltity attached to
using `crev` system, other people will most probably use it to judge you,
your other work, etc.


## Data fields

* `data` - timestamp of the proof creation
* `from` - information about the entity creating the proof
  * `id` - ID of the entity
  * `url` - URL where entity publishes their Web of Trust
* `review`
  * `revision` - revision-system checksum at the time of the review
  * `digest` - recursive hash of the whole project content at the time of the review
  * `thoroughness` - time and effort spent reviewing this project
    * `high` - "I've spent hours or more carefully reviewing"; more than an hour,
               of a focused review; possibly a part of a formal security review;
               compared with previous releases, etc.
    * `medium` - a standard, focused code review of a decent depth
    * `low` - "I glanced through it"
    * `none` - "I actually haven't looked inside, or it was too brief to count"
  * `understanding`
    * `high` - "I'm one of the authors and/or experts in the area"
    * `medium` - "It's within my competency to judge this project, and there's nothing
                 in it that I can't understand"
    * `low` - "There are parts that are unclear to me, but I understand a decent chunk
               of it"
    * `none` - "I have no idea what is going on here"
  * `rating`
    * `strong`
    * `positive`
    * `neutral`
    * `negative`
    * `dangerous`
* `comment` - Optional, human-readable information about this review
* `files` - list of reviewed files
