// Here are the things I can't let in main but that I don't
// know yet where to put (or what they are)

use crev_lib::{self, local::Local, ProofStore, ReviewMode};
use failure::format_err;
use insideout::InsideOutIter;
use resiter::FlatMap;
use serde::Deserialize;
use std::{
    collections::{HashSet},
    default::Default,
    env,
    path::{Path, PathBuf},
    process,
};

use crate::prelude::*;
use crate::opts;
use crate::repo::*;
use crev_data::proof;
use crev_lib::TrustOrDistrust;

/// Name of ENV with original location `crev goto` was called from
pub const GOTO_ORIGINAL_DIR_ENV: &str = "CARGO_CREV_GOTO_ORIGINAL_DIR";
/// Name of ENV with name of the crate that we've `goto`ed to
pub const GOTO_CRATE_NAME_ENV: &str = "CARGO_CREV_GOTO_ORIGINAL_NAME";
/// Name of ENV with version of the crate that we've `goto`ed to
pub const GOTO_CRATE_VERSION_ENV: &str = "CARGO_CREV_GOTO_ORIGINAL_VERSION";

/// Name of file we store user-personalized
pub const KNOWN_CARGO_OWNERS_FILE: &str = "known_cargo_owners.txt";

/// Constant we use for `source` in the review proof
pub const PROJECT_SOURCE_CRATES_IO: &str = "https://crates.io";

/// The file added to crates containing vcs revision
pub const VCS_INFO_JSON_FILE: &str = ".cargo_vcs_info.json";

pub fn pad_left_manually(s: String, width: usize) -> String {
    if s.len() <= width {
        let padding = std::iter::repeat(" ")
            .take(width - s.len())
            .collect::<String>();
        format!("{}{}", s, padding)
    } else {
        s
    }
}

pub fn latest_trusted_version_string(
    base_version: Version,
    latest_trusted_version: Option<Version>,
) -> String {
    latest_trusted_version
        .map(|latest_trusted_version| {
            // there seems to be a big bug in termimad or crossterm in some cases
            // with multibytes characters
            let ch = if base_version < latest_trusted_version {
                ">" //"↑"
            } else if latest_trusted_version < base_version {
                "<"// "↓"
            } else {
                "="
            };
            format!("{}{}", ch, latest_trusted_version)
        })
        .unwrap_or_else(|| "".into())
}

/// Data from `.cargo_vcs_info.json`
#[derive(Debug, Clone, Deserialize)]
pub struct VcsInfoJson {
    git: VcsInfoJsonGit,
}

pub fn vcs_info_to_revision_string(vcs: Option<VcsInfoJson>) -> String {
    vcs.and_then(|vcs| vcs.get_git_revision())
        .unwrap_or_else(|| "".into())
}
#[derive(Debug, Clone, Deserialize)]
enum VcsInfoJsonGit {
    #[serde(rename = "sha1")]
    Sha1(String),
}

impl VcsInfoJson {
    pub fn read_from_crate_dir(pkg_dir: &Path) -> Result<Option<Self>> {
        let path = pkg_dir.join(VCS_INFO_JSON_FILE);

        if path.exists() {
            let txt = crev_common::read_file_to_string(&path)?;
            let info: VcsInfoJson = serde_json::from_str(&txt)?;
            Ok(Some(info))
        } else {
            Ok(None)
        }
    }
    pub fn get_git_revision(&self) -> Option<String> {
        let VcsInfoJsonGit::Sha1(ref s) = self.git;
        Some(s.to_string())
    }
}


/// Ignore things that are commonly added during the review (eg. by RLS)
pub fn cargo_full_ignore_list() -> HashSet<PathBuf> {
    let mut ignore_list = HashSet::new();
    ignore_list.insert(PathBuf::from(".cargo-ok"));
    ignore_list.insert(PathBuf::from("Cargo.lock"));
    ignore_list.insert(PathBuf::from("target"));
    ignore_list
}

