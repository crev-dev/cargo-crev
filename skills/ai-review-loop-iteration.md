You are tasked with reviewing a single Rust dependency using the cargo-crev review skill.

If you don't already have access to the `cargo-crev-review` skill, obtain it by running:

```sh
cargo crev ai skill review
```

Save the output as a local skill and use it.

An up-to-date `cargo crev verify` output is already available at `target/crev/cargo-crev-verify.txt`.
Use it instead of running `cargo crev verify` yourself. Do NOT run `cargo crev update` either —
it was already run during preparation.

Then, follow the skill instructions to:

1. Review a single unreviewed dependency (pick one that hasn't been reviewed yet,
   checking `target/crev/reviews/` for already-reviewed crates to avoid duplicates).
2. Write the review report and unsigned proof as described in the skill.
3. Append the signing command to `target/crev/sign-all.sh` (create it if it doesn't exist,
   with `#!/usr/bin/env bash` and `set -euo pipefail` header).

Important:
- Review exactly ONE crate per invocation.
- Before picking a crate, check `target/crev/sign-all.sh` and `target/crev/reviews/` for
  crates that were already reviewed in previous iterations. Do NOT review them again.
- Do NOT sign the proof — only prepare the unsigned proof and append the signing command.
- Do NOT run `cargo crev publish`.
- Your entire output must be a single paragraph containing: the crate name and version,
  the review parameters (rating, thoroughness, understanding), and a brief result summary.
  No other output.
