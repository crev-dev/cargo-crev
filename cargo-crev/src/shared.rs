// Here are the structs and functions which still need to be sorted
//
use crate::{
    deps::scan::{self, RequiredDetails},
    edit, opts,
    opts::CrateSelector,
    prelude::*,
    repo::Repo,
};
use anyhow::{format_err, Context, Result};
use crev_data::{proof, review::Package, SOURCE_CRATES_IO};
use crev_lib::{self, local::Local, ProofStore, ReviewMode};
use resiter::FlatMap;
use serde::Deserialize;
use std::{
    collections::HashSet,
    env,
    ffi::{OsStr, OsString},
    io,
    path::{Path, PathBuf},
    process,
};

/// Name of ENV with original location `crev goto` was called from
pub const GOTO_ORIGINAL_DIR_ENV: &str = "CARGO_CREV_GOTO_ORIGINAL_DIR";
/// Name of ENV with name of the crate that we've `goto`ed to
pub const GOTO_CRATE_NAME_ENV: &str = "CARGO_CREV_GOTO_ORIGINAL_NAME";
/// Name of ENV with version of the crate that we've `goto`ed to
pub const GOTO_CRATE_VERSION_ENV: &str = "CARGO_CREV_GOTO_ORIGINAL_VERSION";

/// Name of file we store user-personalized
pub const KNOWN_CARGO_OWNERS_FILE: &str = "known_cargo_owners.txt";

/// The file added to crates containing vcs revision
pub const VCS_INFO_JSON_FILE: &str = ".cargo_vcs_info.json";

/// Data from `.cargo_vcs_info.json`
#[derive(Debug, Clone, Deserialize)]
pub struct VcsInfoJson {
    git: VcsInfoJsonGit,
}

pub fn vcs_info_to_revision_string(vcs: Option<VcsInfoJson>) -> String {
    vcs.map(|vcs| vcs.get_git_revision()).unwrap_or_default()
}

#[derive(Debug, Clone, Deserialize)]
pub enum VcsInfoJsonGit {
    #[serde(rename = "sha1")]
    Sha1(String),
}

impl VcsInfoJson {
    fn read_from_crate_dir(pkg_dir: &Path) -> Result<Option<Self>> {
        let path = pkg_dir.join(VCS_INFO_JSON_FILE);

        if path.exists() {
            let txt = std::fs::read_to_string(&path)?;
            let info: VcsInfoJson = serde_json::from_str(&txt)?;
            Ok(Some(info))
        } else {
            Ok(None)
        }
    }
    fn get_git_revision(&self) -> String {
        let VcsInfoJsonGit::Sha1(ref s) = self.git;
        s.to_string()
    }
}

/// Ignore things that are commonly added during the review (eg. by RLS)
pub fn cargo_full_ignore_list(ignore_cargo_lock: bool) -> fnv::FnvHashSet<PathBuf> {
    let mut ignore_list = HashSet::default();
    ignore_list.insert(PathBuf::from(".cargo-ok"));
    ignore_list.insert(PathBuf::from("target"));
    if ignore_cargo_lock {
        ignore_list.insert(PathBuf::from("Cargo.lock"));
    }

    ignore_list
}

/// Ignore only the marker added by `cargo` after fully downloading and extracting crate
pub fn cargo_min_ignore_list() -> fnv::FnvHashSet<PathBuf> {
    let mut ignore_list = HashSet::default();
    ignore_list.insert(PathBuf::from(".cargo-ok"));
    ignore_list
}

#[cfg(target_family = "unix")]
// on Unix we use `exec` so that stuff like Ctrl-C works
// we don't care about destructors at this point
pub fn exec_into(mut command: process::Command) -> Result<()> {
    use std::os::unix::process::CommandExt;
    bail!(command.exec());
}

#[cfg(target_family = "windows")]
// TODO: Is this the way to do it in Windows?
pub fn exec_into(mut command: process::Command) -> Result<()> {
    let status = command.status()?;
    if !status.success() {
        bail!("Shell returned {}", status);
    }
    Ok(())
}

