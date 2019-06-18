use self::prelude::*;
use cargo::{
    core::{dependency::Dependency, source::SourceMap, Package, SourceId},
    util::important_paths::find_root_manifest_for_wd,
};
use crev_common::convert::OptionDeref;
use crev_lib::{self, local::Local, ProofStore, ReviewMode};
use failure::format_err;
use insideout::InsideOutIter;
use resiter::FlatMap;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashSet},
    default::Default,
    env,
    io::BufRead,
    path::{Path, PathBuf},
    process,
};
use structopt::StructOpt;

mod crates_io;
mod opts;
mod prelude;
mod term;
mod tokei;

use crev_data::proof;
use crev_lib::TrustOrDistrust::{self, *};

/// Name of ENV with original location `crev goto` was called from
const GOTO_ORIGINAL_DIR_ENV: &str = "CARGO_CREV_GOTO_ORIGINAL_DIR";
/// Name of ENV with name of the crate that we've `goto`ed to
const GOTO_CRATE_NAME_ENV: &str = "CARGO_CREV_GOTO_ORIGINAL_NAME";
/// Name of ENV with version of the crate that we've `goto`ed to
const GOTO_CRATE_VERSION_ENV: &str = "CARGO_CREV_GOTO_ORIGINAL_VERSION";

/// Name of file we store user-personalized
const KNOWN_CARGO_OWNERS_FILE: &str = "known_cargo_owners.txt";

/// Constant we use for `source` in the review proof
const PROJECT_SOURCE_CRATES_IO: &str = "https://crates.io";

/// The file added to crates containing vcs revision
const VCS_INFO_JSON_FILE: &str = ".cargo_vcs_info.json";

fn pad_left_manually(s: String, width: usize) -> String {
    if s.len() <= width {
        let padding = std::iter::repeat(" ")
            .take(width - s.len())
            .collect::<String>();
        format!("{}{}", s, padding)
    } else {
        s
    }
}

fn latest_trusted_version_string(
    base_version: Version,
    latest_trusted_version: Option<Version>,
) -> String {
    latest_trusted_version
        .map(|latest_trusted_version| {
            let ch = if base_version < latest_trusted_version {
                "↑"
            } else if latest_trusted_version < base_version {
                "↓"
            } else {
                "="
            };
            format!("{}{}", ch, latest_trusted_version)
        })
        .unwrap_or_else(|| "".into())
}

/// Data from `.cargo_vcs_info.json`
#[derive(Debug, Clone, Deserialize)]
struct VcsInfoJson {
    git: VcsInfoJsonGit,
}

#[derive(Debug, Clone, Deserialize)]
enum VcsInfoJsonGit {
    #[serde(rename = "sha1")]
    Sha1(String),
}

impl VcsInfoJson {
    fn read_from_crate_dir(pkg_dir: &Path) -> Result<Option<Self>> {
        let path = pkg_dir.join(VCS_INFO_JSON_FILE);

        if path.exists() {
            let txt = crev_common::read_file_to_string(&path)?;
            let info: VcsInfoJson = serde_json::from_str(&txt)?;
            Ok(Some(info))
        } else {
            Ok(None)
        }
    }
    fn get_git_revision(&self) -> Option<String> {
        let VcsInfoJsonGit::Sha1(ref s) = self.git;
        Some(s.to_string())
    }
}

/// A handle to the current Rust project
struct Repo {
    manifest_path: PathBuf,
    config: cargo::util::config::Config,
}

impl Repo {
    fn auto_open_cwd() -> Result<Self> {
        cargo::core::enable_nightly_features();
        let cwd = env::current_dir()?;
        let manifest_path = find_root_manifest_for_wd(&cwd)?;
        let mut config = cargo::util::config::Config::default()?;
        config.configure(0, None, &None, false, false, &None, &[])?;
        Ok(Repo {
            manifest_path,
            config,
        })
    }

