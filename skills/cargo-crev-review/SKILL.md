---
name: cargo-crev-review
description: Review Rust dependencies and create/publish cargo-crev package review proofs for the user. Use when the user asks you to review one of their Rust dependencies, audit a crate, or produce a crev proof.
---

# cargo-crev: reviewing Rust dependencies as an agent

This skill tells you how to review a Rust dependency on the user's behalf
and help them produce a signed `cargo-crev` package review proof that they can publish
to their proof repository.

## When to use this skill

Use this skill when the user asks you to:

- review one of their Rust dependencies (or a transitive dep),
- audit a specific crate version,
- produce a `cargo crev` review/proof for a crate,
- help them catch up on crates that still need reviewing.

Do *not* use this skill for unrelated security audits, non-Rust code, or
trust proofs for other crev ids — those are separate workflows.

## Prerequisites (check these first)

1. The user has a crev id configured:
   ```sh
   cargo crev id current
   ```
   If this fails, stop and ask the user to run `cargo crev id new
   --github-username <them>` first. Do **not** auto-generate an id.

2. The user's proof repo has been set up and is reachable.
   `cargo crev repo dir` prints the local path.

## High-level workflow

1. **Refresh the user's web of trust.** Run `cargo crev update` once
   at the start of the session to pull the latest proofs from every
   trusted reviewer's proof repo (it's a shortcut for
   `cargo crev repo update`). Without this, both candidate discovery
   (which depends on the `status` column reflecting current trusted
   reviews) and the later cross-check against existing reviews can be
   working off stale data, causing you to either review a crate
   someone else just signed off on or miss a recent advisory. See
   "Refreshing the web of trust" below.
2. **Pick the crate + version to review.** If the user names a specific
   crate, skip to the next step. Otherwise, use the candidate discovery
   step below to produce a ranked list and confirm the pick with the user.
3. **Start a review report file.** See "The review report file" below.
   Every finding from every subsequent step — positive or negative —
   gets appended to this file as you go. This is your scratchpad, your
   audit trail, and the source you distill into the final `comment`.
4. **Locate the crate's local source.** Use
   `cargo crev crate dir <name> <version>` to print the path to
   the crate source tree that cargo downloaded from crates.io. Record
   the path in the report.
5. **External verification.** Check that what crates.io shipped matches
   the public upstream repository at a specific commit, and that that
   commit corresponds to the advertised version tag. See "External
   verification" below. Record the outcome — pass, partial, or fail —
   in the report. Any unexpected discrepancy is a real finding and
   must be raised to the user before proceeding.
6. **Read the code.** See "Reviewing the code" below. Append findings
   to the report as you go.
7. **Cross-check against existing reviews.** Query the local proof
   database for any existing reviews of this crate (any version), read
   them, and reconcile with your own findings. See "Cross-checking
   existing reviews" below. Record the comparison in the report.
8. **Assemble the unsigned proof file.** Fill in the YAML fields
   (rating, thoroughness, understanding, comment, issues,
   advisories, `llm-agent`). The `comment` is **the full text of the
   report file** by default — see "Assembling the unsigned proof"
   below.
9. **Validate the proof file.** Round-trip the unsigned proof through
   `cargo crev review --import-unsigned-from ... --no-store --no-edit
   --print-unsigned`. This step is **mandatory** — fix and re-validate
   until it passes. See "Round-trip validating the draft" below.
10. **Hand off to the user.** The agent does **not** sign or publish.
   Give the user two files — the report and the unsigned proof —
   along with the exact command they should run to interactively
   review, edit and sign it. See "Handing off to the user" below.

## The review report file

Maintain a single markdown file per review, at:

```
target/crev/reviews/<crate>-<version>.md
```