/// Ignore only the marker added by `cargo` after fully downloading and extracting crate
pub fn cargo_min_ignore_list() -> HashSet<PathBuf> {
    let mut ignore_list = HashSet::new();
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
pub fn goto_crate_src(selector: &opts::CrateSelector, unrelated: UnrelatedOrDependency) -> Result<()> {
    if env::var(GOTO_ORIGINAL_DIR_ENV).is_ok() {
        bail!("You're already in a `cargo crev goto` shell");
    };
    let repo = Repo::auto_open_cwd()?;
    let name = selector
        .name
        .clone()
        .ok_or_else(|| format_err!("Crate name argument required"))?;
    let crate_ = repo.find_crate(&name, selector.version.as_ref(), unrelated)?;
    let crate_dir = crate_.root();
    let crate_version = crate_.version();
    let local = crev_lib::Local::auto_create_or_open()?;
    local.record_review_activity(
        PROJECT_SOURCE_CRATES_IO,
        &crate_.name().to_string(),
        crate_version,
        &crev_lib::ReviewActivity::new_full(),
    )?;

    let shell = env::var_os("SHELL").ok_or_else(|| format_err!("$SHELL not set"))?;
    let cwd = env::current_dir()?;

    eprintln!("Opening shell in: {}", crate_dir.display());
    eprintln!("Use `exit` or Ctrl-D to return to the original project.",);
    eprintln!("Use `review` and `flag` without any arguments to review this crate.");
    let mut command = process::Command::new(shell);
    command
        .current_dir(crate_dir)
        .env("PWD", crate_dir)
        .env(GOTO_ORIGINAL_DIR_ENV, cwd)
        .env(GOTO_CRATE_NAME_ENV, name)
        .env(GOTO_CRATE_VERSION_ENV, &crate_version.to_string());

    exec_into(command)
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
        crev_common::read_file_to_string(&path)?
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
    crev_lib::util::edit_file(&path)?;
    Ok(())
}

/// Wipe the crate source, then re-download it
pub fn clean_crate(
    name: &str,
    version: Option<&Version>,
    unrelated: UnrelatedOrDependency,
) -> Result<()> {
    let repo = Repo::auto_open_cwd()?;
    let crate_ = repo.find_crate(name, version, unrelated)?;
    let crate_root = crate_.root();

    assert!(!crate_root.starts_with(std::env::current_dir()?));

    if crate_root.is_dir() {
        std::fs::remove_dir_all(&crate_root)?;
    }
    let _crate = repo.find_crate(name, version, unrelated)?;
    Ok(())
}

pub fn get_open_cmd(local: &Local) -> Result<String> {
    let config = local
        .load_user_config()
        .with_context(|_err| "Can't open user config")?;
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
pub fn crate_open(
    name: &str,
    version: Option<&Version>,
    unrelated: UnrelatedOrDependency,
    cmd: Option<String>,
    cmd_save: bool,
) -> Result<()> {
    let local = Local::auto_create_or_open()?;
    let repo = Repo::auto_open_cwd()?;
    let crate_ = repo.find_crate(name, version, unrelated)?;

    let crate_root = crate_.root();

    if cmd_save && cmd.is_none() {
        bail!("Can't save cmd without specifing it");
    }

    let open_cmd = if let Some(cmd) = cmd {
        if cmd_save {
            local.store_config_open_cmd(cmd.clone())?;
        }
        cmd
    } else {
        get_open_cmd(&local)?
    };
    local.record_review_activity(
        PROJECT_SOURCE_CRATES_IO,
        &crate_.name().to_string(),
        &crate_.version(),
        &crev_lib::ReviewActivity::new_full(),
    )?;
    let status = crev_lib::util::run_with_shell_cmd(open_cmd.into(), crate_root)?;

    if !status.success() {
        bail!("Shell returned {}", status);
    }

    Ok(())
}

#[derive(Copy, Clone, PartialEq, Eq)]
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
}

/// Check `diff` command line argument against previous activity
///
/// Return `Option<Version>` indicating final ReviewMode settings to use.
pub fn crate_review_activity_check(
    local: &Local,
    name: &str,
    version: &Version,
    diff: &Option<Option<Version>>,
    skip_activity_check: bool,
) -> Result<Option<Version>> {
    let activity = local.read_review_activity(PROJECT_SOURCE_CRATES_IO, name, version)?;

    let diff = match diff {
        None => None,
        Some(None) => Some(
            activity
                .clone()
                .ok_or_else(|| {
                    format_err!("No previous review activity to determine base version")
                })?
                .diff_base
                .ok_or_else(|| {
                    format_err!(
                        "Last review activity record for {}:{} indcates full review. \
                         Are you sure you want to use `--diff` flag? \
                         Use `--skip-activity-check` to override.",
                        name,
                        version
                    )
                })?,
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
                    bail!(
                        "Last review activity record for {}:{} indcates full review. \
                         Are you sure you want to use `--diff` flag? \
                         Use `--skip-activity-check` to override.",
                        name,
                        version
                    );
                }
            }
            ReviewMode::Differential => {
                if !diff.is_some() {
                    bail!(
                        "Last review activity record for {}:{} indicates differential review. \
                         Use `--diff` flag? Use `--skip-activity-check` to override.",
                        name,
                        version
                    );
                }
            }
        }

        if activity.timestamp + time::Duration::days(2) < crev_common::now() {
            bail!(
                "Last review activity record for {}:{} is too old. \
                 Re-review or use `--skip-activity-check` to override.",
                name,
                version
            );
        }
    } else {
        bail!(
            "No review activity record for {}:{} found. \
             Make sure you have reviewed the code in this version before creating review proof. \
             Use `--skip-activity-check` to override.",
            name,
            version
        );
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
    crev_common::fs::move_dir_content(&crate_root, &reviewed_pkg_dir)?;
    let crate_second = repo.find_crate(name, Some(version), UnrelatedOrDependency::Unrelated)?;
    let crate_root_second = crate_second.root();
    let crate_version_second = crate_second.version();

    assert_eq!(crate_root, crate_root_second);
    assert_eq!(version, crate_version_second);

    let digest_clean =
        crev_lib::get_recursive_digest_for_dir(&crate_root, &cargo_min_ignore_list())?;
    let digest_reviewed =
        crev_lib::get_recursive_digest_for_dir(&reviewed_pkg_dir, &cargo_full_ignore_list())?;

    if digest_clean != digest_reviewed {
        bail!(
            "The digest of the reviewed and freshly downloaded crate were different; {} != {}; {} != {}",
            digest_clean,
            digest_reviewed,
            crate_root.display(),
            reviewed_pkg_dir.display(),
        );
    }
    std::fs::remove_dir_all(&reviewed_pkg_dir)?;

    let vcs = VcsInfoJson::read_from_crate_dir(&crate_root)?;

    Ok((digest_clean, vcs))
}

