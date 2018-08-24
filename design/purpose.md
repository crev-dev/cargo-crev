# REQ-purpose

The main goal of this project is to solve trust problems
of downstream users of ecosystems like NPM/Cargo/Pip etc.
and potentially any organization utilizing source code written
by multiple people.

No matter how strict the security of such ecosystems are,
any downstream users stay vulnerable to:

* poor quality of upstream libraries
* maliciousness of the authors of upstream libraries
* compromised accounts

and while "vetting your dependencies" and upgrading conservatively
is responsibility of the downstream user, in practice it's unrealistic,
because it does not scale.

This is solved by:

* Making a cryptographically verifiable code review information become a part
  of source code in a way similar to how documentation is a part of source code
  in any modern code-bases. (Review Proofs)
* Making personal, technical trust information explicit and cryptographically verifiable
  in a similar fashion.
* Establishing common set of formats and artifacts to allow
  exchanging such artifacts of code review and personal trust.
* Building tools helping downstream users judge, verify and enforce trust 
  and safety requirements based on the above.