/// `cd` into crate source code and start shell
///
/// Set some `envs` to help other commands work
/// from inside such a "review-shell".
pub fn goto_crate_src(selector: &opts::CrateSelector) -> Result<()> {
    if env::var(GOTO_ORIGINAL_DIR_ENV).is_ok() {
        bail!("You're already in a `cargo crev goto` shell");
    };
    let repo = Repo::auto_open_cwd_default()?;
    selector.ensure_name_given()?;

    let crate_id = repo.find_pkgid_by_crate_selector(selector)?;
    let crate_ = repo.get_crate(&crate_id)?;
    let crate_dir = crate_.root();
    let crate_name = crate_.name();
    let crate_version = crate_.version();
    let local = crev_lib::Local::auto_create_or_open()?;
    local.record_review_activity(
        SOURCE_CRATES_IO,
        &crate_.name(),
        crate_version,
        &crev_lib::ReviewActivity::new_full(),
    )?;

    let shell = env::var_os("SHELL").ok_or_else(|| format_err!("$SHELL not set"))?;
    let cwd = env::current_dir()?;

    eprintln!("Opening shell in: {}", crate_dir.display());
    eprintln!("Use `exit` or Ctrl-D to return to the original project.",);
    eprintln!("Use `cargo crev review` without any arguments to review this crate.");
    let mut command = process::Command::new(shell);
    command
        .current_dir(crate_dir)
        .env("PWD", crate_dir)
        .env(GOTO_ORIGINAL_DIR_ENV, cwd)
        .env(GOTO_CRATE_NAME_ENV, crate_name)
        .env(GOTO_CRATE_VERSION_ENV, &crate_version.to_string());

    exec_into(command)
}

pub fn expand_crate_src(selector: &opts::CrateSelector) -> Result<()> {
    use quote::ToTokens;

    let repo = Repo::auto_open_cwd_default()?;
    selector.ensure_name_given()?;
    let crate_id = repo.find_pkgid_by_crate_selector(selector)?;
    let crate_ = repo.get_crate(&crate_id)?;
    let crate_dir = crate_.root();

    // TODO: discover all the entrypoint, iterate over them, take care of formatting and
    // comments
    println!(
        "{}",
        syn_inline_mod::parse_and_inline_modules(&crate_dir.join("src/lib.rs")).into_token_stream()
    );

    Ok(())
}

pub fn ensure_known_owners_list_exists(local: &crev_lib::Local) -> Result<()> {
    let path = local.get_proofs_dir_path()?.join(KNOWN_CARGO_OWNERS_FILE);
    if !path.exists() {
        crev_common::store_str_to_file(&path, include_str!("known_cargo_owners_defaults.txt"))?;
        local.proof_dir_git_add_path(&PathBuf::from(KNOWN_CARGO_OWNERS_FILE))?;
    }

    Ok(())
}

pub fn read_known_owners_list() -> Result<HashSet<String>> {
    let local = Local::auto_create_or_open()?;
    let content = if let Some(path) = local.get_proofs_dir_path_opt()? {
        let path = path.join(KNOWN_CARGO_OWNERS_FILE);
        std::fs::read_to_string(path)?
    } else {
        include_str!("known_cargo_owners_defaults.txt").to_string()
    };
    Ok(content
        .lines()
        .map(str::trim)
        .filter(|s| !s.starts_with('#'))
        .map(ToString::to_string)
        .collect())
}

pub fn edit_known_owners_list() -> Result<()> {
    let local = Local::auto_create_or_open()?;
    let path = local.get_proofs_dir_path()?.join(KNOWN_CARGO_OWNERS_FILE);
    ensure_known_owners_list_exists(&local)?;
    edit::edit_file(&path)?;
    Ok(())
}

pub fn clean_all_crates_with_digest_mismatch() -> Result<()> {
    let scanner = scan::Scanner::new(CrateSelector::default(), &opts::CrateVerify::default())?;
    let events = scanner.run(&RequiredDetails::none());

    for stats in events {
        if stats.details.accumulative_own.is_local_source_code {
            continue;
        }
        if stats.has_digest_mismatch() {
            clean_crate(&CrateSelector::new(
                Some(stats.info.id.name().to_string()),
                Some(stats.info.id.version().clone()),
                false,
            ))?;
        }
    }

    Ok(())
}