    fn update_source(&self) -> Result<()> {
        let mut source = self.load_source()?;
        source.update()?;
        Ok(())
    }

    fn update_counts(&self) -> Result<()> {
        let local = crev_lib::Local::auto_create_or_open()?;
        let crates_io = crates_io::Client::new(&local)?;

        self.for_every_non_local_dep_crate(|crate_| {
            let _ = crates_io.get_downloads_count(&crate_.name(), &crate_.version());
            Ok(())
        })?;

        Ok(())
    }

    fn load_source<'a>(&'a self) -> Result<Box<cargo::core::source::Source + 'a>> {
        let source_id = SourceId::crates_io(&self.config)?;
        let map = cargo::sources::SourceConfigMap::new(&self.config)?;
        let yanked_whitelist = HashSet::new();
        let source = map.load(source_id, &yanked_whitelist)?;
        Ok(source)
    }

    /// Run `f` for every non-local dependency crate
    fn for_every_non_local_dep_crate(
        &self,
        mut f: impl FnMut(&Package) -> Result<()>,
    ) -> Result<()> {
        let workspace = cargo::core::Workspace::new(&self.manifest_path, &self.config)?;
        let specs = cargo::ops::Packages::All.to_package_id_specs(&workspace)?;
        let (package_set, _resolve) = cargo::ops::resolve_ws_precisely(
            &workspace,
            None,
            &[],
            true,  // all_features
            false, // no_default_features
            &specs,
        )?;
        let mut source = self.load_source()?;

        let pkgs = package_set.get_many(package_set.package_ids())?;

        for pkg in pkgs {
            if !pkg.summary().source_id().is_registry() {
                continue;
            }

            if !pkg.root().exists() {
                source.download(pkg.package_id())?;
            }

            f(&pkg)?;
        }

        Ok(())
    }

    fn find_idependent_crate_dir(
        &self,
        name: &str,
        version: Option<&Version>,
    ) -> Result<Option<Package>> {
        let mut source = self.load_source()?;
        let mut summaries = vec![];
        let version_str = version.map(ToString::to_string);
        let dependency_request =
            Dependency::parse_no_deprecated(name, version_str.as_deref(), source.source_id())?;
        source.query(&dependency_request, &mut |summary| {
            summaries.push(summary.clone())
        })?;
        let summary = if let Some(version) = version {
            summaries.iter().find(|s| s.version() == version)
        } else {
            summaries.iter().max_by_key(|s| s.version())
        };

        let summary = if let Some(summary) = summary {
            summary
        } else {
            return Ok(None);
        };

        let mut source_map = SourceMap::new();
        source_map.insert(source);
        let package_set =
            cargo::core::PackageSet::new(&[summary.package_id()], source_map, &self.config)?;
        let pkg_id = summary.package_id();

        Ok(Some(package_set.get_one(pkg_id)?.to_owned()))
    }

    fn find_dependency(&self, name: &str, version: Option<&Version>) -> Result<Option<Package>> {
        let mut ret = vec![];

        self.for_every_non_local_dep_crate(|pkg| {
            let pkg_id = pkg.package_id();
            if name == pkg_id.name().as_str()
                && (version.is_none() || version == Some(&pkg_id.version()))
            {
                ret.push(pkg.to_owned());
            }
            Ok(())
        })?;

        match ret.len() {
            0 => Ok(None),
            1 => Ok(Some(ret[0].clone())),
            n => bail!("Ambiguous selection: {} matches found", n),
        }
    }

    fn find_crate(
        &self,
        name: &str,
        version: Option<&Version>,
        unrelated: UnrelatedOrDependency,
    ) -> Result<Package> {
        if unrelated.is_unrelated() {
            self.find_idependent_crate_dir(name, version)?
        } else {
            self.find_dependency(name, version)?
        }
        .ok_or_else(|| format_err!("Could not find requested crate"))
    }
}

