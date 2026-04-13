You are preparing for a batch of cargo-crev dependency reviews.

Do the following steps in order:

1. Run `cargo crev update` to refresh the web of trust. Report but ignore any failures.
2. Run `cargo crev verify` and save its full output to `target/crev/cargo-crev-verify.txt`
   (create the `target/crev/` directory if needed).
3. If `target/crev/sign-all.sh` exists, check each `--import-unsigned-from` entry in it.
   For each proof file referenced, read the `package` field to get the crate name and version,
   then run `cargo crev repo query review <name> <version>`. If the user has already signed
   a review for that crate+version, remove the corresponding `--import-unsigned-from` line
   from `sign-all.sh`. If after cleanup the file has no `--import-unsigned-from` lines left,
   delete the script.

Your entire output must be a single short paragraph summarizing what you did.
No other output.