/// Wipe the crate source, then re-download it
pub fn clean_crate(selector: &CrateSelector) -> Result<()> {
    let repo = Repo::auto_open_cwd_default()?;
    let crate_id = repo.find_pkgid_by_crate_selector(selector)?;
    let crate_ = repo.get_crate(&crate_id)?;
    assert!(crate_.package_id().source_id().is_registry());
    let crate_root = crate_.root();

    assert!(crate_root.is_absolute());
    assert!(
        !std::env::current_dir()?.starts_with(crate_root),
        "Cannot delete {} while inside that directory.",
        crate_root.display(),
    );

    if crate_root.is_dir() {
        std::fs::remove_dir_all(crate_root)?;
    }
    let _crate_ = repo.get_crate(&crate_id)?;
    Ok(())
}

pub fn get_open_cmd(local: &Local) -> Result<String> {
    let config = local
        .load_user_config()
        .with_context(|| "Can't open user config")?;
    if let Some(cmd) = config.open_cmd {
        return Ok(cmd);
    }

    Ok(if cfg!(target_os = "windows") {
        "start"
    } else if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "linux") {
        "xdg-open"
    } else {
        eprintln!("Unsupported platform. Please submit a PR!");
        "xdg-open"
    }
    .into())
}

/// Open a crate
///
/// * `unrelated` - the crate might not actually be a dependency
pub fn crate_open(crate_sel: &CrateSelector, cmd: Option<String>, cmd_save: bool) -> Result<()> {
    let local = Local::auto_create_or_open()?;
    let repo = Repo::auto_open_cwd_default()?;
    let crate_id = repo.find_pkgid_by_crate_selector(crate_sel)?;
    let cargo_crate = repo.get_crate(&crate_id)?;

    if cmd_save {
        if let Some(cmd) = &cmd {
            local.store_config_open_cmd(cmd.clone())?;
        } else {
            bail!("Can't save cmd without specifying it");
        }
    }

    let name = cargo_crate.name().to_string();
    let version = cargo_crate.version();
    let src_dir = cargo_crate.root();

    // It's not safe to open Cargo's crate dir directly, because editor integration (like cargo check)
    // could automatically start running crate's potentially malicious build script or proc macros.
    let dest_dir = local.sanitized_crate_copy(SOURCE_CRATES_IO, &name, version, src_dir)?;

    let open_cmd = match cmd {
        Some(cmd) => cmd,
        None => get_open_cmd(&local)?,
    };
    local.record_review_activity(
        SOURCE_CRATES_IO,
        &name,
        version,
        &crev_lib::ReviewActivity::new_full(),
    )?;
    let status = crev_lib::util::run_with_shell_cmd(open_cmd.as_ref(), Some(&dest_dir))?;

    if !status.success() {
        bail!("Shell returned {}", status);
    }

    Ok(())
}

/*
#[derive(Copy, Clone, PartialEq, Eq)]
/// Do you select a dependency of the current project
/// or just a random, unrelated crate.
pub enum UnrelatedOrDependency {
    Unrelated,
    Dependency,
}

impl UnrelatedOrDependency {
    pub fn is_unrelated(self) -> bool {
        self == UnrelatedOrDependency::Unrelated
    }

    pub fn from_unrelated_flag(u: bool) -> Self {
        if u {
            UnrelatedOrDependency::Unrelated
        } else {
            UnrelatedOrDependency::Dependency
        }
    }
}*/

