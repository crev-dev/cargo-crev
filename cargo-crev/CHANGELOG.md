# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2019-01-12
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
