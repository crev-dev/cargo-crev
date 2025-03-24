# Changelog

<!-- next-url -->
## [Unreleased](https://github.com/crev-dev/cargo-crev/compare/v0.26.0...HEAD) - ReleaseDate

- Added mac keychain support to safely store and retrieve passphrases

## [0.26.0](https://github.com/crev-dev/cargo-crev/compare/v0.25.11...v0.26.0) - 2024-11-07

- Fixed handling of the `--diff` flag.

## [0.25.11](https://github.com/crev-dev/cargo-crev/compare/v0.25.9...v0.25.11) - 2024-10-24

- Updated dependencies

## [0.25.9](https://github.com/crev-dev/cargo-crev/compare/v0.24.0...v0.25.9) - 2024-05-19

- `crevette` tool for `cargo-crev` to `cargo-vet` export
- `cargo crev review --diff` suggests a URL for viewing a diff
- `cargo crev review` remembers the last `cargo crev open` crate.

## [0.24.0](https://github.com/crev-dev/cargo-crev/compare/v0.23.0...v0.24.3)

- Added `--direct` flag to trust parameters to use only directly trusted Ids
- Added command "proof reissue" to reissue reviews under a different id.
  The original proof will be referenced in the new proof under the "original:" field
- Fix crash on systems with libgit2 v1.4
- Fix the `crate clean` working directory check.
- Fix the `crate review` working directory check.


## [0.23.0](https://github.com/crev-dev/cargo-crev/compare/v0.22.2...v0.23.0) - 2022-01-22

- Ignore cargo directory source replacements - this essentially ignores `cargo vendor` and `cargo crev` will not read vendored sources
- Proofs moved from the config directory (e.g. ~/.config/crev/proofs)
  to the data directory (e.g. ~/.local/share/crev/proofs).
- Added command `config data-dir`.
- Added command `config cache-dir`.
- `crate verify`: better column width detection
- Better broken pipe errors handling
- Fix `verify --no-dev-dependencies` being ignored
- Deprecate `--no-dev-dependencies`. Make it the default. Introduce `--dev-dependencies` instead.
- Fix binary releases by switching to Github Actions
- Make `crate {goto,dir,open,expand}` assume `-u` outside of an existing Rust project.
- Introduce trust and package review "overrides" which allow overriding (ignoring) specific
   trust / package review
- Add `cargo crev wot log` to help understand your WoT.

## [0.22.2](https://github.com/dpc/crev/compare/cargo-crev-v0.21.4...v0.22.2) - 2022-01-11

- Use `cargo-release` to make our release process more reasonable.
- Display better diagnostics for any digest mismatch.
- Fix interactive use of `trust` and `id trust` to actually read the proofs edited

## [0.21.1](https://github.com/dpc/crev/compare/cargo-crev-v0.18.0...cargo-crev-v0.21.0) - 2020-05-29

- Lots of minor improvements
- We're really sorry, but it takes a considerable effort to maintain a clean
  CHANGELOG for a project still going through a lot of smaller and bigger changes.

## [0.18.0](https://github.com/dpc/crev/compare/cargo-crev-v0.16.1...cargo-crev-v0.17.0) - 2020-04-29

- Faster fetching on `repo fetch ...`
- Fix `verify` reporting missing user
dir

## [0.17.0](https://github.com/dpc/crev/compare/cargo-crev-v0.16.1...cargo-crev-v0.17.0) - 2020-04-29

- user interface now exposes distinction between URLs self-reported by an owner
  of a Crev Id, and unverified URLs reported about others.
  - `id query all` displays whether URLs have been verified to belong to their
    Crev Id: `==` signed and verified, `~=` signed but not fetched, `??`
    reported by others only.
  - `id trust` propagates only URLs signed by their own Crev Id.
- `repo fetch url` reports which Crev Ids belong to the repo (have the same URL)
  and which were copied from other
repos.

## [0.16.1](https://github.com/dpc/crev/compare/cargo-crev-v0.16.0...cargo-crev-v0.16.1) - 2020-02-11