#[derive(Debug, thiserror::Error)]
pub enum ActivityCheckError {
    #[error("No previous review activity to determine base version")]
    NoPreviousReview,
    #[error("Last review activity record indicates full review")]
    UnexpectedFullReview,
    #[error("Last review activity record indicates differential review")]
    UnexpectedDiffReview,
    #[error("Last review activity record is too old")]
    Expired,
    #[error("No review activity record found")]
    NoRecord,
    #[error("{}", _0)]
    Other(#[from] crev_lib::Error),
}

/// Check `diff` command line argument against previous activity
///
/// Return `Option<Version>` indicating final `ReviewMode` settings to use.
pub fn crate_review_activity_check(
    local: &Local,
    name: &str,
    version: &Version,
    diff: &Option<Option<Version>>,
    skip_activity_check: bool,
) -> Result<Option<Version>, ActivityCheckError> {
    let activity = local.read_review_activity(SOURCE_CRATES_IO, name, version)?;

    let diff = match diff {
        None => None,
        Some(None) => Some(
            activity
                .clone()
                .ok_or(ActivityCheckError::NoPreviousReview)?
                .diff_base
                .ok_or(ActivityCheckError::UnexpectedFullReview)?,
        ),
        Some(o) => o.clone(),
    };

    if skip_activity_check {
        return Ok(diff);
    }
    if let Some(activity) = activity {
        match activity.to_review_mode() {
            ReviewMode::Full => {
                if diff.is_some() {
                    return Err(ActivityCheckError::UnexpectedFullReview);
                }
            }
            ReviewMode::Differential => {
                if diff.is_none() {
                    return Err(ActivityCheckError::UnexpectedDiffReview);
                }
            }
        }

        if activity.timestamp + chrono::Duration::days(2) < crev_common::now() {
            return Err(ActivityCheckError::Expired);
        }
    } else {
        return Err(ActivityCheckError::NoRecord);
    }

    Ok(diff)
}

pub fn check_package_clean_state(
    repo: &Repo,
    crate_root: &Path,
    name: &str,
    version: &Version,
) -> Result<(crev_data::Digest, Option<VcsInfoJson>)> {
    // to protect from creating a digest from a crate in unclean state
    // we move the old directory, download a fresh one and double
    // check if the digest was the same
    // BUG: TODO: https://users.rust-lang.org/t/append-an-additional-extension/23586
    let reviewed_pkg_dir: PathBuf =
        crev_common::fs::append_to_path(crate_root.to_owned(), ".crev.reviewed");
    if reviewed_pkg_dir.is_dir() {
        std::fs::remove_dir_all(&reviewed_pkg_dir)?;
    }

    // to prevent user calling `crev review` from the source dir,
    // having the cwd pulled from under them and confusing their
    // shells, we move all the entries in a dir, instead of the whole
    // dir. this is not a perfect solution, but better than nothing.
    crev_common::fs::move_dir_content(crate_root, &reviewed_pkg_dir)?;
    let crate_second_id = repo.find_pkgid(name, Some(version), true)?;
    let crate_second = repo.get_crate(&crate_second_id)?;
    let crate_root_second = crate_second.root();
    let crate_version_second = crate_second.version();

    assert_eq!(crate_root, crate_root_second);
    assert_eq!(version, crate_version_second);

    let digest_clean =
        crev_lib::get_recursive_digest_for_dir(crate_root, &cargo_min_ignore_list())?;
    // if the `Cargo.lock` not exist in the clean package, we can ignore it being created
    // in the reviewed version; that's normal
    let ignore_cargo_lock = !crate_root.join("Cargo.lock").exists();
    let digest_reviewed = crev_lib::get_recursive_digest_for_dir(
        &reviewed_pkg_dir,
        &cargo_full_ignore_list(ignore_cargo_lock),
    )?;

    if digest_clean != digest_reviewed {
        eprintln!(
            r#"The digest of the reviewed source code is different from the digest of a freshly downloaded copy.
            This is most probably caused by your actions: creating new or modified files.
            Will continue with the digest of a fresh copy.
            Fresh copy: {}
            Reviewed code: {}."#,
            crate_root.display(),
            reviewed_pkg_dir.display(),
        );
    } else {
        std::fs::remove_dir_all(&reviewed_pkg_dir)?;
    }

    let vcs = VcsInfoJson::read_from_crate_dir(crate_root)?;

    Ok((digest_clean, vcs))
}

pub fn find_advisories(crate_: &opts::CrateSelector) -> Result<Vec<proof::review::Package>> {
    let local = crev_lib::Local::auto_open()?;
    let db = local.load_db()?;

    Ok(db
        .get_advisories(SOURCE_CRATES_IO, crate_.name.as_deref(), crate_.version()?)
        .cloned()
        .collect())
}

pub fn run_diff(args: &opts::Diff) -> Result<std::process::ExitStatus> {
    let repo = Repo::auto_open_cwd_default()?;
    let name = &args.name;

    let dst_version = &args.dst;
    let dst_crate_id = repo.find_pkgid(name, dst_version.clone().as_ref(), args.unrelated)?;
    let dst_crate = repo.get_crate(&dst_crate_id)?;

    let requirements = crev_lib::VerificationRequirements::from(args.requirements.clone());
    let trust_distance_params = &args.trust_params.clone().into();

    let local = crev_lib::Local::auto_create_or_open()?;
    let current_id = local.get_current_userid()?;
    let db = local.load_db()?;
    let trust_set = db.calculate_trust_set(&current_id, trust_distance_params);
    let src_version = args
        .src
        .clone()
        .or_else(|| {
            crev_lib::find_latest_trusted_version(
                &trust_set,
                SOURCE_CRATES_IO,
                name,
                &requirements,
                &db,
            )
        })
        .ok_or_else(|| format_err!("No previously reviewed version found"))?;
    let src_crate_id = repo.find_pkgid(name, Some(&src_version), true)?;
    let src_crate = repo.get_crate(&src_crate_id)?;

    local.record_review_activity(
        SOURCE_CRATES_IO,
        name,
        dst_crate.version(),
        &crev_lib::ReviewActivity::new_diff(&src_version),
    )?;

    use std::process::Command;

    let diff = |exe| {
        let mut command = Command::new(exe);
        command
            .arg("-r")
            .arg("-N")
            .arg(src_crate.root())
            .arg(dst_crate.root())
            .args(&args.args);
        command
    };

    let mut command = diff(OsStr::new("diff"));

    match command.status() {
        Err(ref err) if err.kind() == io::ErrorKind::NotFound && cfg!(windows) => {
            // On Windows, diff is likely available but *not* in %PATH.  Specifically, the git installer warns that
            // adding *nix tools to %PATH% will change the behavior of some built in windows commands like "find", and
            // by default doesn't do this to avoid breaking anything.

            let program_files =
                env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_owned());
            let mut diff_exe = PathBuf::from(program_files);
            diff_exe.push(r"Git\usr\bin\diff.exe");

            let mut command = diff(diff_exe.as_os_str());
            command
                .status()
                .map_err(|err| panic!("Failed to execute {command:?}\n{err:?}"))
        }
        Err(ref err) => panic!("Failed to execute {command:?}\n{err:?}"),
        Ok(status) => Ok(status),
    }
}