(Create the directories if they don't exist.) Start it as soon as a
candidate is picked, **before** any inspection begins. Append to it
throughout the review — treat it as an append-only journal.

Minimum structure:

```markdown
# Review: <crate> <version>

- Local source: <path from `crate dir`>
- Upstream repository: <URL or "unknown">
- Upstream commit verified: <sha or "n/a">
- Upstream tag for version: <tag or "n/a">
- Verification outcome: pass | partial | fail

## External verification

<details of what was checked, what matched, what differed>

## Code review findings

<ongoing notes — one bullet per observation, grouped by file or theme>

## Cross-check against existing reviews

<how many prior reviews, agreement, disagreement, verdict on whether
they change your draft fields>

## Open questions / things skipped

<anything you didn't examine and why>

## Draft review fields

- rating: ...
- thoroughness: ...
- understanding: ...
```

Rules:

- **Append, don't rewrite.** If you change your mind about something,
  add a follow-up note; don't quietly edit earlier entries. The user
  should be able to see how your understanding evolved.
- **Every problem, however small, goes in.** Even if you decide it's
  not worth reporting in the final proof, it belongs in the report so
  the user can sanity-check your judgement.
- **Record what you did *not* look at.** If you skipped the test
  directory, or only skimmed `build.rs`, say so explicitly under
  "Open questions / things skipped". This is what backs honest
  `thoroughness` / `understanding` values later.
- **Hand the report to the user** alongside the unsigned proof file
  at the end of the workflow (see "Handing off to the user"). The
  report is how the user actually audits *your* work; the unsigned
  proof is the thing they sign after auditing.

## Refreshing the web of trust

Before any candidate discovery or cross-checking, pull the latest
proofs from every trusted reviewer:

```sh
cargo crev update
```

This is a shortcut for `cargo crev repo update`. It walks every proof
repository in the user's WoT and `git fetch`es it, so the local proof
database reflects the current state of the world. It can take a few
seconds to a minute depending on how many ids the user trusts and
network conditions; that's normal.

Run this **once per session**, before the first candidate-discovery
pass, and again only if the session has been running long enough that
the data is plausibly stale or the user explicitly asks. It is not
necessary to re-run it between successive reviews in the same session.

If the command fails for some repos (network blip, dead remote,
auth issue) but succeeds for others, that is usually fine — note the
failures in case they matter, but proceed. If it fails entirely
(e.g. no trusted ids configured, so there is nothing to fetch), stop
and tell the user: candidate discovery and cross-checking will both
be meaningless until the WoT is set up.

## Finding candidate crates to review

When the user hasn't named a specific crate, first make sure the WoT
is fresh (see "Refreshing the web of trust" above), then capture a
full stats dump of their dependencies. This is a potentially slow
operation (it walks every dep, downloads metadata, etc.), so **run it
once and store the output in a file** — don't re-run it unless the
user asks.

```sh
cargo crev verify --show-all --force-print-header \
    > target/crev-verify.txt 2>&1
```

Notes:

- `--show-all` enables every available column (reviews, issues, owners,
  downloads, loc, lpidx, geiger, flags, latest trusted version, …).
- `--force-print-header` makes the tool print the column header even
  when stdout is redirected. **Always** use this when capturing — the
  header is the only reliable way to know which column is which.
- Exit status `255` is expected and does **not** mean the command
  failed. It signals `VerificationFailed` — i.e. at least one
  dependency is not yet fully verified, which is obviously the case
  when you're about to start reviewing. Only treat it as a real error
  if the stdout file is empty or the stderr contains an actual error
  message.
- Status column values you'll see:
  - `local` — crate from a local path source (skip; not reviewable via
    crates.io).
  - `none`  — no existing trusted review; prime candidate.
  - `pass`  — already has sufficient trusted reviews; skip.
  - `flagged` / `dangerous` — has issues reported against it; these
    may be worth confirming rather than reviewing fresh.
  - `N/A`   — there are no trusted ids in the user's WoT at all. If you
    see this, stop and tell the user they should set up trust first
    (`cargo crev trust <id>`) before candidate discovery is meaningful.

Once the file exists, pick candidates from it. Before narrowing down,
confirm the criteria with the user — see the "Picking candidates"
section below.

## Picking candidates

Picking what to review is inherently a bit arbitrary, but the default
goal is: **find the non-passing, non-local, least-reviewed,
least-downloaded package** — i.e. the most obscure crate in the user's
dependency tree that nobody else is likely to be reviewing any time
soon. Unless the user says otherwise, apply these filters/rankings in
order:

1. **Filter by `status`.** Keep only `status = none`. Skip `pass`
   (already covered), `local` (not from crates.io), and `N/A`
   (configuration problem, handled separately). `flagged` / `dangerous`
   are a separate workflow (confirming pre-existing issue reports) —
   don't pick them unless the user asked.
2. **Sort by lowest total review count.** Use the second `reviews`
   column (total across all versions, not the per-version count).
   Zero-review crates come first.
3. **Tiebreak by lowest total download count.** Popular crates will
   eventually get reviewed by someone else; obscure ones won't.
   Reviewing the long tail is higher marginal value. Use the second
   `downloads` column (total).
4. **Tiebreak by `CB` flag.** Among otherwise equal candidates, prefer
   crates with a custom build script — they run arbitrary code at build
   time and are higher-risk, so the review is more valuable.

If the user has additional or different preferences (e.g. "I only care
about direct deps", "prioritize high lpidx", "skip `-sys` crates"), they
override the defaults above — ask if unsure.

**The user's own crates are valid candidates.** If a top candidate turns
out to be authored by the user themselves, don't filter it out — a
review where the author says "I wrote this and had an LLM agent audit
it" is still a useful signal for downstream consumers, and once
published it'll flip to `status = pass` and drop out of the filter
naturally. Agent provenance is recorded structurally via the
`llm-agent:` field on the proof (see "Disclosing agent provenance"
below), so no special comment wording is needed for own-crate reviews.

**Pick one candidate, not a shortlist.** Filter the captured stats file
by the rules above, sort by the priority order (status → review count →
download count → CB flag), and propose the single most promising crate
to the user. Don't offer a menu — it just slows things down. After that
crate is reviewed and published, repeat the selection process on the
same captured file (or re-capture if it's stale) to pick the next one.

### Column legend

`cargo crev verify --help` ends with a full column legend.
Reproduced here so agents can interpret the captured stats file without
re-running the command:

```
- status     - Trust check result: `pass` for trusted, `none` for lacking reviews,
               `flagged` or `dangerous` for crates with problem reports.
               `N/A` when crev is not configured yet.
- reviews    - Number of reviews for the specific version and for all available
               versions (total)
- issues     - Number of issues reported (from trusted sources / all)
- owner      - Owner counts from crates.io (known / all) in non-recursive mode
- downloads  - Download counts from crates.io (version / total)
- loc        - Lines of Rust code
- lpidx      - "left-pad" index (ratio of downloads to lines of code)
- geiger     - Geiger score: number of `unsafe` lines
- flgs       - Flags for specific types of packages
  - CB         - Custom Build (runs arbitrary code at build time)
  - UM         - Unmaintained crate
- name       - Crate name
- version    - Crate version
- latest_t   - Latest trusted version
```

## External verification

Before reading a single line of the crate's code, verify that what
crates.io shipped actually matches the public upstream repository. This
catches the single highest-leverage class of attack (a publisher whose
account was compromised, who can ship a tarball that doesn't match
their public git history). It also tells you *which* upstream commit
the reviewed snapshot corresponds to, which is valuable information in
its own right.

Ideal outcome, recorded in the report file:

> Package matches public repo `<url>` at commit `<sha>`, which is the
> commit tagged as `<tag>` corresponding to version `<version>`.

Anything short of that is a partial or failed verification, and **every
discrepancy must be recorded in the report and raised to the user**.
Don't silently proceed past a verification issue — if the user hasn't
told you what to do in that case, stop and ask.

### Procedure

1. **Get the local source path.**
   ```sh
   cargo crev crate dir <name> <version>
   ```
   Record it in the report. Call this path `$SRC`.

2. **Look for `.cargo_vcs_info.json`** at `$SRC/.cargo_vcs_info.json`.
   When present (it usually is), it contains the upstream git sha the
   tarball was built from:
   ```json
   { "git": { "sha1": "abc123…" }, "path_in_vcs": "subcrate" }
   ```
   Treat this as a **hint, not a guarantee**. The file may be missing
   (older crates, unusual publish flows), or the sha may be slightly
   off (dirty worktree at publish time). If it's missing, note that in
   the report and fall back to searching the upstream repo by version
   tag instead.

3. **Find the upstream URL.** Read `$SRC/Cargo.toml.orig` (the pristine
   pre-publish manifest) and extract `package.repository`. If absent
   or dead, also check `package.homepage`, then crates.io metadata.
   If none of those point at a reachable repository, **search the
   web** (e.g. `"<crate-name>" site:github.com` or the author name
   plus the crate name) — crates sometimes move or get renamed and
   the manifest stays stale. Record every URL you tried and which one
   resolved. If you still cannot find a public source after a
   reasonable search, record the failure and ask the user how to
   proceed. Do not skip external verification silently.

4. **Clone (or update) the upstream cache.** Use a stable cache
   directory so repeat runs don't re-clone:
   ```sh
   mkdir -p target/crev/review-cache
   UPSTREAM=target/crev/review-cache/<crate>
   if [ -d "$UPSTREAM" ]; then
     git -C "$UPSTREAM" fetch --tags origin
   else
     git clone --filter=blob:none "$REPO_URL" "$UPSTREAM"
   fi
   ```
   `--filter=blob:none` keeps the clone fast even for huge histories —
   blobs are fetched on demand when you actually diff.

5. **Identify the upstream commit to compare against.** Prefer, in
   order:
   1. The sha from `.cargo_vcs_info.json` (if it exists upstream).
   2. The commit that the version tag points to — tag naming
      conventions to try:
      `v<version>`, `<version>`, `<crate>-v<version>`, `<crate>-<version>`.
   3. If neither matches, report the failure and ask the user.
   Record in the report which method succeeded and which commit/tag
   was chosen. Ideally both the sha and the version tag agree — if
   they disagree (e.g. vcs_info points at a commit the tag does not),
   that is itself a finding and must be reported.

6. **Check out the commit.**
   ```sh
   git -C "$UPSTREAM" checkout "$SHA"
   ```

7. **Compare the trees.** Compare `$SRC` against
   `$UPSTREAM/<path_in_vcs>` (or `$UPSTREAM` if `path_in_vcs` is empty
   or `.`). Use `diff -r --brief` for an initial pass, then drill into
   any differing file with a full `diff` to see the content.

   **Files expected to differ or to exist on only one side** — add
   these to an ignore list and do not flag them:
   - `Cargo.toml` — always rewritten by `cargo publish`. Compare
     `$SRC/Cargo.toml.orig` against upstream's `Cargo.toml` instead.
     Workspace-inherited fields may differ; those differences are
     expected and benign as long as they're clearly inheritance
     artefacts.
   - `Cargo.toml.orig` — exists only in the crate tarball.
   - `.cargo_vcs_info.json` — exists only in the crate tarball.
   - `.cargo-ok` — cargo extraction marker.
   - `Cargo.lock` — sometimes present, sometimes stripped.
   - `.gitignore`, `.gitattributes` — often excluded via `exclude`.
   - `target/` — build artifacts, never in the tarball.

   Any other difference is real and must be recorded in the report
   with the file path and a short description of what differs.

8. **Cross-check the manifest.** Diff `$SRC/Cargo.toml.orig` against
   `$UPSTREAM/<path_in_vcs>/Cargo.toml` at the checked-out sha.
   Expected differences for workspace-published crates: fields like
   `version`, `authors`, `edition`, `license` being inlined from the
   workspace root; `[workspace]` stanza absent from the crate copy;
   path dependencies resolved to version specs. Flag anything else.

9. **Record the outcome in the report** under
   "## External verification", e.g.:

   > Package matches public repo
   > <https://github.com/foo/bar> at commit
   > `abcdef0123` tagged `v1.2.3`. Manifest differences are limited
   > to expected workspace-inheritance artefacts (listed above). All
   > other files bit-for-bit identical.

   or, on failure:

   > Package does **not** cleanly match upstream. The file
   > `src/util.rs` differs from the upstream copy at the commit
   > recorded in `.cargo_vcs_info.json` (see diff below). Raising
   > to user before continuing.

Any verification failure blocks the rest of the review until the user
says what to do.

## Internal verification (reviewing the code)

Once external verification has passed (or the user has explicitly told
you to proceed past a failure), start the actual code review. This is
"internal verification": convincing yourself that the code inside the
verified tarball does what it claims to and nothing more.

### Mindset

Approach the code **adversarially**. Assume, as a working hypothesis,
that everything you are reading could be wrong or hostile, and it is
your job to either falsify that hypothesis or document what you found.
This is not paranoia; it's the baseline for a review that is worth
signing your name to.

Concrete rules that follow from this:

- **Do not trust comments, docstrings, or variable names at face
  value.** They are not executable. A function called
  `sanitize_input` can do anything; a comment saying "// does not
  allocate" is not a proof. Verify claims by reading the code they
  describe, and note in the report when a comment or name is
  misleading (even innocently).
- **Do not trust the crate's README, `description`, or top-level
  rustdoc as a guarantee of behavior.** They are part of the "claims"
  baseline, and the whole point of the review is to check them against
  reality. A crate that says "pure parsing, no I/O" and then calls
  `std::fs::read` is a finding regardless of whether the author meant
  well.
- **Do not trust that the author had good intentions.** Assume every
  line might have been written by someone trying to smuggle something
  past you. If you can't explain why a piece of code is there, that is
  itself a finding — write it down, don't paper over it with a
  charitable guess.
- **Do not trust that existing reviews (if any) caught everything.**
  They might have been cursory, or written before a suspicious line
  was added. Every line is your responsibility at the version you are
  looking at.
- **Do not trust test files to reflect real behavior.** Tests only
  cover the paths the author thought to test (or chose to show). A
  malicious crate can have a beautiful test suite. Tests are useful
  context, not evidence of correctness.
- **Do not trust that "obvious" code is harmless.** Supply-chain
  attacks hide in the most boring-looking places (a string constant,
  an inlined byte array, a one-line helper). Give every file a real
  look, not a glance.
- **When something confuses you, record the confusion, don't resolve
  it by assumption.** Unresolved questions go under "Open questions"
  in the report. The user can then decide whether to dig deeper or
  downgrade `understanding`.

The goal is not to be uncharitable — most crates you review will be
fine. The goal is that *if* something is wrong, your default posture
catches it rather than glossing over it.

### Step 1: pick a thoroughness level

Do this **before** opening any file, and record the decision in the
report with a one-sentence justification. The level dictates which
strategy you follow below.

The canonical cargo-crev definitions (from `editing-package-review.md`)
are framed for humans in terms of "time per file":

- `high`   — hour or more per file; formal security review.
- `medium` — ~15 minutes per file; standard focused review.
- `low`    — ~2 minutes per file; quick pass for obvious issues.
- `none`   — seconds per file; just skimming, or metadata-only.

For an LLM agent, "time per file" doesn't translate directly (you read
fast, but your value is in careful reasoning, not raw throughput). Use
these rough mappings instead, but stay honest: if you only read half
the files or didn't fully think through the unsafe blocks, downgrade.

**Default thoroughness by crate size.** Estimate the Rust LoC from the
captured stats file (the `loc` column) or from
`tokei <path> --type Rust` if you have it. If the user has specified a
thoroughness level, **their instruction overrides these defaults**.

| Rust LoC        | Default thoroughness | Rationale                              |
|-----------------|----------------------|----------------------------------------|
| < 300           | `medium` or `high`   | Small enough to read carefully end-to-end. |
| 300 – 1 500     | `medium`             | Read everything once, inspect hotspots.|
| 1 500 – 10 000  | `low`                | Full read is impractical; focus on red flags and critical paths. |
| > 10 000        | `low` or `none`      | Only sample; say so explicitly.        |

**Random file sampling for large crates.** When a crate is too large to
read every file (roughly > 1 500 Rust LoC), select a random sample of
at least 10 `.rs` files and review those thoroughly. Use a command like:

```sh
find <src> -name '*.rs' | shuf | head -n 10
```

Every sampled file MUST be reviewed thoroughly in full — read the entire
file, trace data flow, check for red flags, and record findings. Sampled
files MUST NOT be skipped, read partially, or given a superficial scan.
The whole point of sampling is to give a smaller set of files the deep
attention that the full crate cannot receive; a partial review of a
sampled file defeats the purpose entirely. Record which files were
randomly selected and note in the report that the selection was
randomised. The non-negotiable checks (build.rs, unsafe blocks,
proc-macro, deps, FFI) still apply to the entire crate regardless of
sampling.

**Non-negotiable, regardless of level.** The following items get
examined in every review, even at `low` or `none`:

- **Form a "claims" baseline from crate metadata, then verify against
  reality.** Before reading code, read:
  - `Cargo.toml`: `description`, `keywords`, `categories`, `homepage`,
    `documentation`, `repository`, feature flags.
  - `README.md` and any top-level docs.
  - The crate-level rustdoc (`//!` comments in `src/lib.rs`).
  Write a 1–3 sentence summary in the report of **what the crate
  claims to do**. Everything else in the review is essentially a
  check that the code matches this claim and does nothing beyond it.
  Any capability the code exercises that isn't implied by the claim
  (network access, filesystem writes, spawning processes, reading
  env vars, FFI, `unsafe`, pulling in unrelated deps) is a finding
  worth recording even if it turns out to be benign. The agent's job
  is to answer: *"does this crate do only what it claims to do?"*
- **`build.rs`** (any custom build script). Read it top to bottom. This
  is the single highest-leverage attack surface — it runs arbitrary
  code on the user's machine at build time. Flag on sight: network
  calls, shelling out to `curl`/`wget`/`sh`, writing outside `OUT_DIR`,
  reading `~/.ssh/`, `~/.aws/`, `~/.gnupg/`, env-var exfiltration,
  obfuscated strings, embedded binary blobs. If the crate has no
  `build.rs`, record that positively in the report.
- **`Cargo.toml` dependency list.** Cross-check every dependency
  against the crate's stated purpose. A JSON parser that pulls in
  `reqwest` is suspicious. A pure-math library that pulls in `libc` is
  suspicious. Git/path deps should not survive `cargo publish`; if you
  see one, that's a finding.
- **Proc-macro crates** (crates with `proc-macro = true` in
  `Cargo.toml`). Like `build.rs`, these run arbitrary code at compile
  time. Read every `#[proc_macro*]` function.
- **Every `unsafe` block.** Use `rg -n 'unsafe' <src>` to enumerate.
  For each, read the surrounding code and make a concrete argument
  about why it is (or isn't) sound.
- **FFI surface** (`extern "C"`, `bindgen`, linked C libraries).
  Record the name of every C library dep and note what it's used for.

Everything else scales with thoroughness level.

### Step 2: strategy per thoroughness level

Each strategy is additive — `medium` does everything `low` does, plus
more, and so on.

#### `none`

Metadata-only pass. Use only for tiny trivial crates (e.g. a single
type alias) or when you're creating a proof purely to flag issues
(`issues:` / `advisories:`), not to endorse.

Checklist:

- [ ] Read `Cargo.toml` and `Cargo.toml.orig`.
- [ ] Read `README.md` (if present) and the crate-level rustdoc.
- [ ] Record the "claims baseline" in the report (what the crate
      says it does).
- [ ] Run the non-negotiable checks above (claims vs reality,
      build.rs, deps, proc-macro, unsafe count, FFI surface).
- [ ] Record LoC, file count, and what you did *not* read in the
      report.

Do **not** claim `positive` or `strong` rating at this level.

#### `low`

Quick single pass focused on red flags. Target: roughly one pass
through every Rust file, spending real attention only on high-value
areas.

In addition to the `none` checklist:

- [ ] Enumerate all `.rs` files. Open each one at least once.
- [ ] For each file, scan for red-flag patterns (listed below). Dwell
      on any hit; move on if clean.
- [ ] Read every `unsafe` block, every `build.rs`, every
      `proc_macro*` function in full.
- [ ] Note anything that would make you uncomfortable *using* this
      crate yourself, even if you can't prove it's wrong.
- [ ] Explicitly list in the report which files you **read in full**
      vs. **read partially** (with line ranges) vs. **not read**.
      Do not use the word "skimmed" — it is ambiguous. As an LLM you
      either read the content or you didn't; there is no in-between.
      If you read the full file but only did a red-flag grep without
      careful analysis, say "**read in full (red-flag scan only)**".

Red-flag patterns to search for:

- `unsafe`, `transmute`, `from_raw`, raw pointer arithmetic.
- `std::process::Command`, `Command::new`, shell invocation.
- `std::env::var`, `std::env::vars` (env var exfiltration).
- `reqwest`, `ureq`, `hyper`, `curl`, raw TCP sockets — in crates that
  don't advertise themselves as network libraries.
- `include_bytes!`, `include_str!` pointing at suspicious paths.
- Base64 string literals, hex blobs, large `const` byte arrays.
- Obfuscated string construction (char-by-char, byte-level, xor'd).
- `unwrap`, `expect`, `panic!`, `unreachable!` in code paths that
  handle untrusted input.
- Conditional compilation (`#[cfg(...)]`) that behaves differently on
  different targets in non-obvious ways.
- `Drop` impls with side effects beyond freeing memory.
- Custom `serde` deserializers that allocate without bounds.

#### `medium`

A careful end-to-end read. Target: understand the crate's overall
architecture, read every non-test file in full, and spend deeper
attention on sensitive areas.

In addition to the `low` checklist:

- [ ] Read every file in `src/` in full, not just scan.
- [ ] Sketch a mental model of the crate's public API and internal
      data flow. Write a 3–5 sentence summary in the report.
- [ ] **Reconcile against the claims baseline.** For every capability
      exercised by the code (network, filesystem, processes, env,
      FFI, `unsafe`), confirm it is justified by something in the
      crate's stated purpose. Record each reconciliation explicitly:
      "reads `$HOME/.config/foo` — justified, crate is a config
      loader". Any unjustified capability is a finding.
- [ ] For each `unsafe` block, write a one-sentence soundness argument
      in the report (not just "looks fine").
- [ ] For any parser/deserializer, trace how untrusted bytes reach
      each `unwrap`/`expect`/`panic!`.
- [ ] Skim `tests/` and doctests to confirm behavior matches claims.
- [ ] Run `cargo tree` on the crate and note any dependency that
      surprises you.

#### `high`

Formal security review depth. Only claim this level if the user has
explicitly asked for it **and** you have actually gone this deep.

In addition to the `medium` checklist:

- [ ] Multi-pass reading: at least one architecture pass, one
      line-by-line pass, and one adversarial pass ("if I were
      attacking a user of this crate, where would I look?").
- [ ] For every `unsafe` block, work out the safety invariants
      explicitly (validity, alignment, aliasing, lifetime, thread
      safety) and argue them in the report.
- [ ] For every public API function that takes untrusted input,
      enumerate the failure modes and check that none of them can
      corrupt internal state or leak secrets.
- [ ] Cross-check cryptographic code against the Sherlock /
      RustSec-style checklist: canonical serialization, determinism,
      constant-time where needed, no ambient state, no
      locale-dependent parsing on consensus-critical paths.
- [ ] If the crate has FFI, note which sanitizer build
      (ASAN/MSAN/TSAN) would have exercised it, and whether the
      upstream CI actually runs one.
- [ ] Check release engineering: is the upstream using trusted
      publishing? Is the maintainer list stable? Any recent account
      changes?

**You should almost never produce a `high`-thoroughness review as an
agent.** An LLM can do the reading, but the level implies a human-
depth audit. Default to `medium` at most unless the user has been
very explicit.

### Step 3: record everything in the report

As you review, append to the report file continuously. By the end of
the code review phase, the report's "Code review findings" section
should contain:

- The chosen thoroughness level and the justification.
- A per-file map of "**read in full**" / "**read in full (red-flag scan
  only)**" / "**read partially (lines X–Y)**" / "**not read**" with
  reason. Never use the word "skimmed" — be precise about what you
  actually did.
- The results of every non-negotiable check (build.rs, deps, proc-
  macro, unsafe enumeration, FFI surface) — even if the result is
  "none present, positive".
- Every red-flag hit, with file:line and a short note on whether it's
  a real concern or a false positive (and why).
- A short architectural summary (at `medium` and above).
- An explicit list of anything you did not look at and why.

The `comment` field in the signed proof is distilled from this
section — typically a paragraph or two summarising the above in
reviewer-facing language. The full report stays on disk for the user
to audit *your* work.

### Interpreting results into proof fields

- `thoroughness`: the level you chose in Step 1. Do not inflate it.
- `understanding`: honest self-assessment of how well you grasped the
  code. If any `unsafe` block was "I think this is probably fine but
  I'm not sure", that is at most `medium` understanding.
- `rating`:
  - `strong`   — secure and good for all applications. Rare; avoid
    unless the crate is trivial and perfect.
  - `positive` — secure, ok to use, possibly minor issues. The
    default for a clean review.
  - `neutral`  — secure but with flaws worth noting. Default if you
    found anything non-trivial to mention.
  - `negative` — severe flaws, not ok for production.
  - *(the proof format also supports `dangerous` for "unsafe to use /
    possibly malicious" — use via `issues` / `advisories` rather than
    the `rating` field.)*
- `issues` / `advisories`: if you found a concrete bug or security
  problem, raise it here. Ask the user before doing so — this is the
  kind of thing they probably want to review personally.

## Disclosing agent provenance (`llm-agent`)

Every review produced via this skill **must** include an `llm-agent:`
field in the unsigned-review YAML. This is a structured, machine-
readable disclosure that the review was produced by an LLM agent, so
downstream consumers can weight it accordingly. It is not optional and
it is not a substitute for the `comment` — both exist.

Emit the field with:

- `model`: your own model identifier, as precise as you can get it
  (e.g. `"claude-opus-4-6"`, `"gpt-4o"`). If you genuinely don't know
  your own model id, use the closest public identifier you do know.
- `model-version`: include if you know a specific version/date string
  for your model; omit the key otherwise (it's optional, skipped when
  absent).
- `human-guided`: **always emit as `false`** by default. The user may
  later edit the proof to flip it to `true` after they have personally
  reviewed and verified the content — that's their call, not the
  agent's. Do not set it to `true` yourself even during an interactive
  session.

Example block to drop into the YAML:

```yaml
llm-agent:
  model: claude-opus-4-6
  human-guided: false
```

This applies to every review (own crates, third-party crates, trivial
crates, large crates — no exceptions). The `comment` field is for the
actual review findings; it does not need to repeat the "LLM-assisted"
disclosure because `llm-agent:` already conveys it.

## Cross-checking existing reviews

After you've formed your own view of the crate but **before** you
assemble the proof, check whether anyone else has already reviewed
this crate (any version, not just the target) and read what they
wrote. This is a sanity check on your own work, not a substitute for
it — do it last so your own findings are fully formed and you're not
anchored to a prior reviewer's conclusions.

This step assumes the local proof database is fresh. If you haven't
already run `cargo crev update` this session (see "Refreshing the
web of trust"), do it now — querying a stale database can miss a
recent advisory or sign-off.

```sh
# All reviews of any version of this crate:
cargo crev repo query review <crate-name>

# Only the target version:
cargo crev repo query review <crate-name> <version>
```

The output is one or more `package review` YAML documents separated
by `---`. For each one, note:

- Reviewer id and (if you recognise it) who they are.
- Version reviewed.
- `thoroughness`, `understanding`, `rating`.
- `issues` / `advisories` raised, if any.
- Key points from `comment`.

Then reconcile with your own findings. Write a short reconciliation
section into the report under `## Cross-check against existing
reviews` covering at least:

- **How many existing reviews** there are and for which versions.
- **Agreement**: which of your findings line up with prior reviews,
  and which prior ratings are consistent with yours.
- **Disagreement**: any prior finding you could not reproduce, any
  finding of your own that prior reviews missed, and any
  `thoroughness` / `rating` gap. Be specific — reference the prior
  reviewer's id and the line of their comment you're responding to.
- **Verdict**: does reading the existing reviews change your
  proposed `rating`, `thoroughness`, or `understanding`? If yes,
  update your draft fields and say why. If no, say so explicitly
  (so the user can see you didn't just ignore them).

Special cases:

- **Zero existing reviews.** Record that explicitly
  (`"No prior reviews of this crate at any version."`). This is
  common for obscure long-tail picks — it's also why the skill
  prioritises those crates.
- **Prior review flagged a security issue on an older version and
  you found a related pattern on the current version.** Flag it
  loudly in your findings and ask the user whether to raise it as
  an `issue` in your own proof, referencing the prior advisory id.
- **Prior review's rating is substantially more negative than
  yours.** Re-read the prior reviewer's comment carefully — they
  may have seen something you missed. If after reading you still
  disagree, say why in the reconciliation, and consider dropping
  your own rating one step (e.g. `positive` → `neutral`) out of
  epistemic humility.
- **Prior review's rating is substantially more positive than
  yours.** Stick to your own findings — do not anchor upward
  because someone else was charitable. Explain the gap in your
  comment.

## Assembling the unsigned proof

Write the unsigned review body to:

```
target/crev/reviews/<crate>-<version>.proof.yaml
```

(Same directory as the report file, next to it.) The body is a YAML
document with these fields:

```yaml
kind: package review
version: -1
date: "<RFC3339 timestamp>"
from:
  id-type: crev
  id: <user's current crev id — from `cargo crev id current`>
  url: <user's current crev-proofs URL — same source>
package:
  source: "https://crates.io"
  name: <crate name>
  version: <crate version>
  digest: <recursive blake2b digest of the crate source, base64url no-padding>
review:
  thoroughness: none|low|medium|high
  understanding: none|low|medium|high
  rating: negative|neutral|positive|strong
llm-agent:
  model: <your model id, e.g. claude-opus-4-6>
  human-guided: false
comment: |
  <full contents of the report file, indented under the YAML block
  scalar — see rules below>
```

**Put `comment:` last.** The `comment` block is typically hundreds of
lines of inlined report text, and anything written after it is
effectively hidden from the user when they open the file in an
editor during signing. Keeping `comment:` at the very end means every
small, easy-to-verify field (`from`, `package`, `digest`, `review`,
`llm-agent`) is visible without scrolling past the review body. This
also matches how cargo-crev itself serialises signed proofs —
`serde_content_serialize!` in `crev-data` always moves `comment` to
the bottom — so your draft stays close to the canonical form on
both sides of the signing step.

**Populating `from`.** Prefer using the user's actual current crev id
rather than a placeholder. Run:

```sh
cargo crev id current
```

The output is one line per own id, in the form:

```
<id> <url> (current)
```

Take the `<id>` and `<url>` from the line marked `(current)` and
write them into `from.id` and `from.url`. This makes the draft
self-consistent, lets the dry-validate round-trip (see below) show
the real id in its output, and still works correctly when the user
eventually signs (the signing step will re-set `from` from the
unlocked id, which will be the same value, so it's a no-op).

**Fallback placeholder.** If `cargo crev id current` fails (no id
configured — but the prerequisites section says to stop and ask in
that case, so this should not happen in practice) use the stock
43-character all-`A` placeholder: `AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA`
with `url: "https://placeholder.invalid"`. The YAML parser requires
`from.id` to be a valid base64url-encoded crev id (43 chars,
alphabet `A–Z a–z 0–9 - _`), so arbitrary strings like
`"placeholder"` will fail parsing.

### What goes in `comment`

**By default, `comment` is the entire text of the report file.** The
user can audit a self-contained proof without opening a second file,
the full audit trail travels with the signed artifact, and there's no
risk of the agent quietly dropping inconvenient findings during a
"summarisation" step.

Only compress or re-edit the report on its way into `comment` if there
is a concrete reason — e.g. the report exceeds cargo-crev's
`MAX_PROOF_BODY_LENGTH` (~32 KB) and would be rejected as-is. If you
**do** compress, state that explicitly as the first line of `comment`,
for example:

```
[COMPRESSED: original report truncated to fit proof body size limit;
full text at target/crev/reviews/<crate>-<version>.md]
```

…and keep the untouched report file on disk so the user can inspect
the original. Do not compress "just to be tidy" — verbosity is fine,
silent editorialising is not.

### Getting the package digest

Use the dedicated `crate digest` subcommand. It prints exactly the
value that goes into the `digest:` field of the proof:

```sh
cargo crev crate digest -u <name> <version>
```

The `-u` / `--unrelated` flag tells cargo-crev the crate does not
need to be a direct dependency of the current workspace. The output
is a single line — the recursive blake2b digest, base64url-encoded
without padding — which you paste straight into your YAML.

Do not hand-compute the digest: the recursive-digest algorithm and
its encoding are cargo-crev-specific and easy to get wrong.

### Round-trip validating the draft (mandatory)

**Every proof file must pass round-trip validation before handoff.**
This is not optional — never hand the user a proof that hasn't been
validated, and never skip this step even if the YAML "looks correct".

Run the proof through `cargo crev review` in dry-validate mode:

```sh
cargo crev review \
  --import-unsigned-from target/crev/reviews/<crate>-<version>.proof.yaml \
  --no-store --no-edit --print-unsigned
```

With this exact combination of flags (no store, no edit, print
unsigned only), the command does not unlock the user's crev id and
does not produce any side effects — it just parses, normalises, and
re-prints the proof body. If the command succeeds and the output
looks structurally sane (all fields preserved, `comment` intact),
the file is ready for handoff. If it fails, fix the YAML and
re-validate — repeat until it passes. Never hand the user a draft
that doesn't round-trip.

## Handing off to the user

The agent does **not** sign the proof, does **not** store it in the
local proof repo, and does **not** publish. Signing is the user's
affirmative act: it is their key and their reputation on the line.

At the end of the workflow, present the user with exactly three
things:

1. **The path to the report file**
   (`target/crev/reviews/<crate>-<version>.md`), with a one-line
   summary of the outcome (e.g. *"verification passed at
   <commit>; rated `positive` at `medium` thoroughness; one minor
   finding in `src/util.rs`"*).
2. **The path to the unsigned proof file**
   (`target/crev/reviews/<crate>-<version>.proof.yaml`).
3. **The exact command to run** to review, edit, and sign:

   ```sh
   cargo crev review \
     --import-unsigned-from target/crev/reviews/<crate>-<version>.proof.yaml
   ```

   This will:
   - load the unsigned YAML,
   - replace the placeholder `from` with the user's current crev id,
   - open the editor on the draft so the user can review, correct
     anything the agent got wrong, and flip `human-guided: true` if
     they verified it themselves,
   - sign and store the proof in the local proof repo once the
     user saves and exits the editor.

   Note that neither `--no-edit` nor `--no-store` appears here —
   the user **is** editing (that's the point) and they **are**
   storing. `--no-edit`/`--no-store` are agent-side flags for
   preparing drafts, not user-side flags for signing them.

### Batch handoff: the signing script

When reviewing multiple crates in a session, maintain a **single
executable script** that collects all the signing commands:

```
target/crev/sign-all.sh
```

Create it at the start of the first review (or when the user asks to
review multiple crates) and append to it as each review completes.
The script should:

1. Have a `#!/usr/bin/env bash` shebang and `set -euo pipefail`.
2. Contain a **single** `cargo crev review` invocation with multiple
   `--import-unsigned-from` arguments — one per completed review.
   The command accepts multiple files and unlocks the user's identity
   only once for the whole batch:

   ```sh
   cargo crev review \
     --import-unsigned-from target/crev/reviews/foo-1.2.3.proof.yaml \
     --import-unsigned-from target/crev/reviews/bar-0.4.1.proof.yaml \
     --import-unsigned-from target/crev/reviews/baz-2.0.0.proof.yaml
   ```

   Use `cargo run -- crev review ...` if the session is using the
   local build of cargo-crev.
3. **Remove lines for crates the user has already signed.** When the
   user tells you they've signed some, or you can verify via
   `cargo crev repo query review <crate> <version>`, remove the
   corresponding `--import-unsigned-from` argument from the command.
4. Be executable (`chmod +x`).

This way the user can run the script once, signing each review
interactively as the editor opens in sequence, with the passphrase
prompted only once. Tell the user the script path after each batch
of reviews completes.

After the user has signed the proofs, if they want to publish to their
public proof repo, that's a separate explicit step on their side:

```sh
cargo crev publish
```

The agent never runs `publish`.

## Rating / thoroughness / understanding cheat-sheet

> TODO(interactive): record the user's personal calibration for these
> values. For now, default to conservative choices and ask.

- `rating`: start with `neutral` unless the user tells you otherwise.
- `thoroughness`: reflect how much you actually read.
- `understanding`: reflect how much you actually understood.

## Things to never do without asking

- Never sign the proof yourself. The agent prepares the unsigned
  YAML file; the user runs `cargo crev review --import-unsigned-from`
  to edit and sign. Signing is the user's affirmative act.
- Never run `cargo crev publish`. Publishing is a separate,
  explicit step the user takes after they've audited and signed.
- Never fabricate a review you didn't actually perform. If you skipped
  files, say so in the `comment`.
- Never set `thoroughness: high` or `understanding: high` unless the user
  has explicitly confirmed.
- Never create trust proofs (`cargo crev id trust`) from this skill —
  that's a separate, higher-stakes action.
- Never omit the `llm-agent:` field from a review you produce, and
  never set `human-guided: true` yourself. Flipping that flag is the
  user's decision after they personally verify the review.
- Never hand off a proof file that hasn't passed round-trip validation
  (`cargo crev review --import-unsigned-from ... --no-store --no-edit
  --print-unsigned`). Always validate, fix, and re-validate until it
  passes.

## Delegation policy (sub-agents)

**Prefer doing reviews yourself.** The review workflow described in this
skill requires careful, adversarial reading of code, nuanced judgement
about unsafe blocks and FFI, thorough external verification with diffs,
and honest self-assessment of thoroughness/understanding. These are
difficult to delegate well.

**Default: do not delegate reviews to sub-agents.** Perform each review
in the main conversation context where you have the full skill
instructions, the review report as a living document, and can maintain
the adversarial mindset throughout.

**If the user explicitly asks you to parallelise** (e.g. "review several
at once", "keep going in the background"), you may delegate to
sub-agents, but only under these conditions:

1. **Pass the full skill context.** The sub-agent prompt must include
   the complete review procedure — not a summary, not bullet points,
   but the actual workflow steps, the thoroughness-level checklists,
   the external verification procedure, the non-negotiable checks, and
   the adversarial-mindset rules from this skill file. If the prompt
   would be too long, do fewer reviews in parallel rather than cutting
   corners on the instructions.

2. **Include the exact proof YAML template** with the user's crev id,
   proof URL, and package digest already filled in, so the sub-agent
   doesn't have to guess or look them up.

3. **Specify the thoroughness floor.** Tell the sub-agent which
   thoroughness level to target (based on LoC) and that it must not
   inflate its self-assessment. If it only skimmed files, it must say
   `low`, not `medium`.

4. **Require the full report file.** The sub-agent must produce the
   same structured report (`target/crev/reviews/<crate>-<version>.md`)
   with all sections: external verification, file map with read/skimmed
   annotations, non-negotiable checks, red-flag scan results,
   architecture summary, claims-vs-reality reconciliation, cross-check
   against existing reviews, and open questions.

5. **Quality over quantity.** It is always better to produce 3 thorough
   reviews than 10 shallow ones. A review that inflates its
   thoroughness or misses findings is worse than no review — it gives
   false confidence to downstream consumers. If you cannot maintain
   quality at the requested parallelism, reduce the parallelism.

6. **Verify sub-agent output.** After a sub-agent completes, spot-check
   its report before adding the proof to the signing script. If the
   report is shallow (e.g. no external verification diff, no per-file
   annotations, no unsafe analysis), either redo the review yourself
   or send the sub-agent back with corrections.

## Open questions for the user (interactive session)

These are placeholders for the user's interactive input. Update this file
as answers come in.

1. What files/directories do you typically skip when reviewing a crate?
2. How do you decide between `issue` and `advisory`?
3. How do you pick `thoroughness` and `understanding` values?
4. Do you want agent reviews tagged somehow in the `comment` (e.g. a
   prefix like `[agent-assisted]`)?
5. Any crates/patterns that are automatic red flags for you (`build.rs`,
   `unsafe`, proc macros, network calls, etc.)?