### Fixed

- Fix default `features` not recognized in some
crates

## [0.16.0](https://github.com/dpc/crev/compare/cargo-crev-v0.15.0...cargo-crev-v0.16.0) - 2020-02-11

### Fixed

- Support for new cargo lockfile
version

## [0.15.0](https://github.com/dpc/crev/compare/cargo-crev-v0.14.0...cargo-crev-v0.15.0) - 2020-01-14

### Fixed

- `crate verify` no longer hangs on unpublished local crates
- Use effective instead of direct trust in WoT graph calculations

### Changed

- Make most columns in `crate verify` optional with `--show-xyz` options.
- Added some helpful informative
messages.

## [0.14.0](https://github.com/dpc/crev/compare/cargo-crev-v0.13.0...cargo-crev-v0.14.0) - 2019-12-16

### Fixed

- `crate verify` performance for local crates
- `cargo install` works without `--locked` flag

### Changed

- `alternatives` are reported in both source and
destination

## [0.13.0](https://github.com/dpc/crev/compare/cargo-crev-v0.12.0...cargo-crev-v0.13.0) - 2019-11-26

### Changed

- Handle local packages more consistently

### Fixed

- `comment` field serialization in trust proofs
- `crate verify` return code
- Stale
documentation

## [0.12.0](https://github.com/dpc/crev/compare/cargo-crev-v0.11.0...cargo-crev-v0.12.0) - 2019-11-19

### Changed

- Comment field handling in proofs and drafts

### Fixed

- Documentation here and there
- Backfill `kind` field when deserializing old-format content
- `--skip-known-owners` and `--skip-verified` returning errors

## Added

- Add `config dir` and `repo
dir`

## [0.11.0](https://github.com/dpc/crev/compare/cargo-crev-v0.10.1...cargo-crev-v0.11.0) - 2019-11-01

# Changed

- Change the proof format to simplify it and allow 3rd party "kinds" of them.
  The main goal is to support `git-crev` project:
  <https://github.com/crev-dev/git-crev>
- Tune tokei line counts to exclude tests and examples

## Added

- Ability report the crate as unmaintained and/or list alternatives to it
- `--target` to `crate verify` to ignore crates not compiled on the given
  (default: current) target
- `--for-id` for `cargo crev repo fetch trusted`
- Autocompletion generation with `config completions`
- `crate info` subcommand to get some detailed info for a single dependency

### Fixed

- Stale old-syntax commands in documentation
- Minor help
messages

## [0.10.1](https://github.com/dpc/crev/compare/cargo-crev-v0.10.0...cargo-crev-v0.10.1) - 2019-10-13

# Changed

- Fixed "Getting Started" documentation module
- Updated dependencies
- Fixed minor bugs & some QoL
improvements

## [0.10.0](https://github.com/dpc/crev/compare/cargo-crev-v0.9.0...cargo-crev-v0.10.0) - 2019-10-07

### Changed

- **BREAKING**: `cargo crev <verb> <noun>` was change to `cargo crev <noun>
  <verb>`
- Introduces one letter aliases for most (all?) commands
- Commands querying proofs will now print them as a multi-object yaml document
  for easier parsing
- Shortened `=1.2.3` in `latest_t` to just `=`

### Added

- `id untrust` and `crate unreview` to overwrite/clean errnous review/trust
  proofs
- `crate mvp` to discovering best reviewers
- `crate search` for looking up best reviewed dependency candidates
- `crate verify --recursive`
- `CREV_PASSPHRASE_CMD` for users of `pass` and similar
- Multiple flags and arguments to narrow down `crate verify` scope
- Handling of `--level <level>` in many commands
- "Tips and tricks" in user
documentation

## [0.9.0](https://github.com/dpc/crev/compare/cargo-crev-v0.8.0...cargo-crev-v0.9.0) - 2019-08-26

### Changed

- Performance improvement in `verify`

### Fixed

