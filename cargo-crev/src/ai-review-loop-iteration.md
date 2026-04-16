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
3. Validate the proof by running:
   ```sh
   cargo crev review \
     --import-unsigned-from target/crev/reviews/<crate>-<version>.proof.yaml \
     --no-store --no-edit --print-unsigned
   ```
   If validation fails, fix the YAML and re-validate until it passes.
   **Do not proceed to step 4 until validation succeeds.**
4. Add the proof file to the signing command in `target/crev/sign-all.sh`.
   The script must contain a single `cargo crev review` invocation with multiple
   `--import-unsigned-from` arguments — one per reviewed crate. If the file doesn't
   exist yet, create it with a `#!/usr/bin/env bash` shebang and `set -euo pipefail`,
   followed by the `cargo crev review` line. If it already exists, append a new
   `--import-unsigned-from <proof-path>` argument to the existing command.

   Example `target/crev/sign-all.sh`:
   ```sh
   #!/usr/bin/env bash
   set -euo pipefail
   cargo crev review \
     --import-unsigned-from target/crev/reviews/foo-1.2.3.proof.yaml \
     --import-unsigned-from target/crev/reviews/bar-0.4.1.proof.yaml
   ```

Important:
- Review exactly ONE crate per invocation.
- Before picking a crate, check `target/crev/sign-all.sh` and `target/crev/reviews/` for
  crates that were already reviewed in previous iterations. Do NOT review them again.
- Do NOT sign the proof — only prepare the unsigned proof and update the signing command.
- Do NOT run `cargo crev publish`.
- Your entire output must be a single paragraph containing: the crate name and version,
  the review parameters (rating, thoroughness, understanding), and a brief result summary.
  No other output.