pub fn find_previous_review_data(
    db: &crev_lib::ProofDB,
    id: &crev_data::PubId,
    name: &str,
    crate_version: &Version,
    diff_base_version: &Option<Version>,
) -> Option<(
    Option<crev_data::proof::Date>,
    crev_data::proof::review::Review,
    Option<crev_data::proof::review::package::Advisory>,
    String,
)> {
    if let Some(previous_review) =
        db.get_package_review_by_author(PROJECT_SOURCE_CRATES_IO, name, crate_version, &id.id)
    {
        return Some((
            Some(previous_review.date),
            previous_review.review,
            previous_review.advisory,
            previous_review.comment,
        ));
    } else if let Some(diff_base_version) = diff_base_version {
        if let Some(base_review) = db.get_package_review_by_author(
            PROJECT_SOURCE_CRATES_IO,
            name,
            &diff_base_version,
            &id.id,
        ) {
            return Some((None, base_review.review, None, base_review.comment));
        }
    }
    None
}

/// Review a crate
///
/// * `unrelated` - the crate might not actually be a dependency
pub fn create_review_proof(
    name: &str,
    version: Option<&Version>,
    unrelated: UnrelatedOrDependency,
    advise_common: Option<opts::AdviseCommon>,
    trust: TrustOrDistrust,
    proof_create_opt: &opts::CommonProofCreate,
    diff_version: &Option<Option<Version>>,
    skip_activity_check: bool,
) -> Result<()> {
    let repo = Repo::auto_open_cwd()?;

    let crate_ = repo.find_crate(name, version, unrelated)?;
    let crate_root = crate_.root();
    let effective_crate_version = crate_.version();

    assert!(!crate_root.starts_with(std::env::current_dir()?));
    let local = Local::auto_open()?;

    let diff_base_version = crate_review_activity_check(
        &local,
        name,
        &effective_crate_version,
        &diff_version,
        skip_activity_check,
    )?;

    let (digest_clean, vcs) =
        check_package_clean_state(&repo, &crate_root, name, &effective_crate_version)?;

    let diff_base = if let Some(ref diff_base_version) = diff_base_version {
        let crate_ = repo.find_crate(
            name,
            Some(diff_base_version),
            UnrelatedOrDependency::Unrelated,
        )?;
        let crate_root = crate_.root();

        let (digest, vcs) =
            check_package_clean_state(&repo, &crate_root, name, &diff_base_version)?;

        Some(proof::PackageInfo {
            id: None,
            source: PROJECT_SOURCE_CRATES_IO.to_owned(),
            name: name.to_owned(),
            version: diff_base_version.to_owned(),
            digest: digest.into_vec(),
            digest_type: proof::default_digest_type(),
            revision: vcs_info_to_revision_string(vcs),
            revision_type: proof::default_revision_type(),
        })
    } else {
        None
    };

    let id = local.read_current_unlocked_id(&crev_common::read_passphrase)?;

    let db = local.load_db()?;
    let mut review = proof::review::PackageBuilder::default()
        .from(id.id.to_owned())
        .package(proof::PackageInfo {
            id: None,
            source: PROJECT_SOURCE_CRATES_IO.to_owned(),
            name: name.to_owned(),
            version: effective_crate_version.to_owned(),
            digest: digest_clean.into_vec(),
            digest_type: proof::default_digest_type(),
            revision: vcs_info_to_revision_string(vcs),
            revision_type: proof::default_revision_type(),
        })
        .review(if advise_common.is_some() {
            crev_data::Review::new_none()
        } else {
            trust.to_review()
        })
        .diff_base(diff_base)
        .build()
        .map_err(|e| format_err!("{}", e))?;

    let previous_date = if let Some((prev_date, prev_review, prev_advisory, prev_comment)) =
        find_previous_review_data(
            &db,
            &id.id,
            name,
            effective_crate_version,
            &diff_base_version,
        ) {
        review.review = prev_review;
        review.comment = prev_comment;
        if let Some(prev_advisory) = prev_advisory {
            review.advisory = Some(prev_advisory);
        }
        prev_date
    } else {
        None
    };

    if review.advisory.is_none() {
        if let Some(advise_common) = advise_common {
            let mut advisory: proof::review::package::Advisory = advise_common.affected.into();
            advisory.critical = advise_common.critical;
            review.advisory = Some(advisory);
        }
    }
    let review = crev_lib::util::edit_proof_content_iteractively(
        &review.into(),
        previous_date.as_ref(),
        diff_base_version.as_ref(),
    )?;

    let proof = review.sign_by(&id)?;

    let commit_msg = format!(
        "Add review for {crate} v{version}",
        crate = name,
        version = effective_crate_version
    );
    maybe_store(&local, &proof, &commit_msg, proof_create_opt)
}