- Fixed detailed help for `verify` not showing.
- Renamed `cargo crev * id` to `cargo crev id *`, e.g. `cargo crev id new`,
  `cargo crev id export`. Added `cargo crev id show`.
- Combined `advise`, `flag`, `report` into `review --advisory` and `review
  --issue`

## [0.8.0](https://github.com/dpc/crev/compare/cargo-crev-v0.7.0...cargo-crev-v0.8.0) - 2019-07-11

### Changed

- `verify deps` was renamed to just `verify`
- Not saving the default draft is considered as canceling the operation.
- Revamp *advisories* system and add

### Added

- Statically compiled release binaries
- User Documentation, including Getting Started Guide
- `query dir` command
- Differential reviews with `diff` and `review --diff` commands
- New options, particularly for
`verify`

## [0.7.0](https://github.com/dpc/crev/compare/cargo-crev-v0.6.0...cargo-crev-v0.7.0) - 2019-04-27

### Added

- Advisories (<https://github.com/dpc/crev/wiki/Advisories>)
  - `cargo crev advise [name [version]]`
  - `cargo crev query advisory [name
[version]]`

## [0.6.0](https://github.com/dpc/crev/compare/cargo-crev-v0.5.0...cargo-crev-v0.6.0) - 2019-04-13

### Changed

- BREAKING: Switch cryptography to standard Ed25519/RFC8032. This will render
  existing IDs and artificates invalid. We're sorry for that. Please recreate
  your IDs, and use `cargo crev import proof` to recreate your reviews.

### Added

- `cargo crev edit config` allows interactive user config edition
- `open-cmd` in user config for customizing `cargo crev open` command
- `cargo crev import proof` for mass-import of
proofs

## [0.5.0](https://github.com/dpc/crev/compare/cargo-crev-v0.4.0...cargo-crev-v0.5.0) - 2019-03-06

### Added

- `unsafe` counts via `geiger`
crate

## [0.4.0](https://github.com/dpc/crev/compare/cargo-crev-v0.3.0...cargo-crev-v0.4.0) - 2019-01-12

### Added

- This `CHANGELOG.md` file.
- `LICENSE` files
- Ability to work without an Id for most commands.
- `open` command to help IDE users.
- Tracking effective trust level in WoT.
- Distrust calculation when calculating WoT.
- `review` command options: `--print-[un]signed` and `-no-store`.
- `export` and `import` commands for Ids.
- New column in `verify deps`: `lines` - line counts using `tokei`.
- New column in `verify dpes`: `flags` - Custom Build.
- `verify deps` option: `--for-id`.
- Exit status on `verify deps` to make it usable in CI pipelines.
- Counts of new proofs on `fetch` commands.
- Effecttive trust level column in `query id trusted` output.
- `update` command.

### Changed

- Windows cache folder changed from `%AppData%\Local\Dawid
  Ci,281,,380,arkiewicz\crev` to `%AppData%\Local\crev`.
- Windows config folder changed from `%AppData%\Roaming\Dawid
  Ci,281,,380,arkiewicz\crev` to `%AppData%\Roaming\crev`.
- MacOS config folder changed from `$HOME/Library/Application Support/crev` to
  `$HOME/Library/Preferences/crev`.
- Improve `verify deps` names and format.
- Handle error messages better in many places.
- Use host-specific salt in paths of proof files, to prevent dealing with git
  conflicts when sharing Id between many machines
- Make newer reviews (for the same package and version) effectively overwrite
  older ones.
- Change `push`, `pull`, `publish` to be more ID-sharing (between hosts)
  friendly
- Rename `--independent` to `--unrelated` and add `-u` as a short version.
- Avoid fetching things during normal work (helps offline use).
- Hardcode dpc's proof-repo url on `fetch all` to help bootstrap the ecosystem.

### Fixed

- Fix `$EDITOR`/`$VISUAL` handling, especially on Windows
- Save `lanes` in `LockedId`. Old Ids need to be fixed manually.

## \[0.3.0\] - 2018-12-28

Changelog was not maintained for this and earlier releases