/// Ignore things that are commonly added during the review (eg. by RLS)
fn cargo_full_ignore_list() -> HashSet<PathBuf> {
    let mut ignore_list = HashSet::new();
    ignore_list.insert(PathBuf::from(".cargo-ok"));
    ignore_list.insert(PathBuf::from("Cargo.lock"));
    ignore_list.insert(PathBuf::from("target"));
    ignore_list
}

/// Ignore only the marker added by `cargo` after fully downloading and extracting crate
fn cargo_min_ignore_list() -> HashSet<PathBuf> {
    let mut ignore_list = HashSet::new();
    ignore_list.insert(PathBuf::from(".cargo-ok"));
    ignore_list
}

#[cfg(target_family = "unix")]
// on Unix we use `exec` so that stuff like Ctrl-C works
// we don't care about destructors at this point
fn exec_into(mut command: process::Command) -> Result<()> {
    use std::os::unix::process::CommandExt;
    bail!(command.exec());
}

#[cfg(target_family = "windows")]
// TODO: Is this the way to do it in Windows?
fn exec_into(mut command: process::Command) -> Result<()> {
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
fn goto_crate_src(selector: &opts::CrateSelector, unrelated: UnrelatedOrDependency) -> Result<()> {
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

fn ensure_known_owners_list_exists(local: &crev_lib::Local) -> Result<()> {
    let path = local.get_proofs_dir_path()?.join(KNOWN_CARGO_OWNERS_FILE);
    if !path.exists() {
        crev_common::store_str_to_file(&path, include_str!("known_cargo_owners_defaults.txt"))?;
        local.proof_dir_git_add_path(&PathBuf::from(KNOWN_CARGO_OWNERS_FILE))?;
    }

    Ok(())
}

fn read_known_owners_list() -> Result<HashSet<String>> {
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

fn edit_known_owners_list() -> Result<()> {
    let local = Local::auto_create_or_open()?;
    let path = local.get_proofs_dir_path()?.join(KNOWN_CARGO_OWNERS_FILE);
    ensure_known_owners_list_exists(&local)?;
    crev_lib::util::edit_file(&path)?;
    Ok(())
}

/// Wipe the crate source, then re-download it
fn clean_crate(
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

fn get_open_cmd(local: &Local) -> Result<String> {
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
fn crate_open(
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
    fn is_unrelated(self) -> bool {
        self == UnrelatedOrDependency::Unrelated
    }

    fn from_unrelated_flag(u: bool) -> Self {
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
fn crate_review_activity_check(
    local: &Local,
    name: &str,
    version: &Version,
    diff: &Option<Option<Version>>,
    skip_activity_check: bool,
) -> Result<Option<Version>> {
    let activity =
        local.read_review_activity(PROJECT_SOURCE_CRATES_IO, name, version)?;

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
                if diff.is_some() {
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

fn check_package_clean_state(
    repo: &Repo,
    crate_root: &Path,
    name: &str,
    version: &Version,
) -> Result<crev_data::Digest> {
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

    Ok(digest_clean)
}

/// Review a crate
///
/// * `unrelated` - the crate might not actually be a dependency
fn create_review_proof(
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

    let digest_clean =
        check_package_clean_state(&repo, &crate_root, name, &effective_crate_version)?;

    let diff_base = if let Some(diff_base_version) = diff_base_version {
        Some(
            crev_data::proof::review::package::DifferentialBaseBuilder::default()
                .digest(
                    check_package_clean_state(&repo, &crate_root, name, &diff_base_version)?
                        .into_vec(),
                )
                .version(diff_base_version)
                .build()
                .map_err(|e| format_err!("{}", e))?,
        )
    } else {
        None
    };

    let vcs = VcsInfoJson::read_from_crate_dir(&crate_root)?;
    let id = local.read_current_unlocked_id(&crev_common::read_passphrase)?;

    let db = local.load_db()?;

    let (previous_date, review) = if let Some(mut previous_review) = db
        .get_package_review_by_author(
            PROJECT_SOURCE_CRATES_IO,
            name,
            effective_crate_version,
            &id.id.id,
        ) {
        previous_review.from = id.id.to_owned();
        let previous_date = previous_review.date;
        previous_review.date = crev_common::now();

        if previous_review.advisory.is_none() {
            if let Some(advise_common) = advise_common {
                let mut advisory: proof::review::package::Advisory = advise_common.affected.into();
                advisory.critical = advise_common.critical;
                previous_review.advisory = Some(advisory);
            }
        }
        (Some(previous_date), previous_review)
    } else {
        let mut review = proof::review::PackageBuilder::default()
            .from(id.id.to_owned())
            .package(proof::PackageInfo {
                id: None,
                source: PROJECT_SOURCE_CRATES_IO.to_owned(),
                name: name.to_owned(),
                version: effective_crate_version.to_owned(),
                digest: digest_clean.into_vec(),
                digest_type: proof::default_digest_type(),
                revision: vcs
                    .and_then(|vcs| vcs.get_git_revision())
                    .unwrap_or_else(|| "".into()),
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

        if let Some(advise_common) = advise_common {
            let mut advisory: proof::review::package::Advisory = advise_common.affected.into();
            advisory.critical = advise_common.critical;
            review.advisory = Some(advisory);
        }
        (None, review)
    };

    let review = crev_lib::util::edit_proof_content_iteractively(&review.into(), previous_date)?;

    let proof = review.sign_by(&id)?;

    let commit_msg = format!(
        "Add review for {crate} v{version}",
        crate = name,
        version = effective_crate_version
    );
    maybe_store(&local, &proof, &commit_msg, proof_create_opt)
}

fn maybe_store(
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

fn find_reviews(
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

fn list_reviews(crate_: &opts::CrateSelector) -> Result<()> {
    for review in find_reviews(crate_)? {
        println!("{}", review);
    }

    Ok(())
}

fn find_advisories(
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

fn run_diff(args: &opts::Diff) -> Result<std::process::ExitStatus> {
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
        .arg(src_crate.root())
        .arg(dst_crate.root())
        .args(&args.args)
        .status()
        .expect("failed to execute git");

    Ok(status)
}

fn show_dir(crate_: &opts::CrateSelector, unrelated: UnrelatedOrDependency) -> Result<()> {
    let repo = Repo::auto_open_cwd()?;
    let name = crate_
        .name
        .clone()
        .ok_or_else(|| format_err!("Crate name argument required"))?;
    let crate_ = repo.find_crate(&name, crate_.version.as_ref(), unrelated)?;
    println!("{}", crate_.root().display());

    Ok(())
}

fn list_advisories(crate_: &opts::CrateSelector) -> Result<()> {
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
fn handle_goto_mode_command<F>(args: &opts::ReviewOrGotoCommon, f: F) -> Result<()>
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

fn create_trust_proof(
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

fn iter_rs_files_in_dir(dir: &Path) -> impl Iterator<Item = Result<PathBuf>> {
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

fn get_geiger_count(path: &Path) -> Result<u64> {
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
enum CommandExitStatus {
    // `verify deps` failed
    VerificationFailed,
    // Success, exit code 0
    Successs,
}

fn is_digest_clean(
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

fn run_command(command: opts::Command) -> Result<CommandExitStatus> {
    match command {
        opts::Command::New(cmd) => match cmd {
            opts::New::Id(args) => {
                let local = Local::auto_create_or_open()?;
                let res = local.generate_id(args.url, args.github_username, args.use_https_push);
                if res.is_err() {
                    eprintln!("Visit https://github.com/dpc/crev/wiki/Proof-Repository for help.");
                }
                let local = crev_lib::Local::auto_open()?;
                let _ = ensure_known_owners_list_exists(&local);
                res?;
            }
        },
        opts::Command::Switch(cmd) => match cmd {
            opts::Switch::Id(args) => {
                let local = Local::auto_open()?;
                local.switch_id(&args.id)?
            }
        },
        opts::Command::Diff(args) => {
            let status = run_diff(&args)?;
            std::process::exit(status.code().unwrap_or(-159));
        }
        opts::Command::Edit(cmd) => match cmd {
            opts::Edit::Readme => {
                let local = crev_lib::Local::auto_open()?;
                local.edit_readme()?;
            }
            opts::Edit::Config => {
                let local = crev_lib::Local::auto_create_or_open()?;
                local.edit_user_config()?;
            }
            opts::Edit::Known => {
                edit_known_owners_list()?;
            }
        },
        opts::Command::Verify(cmd) => match cmd {
            // TODO: This is waaay too long; refactor
            opts::Verify::Deps(args) => {
                let mut term = term::Term::new();
                let local = crev_lib::Local::auto_create_or_open()?;
                let db = local.load_db()?;

                let trust_set =
                    if let Some(for_id) = local.get_for_id_from_str_opt(args.for_id.as_deref())? {
                        db.calculate_trust_set(&for_id, &args.trust_params.clone().into())
                    } else {
                        crev_lib::proofdb::TrustSet::default()
                    };

                let repo = Repo::auto_open_cwd()?;
                let ignore_list = cargo_min_ignore_list();
                let crates_io = crates_io::Client::new(&local)?;

                if term.stderr_is_tty && term.stdout_is_tty {
                    if args.verbose {
                        eprint!("{:43} ", "digest");
                    }
                    eprint!(
                        "{:5} {:8} {:^15} {:4} {:6} {:6} {:6} {:4}",
                        "trust",
                        "reviews",
                        "downloads",
                        "own.",
                        "advisr",
                        "lines",
                        "geiger",
                        "flgs"
                    );
                    eprintln!(
                        " {:<20} {:<15} {:<15}",
                        "crate", "version", "latest trusted"
                    );
                }
                let requirements =
                    crev_lib::VerificationRequirements::from(args.requirements.clone());
                let mut unclean_digests = BTreeMap::new();
                let known_owners = read_known_owners_list().unwrap_or_else(|_| HashSet::new());
                let mut total_verification_successful = true;
                repo.for_every_non_local_dep_crate(|crate_| {
                    let crate_id = crate_.package_id();
                    let crate_name = crate_id.name().as_str();
                    let crate_version = crate_id.version();
                    let crate_root = crate_.root();

                    let digest = crev_lib::get_dir_digest(&crate_root, &ignore_list)?;

                    if !is_digest_clean(&db, &crate_name, &crate_version, &digest) {
                        unclean_digests.insert((crate_name, crate_version), digest.clone());
                    }

                    let result = db.verify_package_digest(&digest, &trust_set, &requirements);

                    if !result.is_verified() {
                        total_verification_successful = false;
                    }

                    if result.is_verified() && args.skip_verified {
                        return Ok(());
                    }

                    let pkg_review_count = db.get_package_review_count(
                        PROJECT_SOURCE_CRATES_IO,
                        Some(crate_name),
                        None,
                    );
                    let pkg_version_review_count = db.get_package_review_count(
                        PROJECT_SOURCE_CRATES_IO,
                        Some(crate_name),
                        Some(&crate_version),
                    );

                    let (
                        version_downloads_str,
                        total_downloads_str,
                        version_downloads,
                        total_downloads,
                    ) = crates_io
                        .get_downloads_count(&crate_name, &crate_version)
                        .map(|(a, b)| (a.to_string(), b.to_string(), a, b))
                        .unwrap_or_else(|_e| ("err".into(), "err".into(), 0, 0));

                    let owners = crates_io.get_owners(&crate_name).ok();
                    let (known_owners_count, total_owners_count) = if let Some(owners) = owners {
                        let total_owners_count = owners.len();
                        let known_owners_count = owners
                            .iter()
                            .filter(|o| known_owners.contains(o.as_str()))
                            .count();

                        if known_owners_count > 0 && args.skip_known_owners {
                            return Ok(());
                        }
                        (Some(known_owners_count), Some(total_owners_count))
                    } else {
                        (None, None)
                    };

                    if args.verbose {
                        print!("{:43} ", digest);
                    }
                    term.print(
                        format_args!("{:5}", result),
                        term::verification_status_color(&result),
                    )?;
                    print!(" {:2} {:2}", pkg_version_review_count, pkg_review_count,);
                    term.print(
                        format_args!(" {:>8}", version_downloads_str),
                        if version_downloads < 1000 {
                            Some(::term::color::YELLOW)
                        } else {
                            None
                        },
                    )?;
                    term.print(
                        format_args!(" {:>9}", total_downloads_str),
                        if total_downloads < 10000 {
                            Some(::term::color::YELLOW)
                        } else {
                            None
                        },
                    )?;
                    term.print(
                        format_args!(
                            " {}",
                            &known_owners_count
                                .map(|c| c.to_string())
                                .unwrap_or_else(|| "?".into())
                        ),
                        term::known_owners_count_color(known_owners_count.unwrap_or(0)),
                    )?;
                    print!(
                        "/{} ",
                        total_owners_count
                            .map(|c| c.to_string())
                            .unwrap_or_else(|| "?".into())
                    );

                    let advisories = db.get_advisories_for_version(
                        PROJECT_SOURCE_CRATES_IO,
                        crate_name,
                        crate_version,
                    );
                    let trusted_advisories = advisories
                        .iter()
                        .filter(|(_version, package_review)| {
                            trust_set.contains_trusted(&package_review.from.id)
                        })
                        .count();

                    term.print(
                        format_args!("{:4}", trusted_advisories),
                        if trusted_advisories > 0 {
                            Some(::term::color::RED)
                        } else {
                            None
                        },
                    )?;
                    print!("/");
                    term.print(
                        format_args!("{:<2}", advisories.len()),
                        if advisories.is_empty() {
                            None
                        } else {
                            Some(::term::color::YELLOW)
                        },
                    )?;
                    print!(
                        "{:>6} {:>7}",
                        tokei::get_rust_line_count(crate_root)
                            .ok()
                            .map(|n| n.to_string())
                            .unwrap_or_else(|| "err".into()),
                        get_geiger_count(crate_root)
                            .map(|n| n.to_string())
                            .unwrap_or_else(|_| "err".into()),
                    );
                    term.print(
                        format_args!(" {:4}", if crate_.has_custom_build() { "CB" } else { "" }),
                        ::term::color::YELLOW,
                    )?;
                    print!(
                        " {:<20} {:<15}",
                        crate_name,
                        pad_left_manually(crate_version.to_string(), 15)
                    );

                    let latest_trusted_version = db.find_latest_trusted_version(
                        &trust_set,
                        PROJECT_SOURCE_CRATES_IO,
                        &crate_name,
                        &requirements,
                    );
                    print!(
                        " {:<15}",
                        latest_trusted_version_string(
                            crate_version.clone(),
                            latest_trusted_version
                        )
                    );
                    println!("");

                    Ok(())
                })?;

                if !unclean_digests.is_empty() {
                    eprintln!();
                }

                for (name, version) in unclean_digests.keys() {
                    term.eprint(
                        format_args!("Unclean crate {} {}\n", name, version),
                        ::term::color::RED,
                    )?;
                }

                if !unclean_digests.is_empty() {
                    bail!("Unclean packages detected. Use `cargo crev clean <crate>` to wipe the local source.");
                }

                return Ok(if total_verification_successful {
                    CommandExitStatus::Successs
                } else {
                    CommandExitStatus::VerificationFailed
                });
            }
        },
        opts::Command::Query(cmd) => match cmd {
            opts::Query::Id(cmd) => match cmd {
                opts::QueryId::Current => {
                    let local = Local::auto_open()?;
                    local.show_current_id()?
                }
                opts::QueryId::Own => {
                    let local = Local::auto_open()?;
                    local.list_own_ids()?
                }
                opts::QueryId::Trusted {
                    for_id,
                    trust_params,
                } => {
                    let local = crev_lib::Local::auto_open()?;
                    let db = local.load_db()?;
                    let for_id = local.get_for_id_from_str(for_id.as_deref())?;
                    let trust_set = db.calculate_trust_set(&for_id, &trust_params.into());

                    for id in trust_set.trusted_ids() {
                        println!(
                            "{} {:6} {}",
                            id,
                            trust_set
                                .get_effective_trust_level(id)
                                .expect("Some trust level"),
                            db.lookup_url(id).map(|url| url.url.as_str()).unwrap_or("")
                        );
                    }
                }
                // TODO: move to crev-lib
                opts::QueryId::All => {
                    let local = crev_lib::Local::auto_create_or_open()?;
                    let db = local.load_db()?;

                    for id in &db.all_known_ids() {
                        println!(
                            "{} {}",
                            id,
                            db.lookup_url(id).map(|url| url.url.as_str()).unwrap_or("")
                        );
                    }
                }
            },
            opts::Query::Review(args) => list_reviews(&args.crate_)?,
            opts::Query::Dir(args) => show_dir(
                &args.common.crate_,
                UnrelatedOrDependency::from_unrelated_flag(args.common.unrelated),
            )?,
            opts::Query::Advisory(args) => list_advisories(&args.crate_)?,
        },
        opts::Command::Review(args) => {
            handle_goto_mode_command(&args.common, |c, v, i| {
                create_review_proof(
                    c,
                    v,
                    i,
                    if args.advisory {
                        Some(opts::AdviseCommon::default())
                    } else {
                        None
                    },
                    TrustOrDistrust::Trust,
                    &args.common_proof_create,
                    &args.diff,
                    args.skip_activity_check,
                )
            })?;
        }
        opts::Command::Goto(args) => {
            goto_crate_src(
                &args.crate_,
                UnrelatedOrDependency::from_unrelated_flag(args.unrelated),
            )?;
        }
        opts::Command::Open(args) => {
            handle_goto_mode_command(&args.common.clone(), |c, v, i| {
                crate_open(c, v, i, args.cmd, args.cmd_save)
            })?;
        }
        opts::Command::Flag(args) => {
            handle_goto_mode_command(&args.common, |c, v, i| {
                create_review_proof(
                    c,
                    v,
                    i,
                    if args.advisory {
                        Some(opts::AdviseCommon::default())
                    } else {
                        None
                    },
                    TrustOrDistrust::Distrust,
                    &args.common_proof_create,
                    &args.diff,
                    args.skip_activity_check,
                )
            })?;
        }
        opts::Command::Advise(args) => {
            handle_goto_mode_command(&args.common.clone(), |c, v, i| {
                create_review_proof(
                    c,
                    v,
                    i,
                    Some(args.advise_common),
                    TrustOrDistrust::Distrust,
                    &args.common_proof_create,
                    &None,
                    true,
                )
            })?;
        }
        opts::Command::Clean(args) => {
            handle_goto_mode_command(&args, |c, v, i| clean_crate(c, v, i))?;
        }
        opts::Command::Trust(args) => {
            create_trust_proof(args.pub_ids, Trust, &args.common_proof_create)?;
        }
        opts::Command::Distrust(args) => {
            create_trust_proof(args.pub_ids, Distrust, &args.common_proof_create)?;
        }
        opts::Command::Git(git) => {
            let local = Local::auto_open()?;
            let status = local.run_git(git.args)?;
            std::process::exit(status.code().unwrap_or(-159));
        }
        opts::Command::Push => {
            let local = Local::auto_open()?;
            let status = local.run_git(vec!["push".into()])?;
            std::process::exit(status.code().unwrap_or(-159));
        }
        opts::Command::Publish => {
            let local = Local::auto_open()?;
            let mut status = local.run_git(vec!["diff".into(), "--exit-code".into()])?;

            if status.code().unwrap_or(-2) == 1 {
                status = local.run_git(vec![
                    "commit".into(),
                    "-a".into(),
                    "-m".into(),
                    "auto-commit on `crev publish`".into(),
                ])?;
            }

            if status.code().unwrap_or(-1) == 0 {
                status = local.run_git(vec!["pull".into(), "--rebase".into()])?;
            }
            if status.code().unwrap_or(-1) == 0 {
                status = local.run_git(vec!["push".into()])?;
            }
            std::process::exit(status.code().unwrap_or(-159));
        }
        opts::Command::Pull => {
            let local = Local::auto_open()?;
            let status = local.run_git(vec!["pull".into(), "--rebase".into()])?;
            std::process::exit(status.code().unwrap_or(-159));
        }
        opts::Command::Fetch(cmd) => match cmd {
            opts::Fetch::Trusted(params) => {
                let local = Local::auto_create_or_open()?;
                local.fetch_trusted(params.into())?;
            }
            opts::Fetch::Url(params) => {
                let local = Local::auto_create_or_open()?;
                local.fetch_url(&params.url)?;
            }
            opts::Fetch::All => {
                let local = Local::auto_create_or_open()?;
                local.fetch_all()?;
            }
        },
        opts::Command::Update => {
            let repo = Repo::auto_open_cwd()?;
            repo.update_source()?;
            repo.update_counts()?;
        }
        opts::Command::Export(cmd) => match cmd {
            opts::Export::Id(params) => {
                let local = Local::auto_open()?;
                println!("{}", local.export_locked_id(params.id)?);
            }
        },
        opts::Command::Import(cmd) => match cmd {
            opts::Import::Id => {
                let local = Local::auto_create_or_open()?;
                let s = load_stdin_with_prompt()?;
                let id = local.import_locked_id(&String::from_utf8(s)?)?;
                // Note: It's unclear how much of this should be done by
                // the library
                local.save_current_id(&id.id)?;

                let proof_dir_path = local.get_proofs_dir_path_for_url(&id.url)?;
                if !proof_dir_path.exists() {
                    local.clone_proof_dir_from_git(&id.url.url, false)?;
                }
            }
            opts::Import::Proof(args) => {
                let local = Local::auto_create_or_open()?;
                let id = local.read_current_unlocked_id(&crev_common::read_passphrase)?;

                let s = load_stdin_with_prompt()?;
                let proofs = crev_data::proof::Proof::parse(s.as_slice())?;
                let commit_msg = "Import proofs";

                for proof in proofs {
                    let mut content = proof.content;
                    if args.reset_date {
                        content.set_date(&crev_common::now());
                    }
                    content.set_author(&id.as_pubid());
                    let proof = content.sign_by(&id)?;
                    maybe_store(&local, &proof, &commit_msg, &args.common)?;
                }
            }
        },
    }

    Ok(CommandExitStatus::Successs)
}

fn load_stdin_with_prompt() -> Result<Vec<u8>> {
    let term = term::Term::new();

    if term.stdin_is_tty {
        eprintln!("Paste in the text and press Ctrl+D.")
    }
    let mut s = vec![];

    std::io::stdin().lock().read_until(0, &mut s)?;
    Ok(s)
}

fn main() {
    let opts = opts::Opts::from_args();
    let opts::MainCommand::Crev(command) = opts.command;
    match run_command(command) {
        Ok(CommandExitStatus::Successs) => {}
        Ok(CommandExitStatus::VerificationFailed) => std::process::exit(-1),
        Err(e) => {
            eprintln!("{}", e.display_causes_and_backtrace());
            std::process::exit(-2)
        }
    }
}