pub fn maybe_store(
    local: &Local,
    proof: &crev_data::proof::Proof,
    commit_msg: &str,
    proof_create_opt: &opts::CommonProofCreate,
) -> Result<()> {
    if proof_create_opt.print_unsigned {
        print!("{}", proof.body);
    }

    if proof_create_opt.print_signed {
        print!("{}", proof);
    }

    if !proof_create_opt.no_store {
        local.insert(&proof)?;

        if !proof_create_opt.no_commit {
            local
                .proof_dir_commit(&commit_msg)
                .with_context(|_| format_err!("Could not not automatically commit"))?;
        }
    }

    Ok(())
}

pub fn find_reviews(
    crate_: &opts::CrateSelector,
) -> Result<impl Iterator<Item = proof::review::Package>> {
    let local = crev_lib::Local::auto_open()?;
    let db = local.load_db()?;
    Ok(db.get_package_reviews_for_package(
        PROJECT_SOURCE_CRATES_IO,
        crate_.name.as_ref().map(String::as_str),
        crate_.version.as_ref(),
    ))
}

pub fn list_reviews(crate_: &opts::CrateSelector) -> Result<()> {
    for review in find_reviews(crate_)? {
        println!("{}", review);
    }

    Ok(())
}

pub fn find_advisories(
    crate_: &opts::CrateSelector,
) -> Result<impl Iterator<Item = (Version, proof::review::Package)>> {
    let local = crev_lib::Local::auto_open()?;
    let db = local.load_db()?;

    Ok(db
        .get_advisories(
            PROJECT_SOURCE_CRATES_IO,
            crate_.name.as_ref().map(String::as_str),
            crate_.version.as_ref(),
        )
        .into_iter())
}