pub fn show_dir(sel: &opts::CrateSelector) -> Result<()> {
    let repo = Repo::auto_open_cwd_default()?;

    sel.ensure_name_given()?;
    let crate_id = repo.find_pkgid_by_crate_selector(sel)?;
    let crate_ = repo.get_crate(&crate_id)?;
    println!("{}", crate_.root().display());

    Ok(())
}

pub fn list_advisories(crate_: &opts::CrateSelector) -> Result<()> {
    for review in find_advisories(crate_)? {
        println!("---\n{review}");
    }

    Ok(())
}

pub fn list_issues(args: &opts::RepoQueryIssue) -> Result<()> {
    let trust_distance_params = args.trust_params.clone().into();

    let local = crev_lib::Local::auto_open()?;
    let current_id = local.get_current_userid()?;
    let db = local.load_db()?;
    let trust_set = db.calculate_trust_set(&current_id, &trust_distance_params);

    for review in db.get_pkg_reviews_with_issues_for(
        SOURCE_CRATES_IO,
        args.crate_.name.as_deref(),
        args.crate_.version()?,
        &trust_set,
        args.trust_level.into(),
    ) {
        println!("---\n{review}");
    }

    Ok(())
}

/// Are we executing from a shell started by `cargo crev goto`?
///
/// If yes - return the path the original directory where the
/// `goto` command was issued.
pub fn are_we_called_from_goto_shell() -> Option<OsString> {
    env::var_os(GOTO_ORIGINAL_DIR_ENV)
}

/// Handle the `goto mode` commands
///
/// After jumping to a crate with `goto`, the crate is selected
/// already, and commands like `review` must not be given any arguments
/// like that.
pub fn handle_goto_mode_command<F>(args: &opts::ReviewOrGotoCommon, f: F) -> Result<()>
where
    F: FnOnce(&CrateSelector) -> Result<()>,
{
    if let Some(org_dir) = are_we_called_from_goto_shell() {
        if args.crate_.name.is_some() {
            bail!("In `crev goto` mode no arguments can be given");
        } else {
            let name = env::var(GOTO_CRATE_NAME_ENV)
                .map_err(|_| format_err!("crate name env var not found"))?;
            let version = env::var(GOTO_CRATE_VERSION_ENV)
                .map_err(|_| format_err!("crate version env var not found"))?;

            env::set_current_dir(org_dir)?;
            f(&CrateSelector::new(
                Some(name),
                Some(Version::parse(&version)?),
                true,
            ))?;
        }
    } else {
        args.crate_.ensure_name_given()?;

        f(&args.crate_)?;
    }
    Ok(())
}

