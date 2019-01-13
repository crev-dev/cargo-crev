# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased](https://github.com/dpc/crev/compare/cargo-crev-v0.3.0...HEAD)
### Added
- This `CHANGELOG.md` file.
- Add ability to work without an Id for most commands
- Add `cargo crev open` command to help IDE users
- Track effective trust level in WoT
- Actually use distrust when calculating WoT
- Add `review` command options: `--print-[un]signed` and `-no-store`
- Add `export` and `import` commands for Ids
- LICENSE files
- New column in `verify deps`: `lines` - line counts using `tokei`
- New column in `verify dpes`: `flags` - Custom Build
- `verify deps` argument: `--for-id`
- Set exit status on `verify deps` to make it usuable in CI
- Display counts of new proofs on `fetch` commands
- Display effecttive trust level on `query id trusted`
- Avoid fetching things during normal work (helps offline use)
- Add new command: `update`
- Hardcode dpc's proof-repo url on `fetch url` to help bootstrap the ecosystem

### Changed
- Windows cache folder changed from `%AppData%\Local\Dawid Ci,281,,380,arkiewicz\crev` to `%AppData%\Local\crev`.
- Windows config folder changed from `%AppData%\Roaming\Dawid Ci,281,,380,arkiewicz\crev` to `%AppData%\Roaming\crev`.
- MacOS config folder changed from `$HOME/Library/Application Support/crev` to `$HOME/Library/Preferences/crev`.
- Better `verify deps` names and format
- Better error messages in many areas
- Use host-specific salt in paths of proof files, to prevent dealing with git conflicts when sharing Id between many machines
- Make newer reviews overwrite older ones (for the same package and version)
- Change `push`, `pull`, `publish` to be more ID-sharing (between hosts) friendly
- `--independent` was renamed to `--unrelated` and has a short version `-u` now

### Fixed
- `$EDITOR`/`$VISUAL` handling, especially on Windows
- Save `lanes` in `LockedId`. Old Ids need to be fixed manually.

## 0.3.0 - 2018-12-28

Changelog was not maintained for this and earlier releases