pub fn run_diff(args: &opts::Diff) -> Result<std::process::ExitStatus> {
    let repo = Repo::auto_open_cwd()?;
    let name = &args.name;

    let dst_version = &args.dst;
    let dst_crate = repo.find_crate(
        &name,
        dst_version.as_ref(),
        UnrelatedOrDependency::Unrelated,
    )?;

    let requirements = crev_lib::VerificationRequirements::from(args.requirements.clone());
    let trust_distance_params = &args.trust_params.clone().into();

    let local = crev_lib::Local::auto_create_or_open()?;
    let current_id = local.get_current_userid()?;
    let db = local.load_db()?;
    let trust_set = db.calculate_trust_set(&current_id, &trust_distance_params);
    let src_version = args
        .src
        .clone()
        .or_else(|| {
            db.find_latest_trusted_version(
                &trust_set,
                PROJECT_SOURCE_CRATES_IO,
                &name,
                &requirements,
            )
        })
        .ok_or_else(|| format_err!("No previously reviewed version found"))?;
    let src_crate = repo.find_crate(&name, Some(&src_version), UnrelatedOrDependency::Unrelated)?;

    local.record_review_activity(
        PROJECT_SOURCE_CRATES_IO,
        &name,
        &dst_crate.version(),
        &crev_lib::ReviewActivity::new_diff(&src_version),
    )?;

    use std::process::Command;

    let status = Command::new("diff")
        .arg("-r")
        .arg("-N")
        .arg(src_crate.root())
        .arg(dst_crate.root())
        .args(&args.args)
        .status()
        .expect("failed to execute git");

    Ok(status)
}

pub fn show_dir(crate_: &opts::CrateSelector, unrelated: UnrelatedOrDependency) -> Result<()> {
    let repo = Repo::auto_open_cwd()?;
    let name = crate_
        .name
        .clone()
        .ok_or_else(|| format_err!("Crate name argument required"))?;
    let crate_ = repo.find_crate(&name, crate_.version.as_ref(), unrelated)?;
    println!("{}", crate_.root().display());

    Ok(())
}