pub fn is_file_with_ext(entry: &walkdir::DirEntry, file_ext: &str) -> bool {
    if !entry.file_type().is_file() {
        return false;
    }
    entry
        .path()
        .extension()
        .map_or(false, |ext| ext.to_string_lossy().as_ref() == file_ext)
}

pub fn iter_rs_files_in_dir(dir: &Path) -> impl Iterator<Item = Result<PathBuf>> {
    let walker = walkdir::WalkDir::new(dir).into_iter();
    walker
        .map(|entry| {
            let entry = entry?;
            if !is_file_with_ext(&entry, "rs") {
                return Ok(None);
            }
            Ok(Some(entry.path().canonicalize()?))
        })
        .filter_map(|res| res.transpose())
}

// Note: this function is very slow
pub fn get_geiger_count(path: &Path) -> Result<u64> {
    let mut count = 0;
    for metrics in iter_rs_files_in_dir(path)
        .flat_map_ok(|path| geiger::find::find_unsafe_in_file(&path, geiger::IncludeTests::No))
    {
        let counters = metrics?.counters;
        count += counters.functions.unsafe_
            + counters.exprs.unsafe_
            + counters.item_impls.unsafe_
            + counters.item_traits.unsafe_
            + counters.methods.unsafe_;
    }

    Ok(count)
}

/// Result of `run_command`
///
/// This is to distinguish expected non-success results,
/// from errors: unexpected failures.
#[must_use]
pub enum CommandExitStatus {
    // `verify deps` failed
    VerificationFailed,
    // Success, exit code 0
    Success,
    CommandExitCode(i32),
}

pub fn get_crate_digest_mismatches(
    db: &crev_wot::ProofDB,
    name: &str,
    version: &Version,
    digest: &crev_data::Digest,
) -> Vec<Package> {
    db.get_package_reviews_for_package(SOURCE_CRATES_IO, Some(name), Some(version))
        .filter(|review| review.package.digest != digest.as_slice())
        .cloned()
        .collect()
}

pub fn maybe_store(
    local: &Local,
    proof: &crev_data::proof::Proof,
    commit_msg: &str,
    proof_create_opt: &opts::CommonProofCreate,
) -> Result<()> {
    if proof_create_opt.print_unsigned {
        print!("{}", proof.body());
    }

    if proof_create_opt.print_signed {
        print!("{proof}");
    }

    if !proof_create_opt.no_store {
        local.insert(proof)?;

        if !proof_create_opt.no_commit {
            local
                .proof_dir_commit(commit_msg)
                .with_context(|| "Could not not automatically commit")?;
        }
    }

    Ok(())
}

pub fn lookup_crates(query: &str, count: usize) -> Result<()> {
    struct CrateStats {
        name: String,
        downloads: u64,
        proof_count: usize,
    }

    use crates_io_api::{CratesQuery, Sort, SyncClient};

    let local = crev_lib::Local::auto_create_or_open()?;
    let db = local.load_db()?;

    let client = SyncClient::new("cargo-crev", std::time::Duration::from_secs(1))?;
    let mut stats: Vec<_> = client
        .crates(
            CratesQuery::builder()
                .sort(Sort::Downloads)
                .page_size(100)
                .search(query.to_string())
                .build(),
        )?
        .crates
        .iter()
        .map(|crate_| CrateStats {
            name: crate_.name.clone(),
            downloads: crate_.downloads,
            proof_count: db.get_package_review_count(SOURCE_CRATES_IO, Some(&crate_.name), None),
        })
        .collect();

    stats.sort_by(|a, b| {
        a.proof_count
            .cmp(&b.proof_count)
            .then(a.downloads.cmp(&b.downloads))
            .reverse()
    });

    for stats in stats.iter().take(count) {
        println!("{:8} {}", stats.proof_count, stats.name);
    }

    Ok(())
}
