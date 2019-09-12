# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.9.0](https://github.com/dpc/crev/compare/cargo-crev-v0.8.0...cargo-crev-v0.9.0) - 2019-08-26
### Changed

* Performance improvement in `verify`

### Fixed

* Fixed detailed help for `verify` not showing.
* Renamed `cargo crev * id` to `cargo crev id *`, e.g. `cargo crev id new`, `cargo crev id export`. Added `cargo crev id show`.
* Combined `advise`, `flag`, `report` into `review --advisory` and `review --issue`

## [0.8.0](https://github.com/dpc/crev/compare/cargo-crev-v0.7.0...cargo-crev-v0.8.0) - 2019-07-11
### Changed

* `verify deps` was renamed to just `verify`
* Not saving the default draft is considered as canceling the operation.
* Revamp *advisories* system and add

### Added

* Statically compiled release binaries
* User Documentation, including Getting Started Guide
* `query dir` command
* Differential reviews with `diff` and `review --diff` commands
* New options, particularily for `verify`

## [0.7.0](https://github.com/dpc/crev/compare/cargo-crev-v0.6.0...cargo-crev-v0.7.0) - 2019-04-27
### Added

* Advisories (https://github.com/dpc/crev/wiki/Advisories)
    * `cargo crev advise [name [version]]`
    * `cargo crev query advisory [name [version]]`

## [0.6.0](https://github.com/dpc/crev/compare/cargo-crev-v0.5.0...cargo-crev-v0.6.0) - 2019-04-13
### Changed

- BREAKING: Switch cryptography to standard Ed25519/RFC8032. This will render existing
  IDs and artificates invalid. We're sorry for that. Please recreate your IDs, and use
  `cargo crev import proof` to recreate your reviews.

### Added

- `cargo crev edit config` allows interactive user config edition
- `open-cmd` in user config for customizing `cargo crev open` command
- `cargo crev import proof` for mass-import of proofs

## [0.5.0](https://github.com/dpc/crev/compare/cargo-crev-v0.4.0...cargo-crev-v0.5.0) - 2019-03-06
### Added

- `unsafe` counts via `geiger` crate

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

- Windows cache folder changed from `%AppData%\Local\Dawid Ci,281,,380,arkiewicz\crev` to `%AppData%\Local\crev`.
- Windows config folder changed from `%AppData%\Roaming\Dawid Ci,281,,380,arkiewicz\crev` to `%AppData%\Roaming\crev`.
- MacOS config folder changed from `$HOME/Library/Application Support/crev` to `$HOME/Library/Preferences/crev`.
- Improve `verify deps` names and format.
- Handle error messages better in many places.
- Use host-specific salt in paths of proof files, to prevent dealing with git conflicts when sharing Id between many machines
- Make newer reviews (for the same package and version) effectively overwrite older ones.
- Change `push`, `pull`, `publish` to be more ID-sharing (between hosts) friendly
- Rename `--independent` to `--unrelated` and add `-u` as a short version.
- Avoid fetching things during normal work (helps offline use).
- Hardcode dpc's proof-repo url on `fetch all` to help bootstrap the ecosystem.

### Fixed

- Fix `$EDITOR`/`$VISUAL` handling, especially on Windows
- Save `lanes` in `LockedId`. Old Ids need to be fixed manually.

## [0.3.0] - 2018-12-28

Changelog was not maintained for this and earlier releases