pub fn list_advisories(crate_: &opts::CrateSelector) -> Result<()> {
    for (_, review) in find_advisories(crate_)? {
        println!("{}", review);
    }

    Ok(())
}
/// Handle the `goto mode` commands
///
/// After jumping to a crate with `goto`, the crate is selected
/// already, and commands like `review` must not be given any arguments
/// like that.
pub fn handle_goto_mode_command<F>(args: &opts::ReviewOrGotoCommon, f: F) -> Result<()>
where
    F: FnOnce(&str, Option<&Version>, UnrelatedOrDependency) -> Result<()>,
{
    if let Some(org_dir) = env::var_os(GOTO_ORIGINAL_DIR_ENV) {
        if args.crate_.name.is_some() {
            bail!("In `crev goto` mode no arguments can be given");
        } else {
            let name = env::var(GOTO_CRATE_NAME_ENV)
                .map_err(|_| format_err!("crate name env var not found"))?;
            let version = env::var(GOTO_CRATE_VERSION_ENV)
                .map_err(|_| format_err!("crate version env var not found"))?;

            env::set_current_dir(org_dir)?;
            f(
                &name,
                Some(&Version::parse(&version)?),
                UnrelatedOrDependency::Unrelated,
            )?;
        }
    } else {
        let name = args
            .crate_
            .name
            .clone()
            .ok_or_else(|| format_err!("Crate name required"))?;

        f(
            &name,
            args.crate_.version.as_ref(),
            UnrelatedOrDependency::from_unrelated_flag(args.unrelated),
        )?;
    }
    Ok(())
}

pub fn create_trust_proof(
    ids: Vec<String>,
    trust_or_distrust: TrustOrDistrust,
    proof_create_opt: &opts::CommonProofCreate,
) -> Result<()> {
    let local = Local::auto_open()?;

    let own_id = local.read_current_unlocked_id(&crev_common::read_passphrase)?;

    let trust = local.build_trust_proof(own_id.as_pubid(), ids.clone(), trust_or_distrust)?;

    let proof = trust.sign_by(&own_id)?;
    let commit_msg = format!(
        "Add {t_or_d} for {ids}",
        t_or_d = trust_or_distrust,
        ids = ids.join(", ")
    );

    maybe_store(&local, &proof, &commit_msg, proof_create_opt)?;

    Ok(())
}

pub fn is_file_with_ext(entry: &walkdir::DirEntry, file_ext: &str) -> bool {
    if !entry.file_type().is_file() {
        return false;
    }
    entry
        .path()
        .extension()
        .map(|ext| ext.to_string_lossy().as_ref() == file_ext)
        .unwrap_or(false)
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
        .inside_out_iter()
        .filter_map(|res| res)
}

pub fn get_geiger_count(path: &Path) -> Result<u64> {
    let mut count = 0;
    for metrics in iter_rs_files_in_dir(path)
        .flat_map_ok(|path| geiger::find_unsafe_in_file(&path, geiger::IncludeTests::No))
    {
        let counters = metrics?.counters;
        count += counters.functions.unsafe_
            + counters.exprs.unsafe_
            + counters.item_impls.unsafe_
            + counters.item_traits.unsafe_
            + counters.methods.unsafe_
    }

    Ok(count)
}

/// Result of `run_command`
///
/// This is to distinguish expeced non-success results,
/// from errors: unexpected failures.
pub enum CommandExitStatus {
    // `verify deps` failed
    VerificationFailed,
    // Success, exit code 0
    Successs,
}

pub fn is_digest_clean(
    db: &crev_lib::ProofDB,
    name: &str,
    version: &Version,
    digest: &crev_data::Digest,
) -> bool {
    let mut at_least_one = false;
    !db.get_package_reviews_for_package(PROJECT_SOURCE_CRATES_IO, Some(name), Some(version))
        .map(|review| {
            at_least_one = true;
            review
        })
        .all(|review| review.package.digest != digest.as_slice())
        || !at_least_one
}

