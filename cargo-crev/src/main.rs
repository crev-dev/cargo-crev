#[macro_use]
extern crate structopt;

use self::prelude::*;
use ::term::color;
use cargo::{
    core::dependency::Dependency,
    core::source::SourceMap,
    core::{package_id::PackageId, SourceId},
    util::important_paths::find_root_manifest_for_wd,
};
use crev_lib::ProofStore;
use crev_lib::{self, local::Local};
use default::default;
use semver;
use serde::Deserialize;
use std::{
    collections::HashSet,
    env, fmt,
    io::BufRead,
    path::{Path, PathBuf},
    process,
};
use structopt::StructOpt;

mod crates_io;
mod opts;
mod prelude;
mod term;

use crev_data::proof;
use crev_lib::{TrustOrDistrust, TrustOrDistrust::*};

struct Repo {
    manifest_path: PathBuf,
    config: cargo::util::config::Config,
}

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

const VCS_INFO_JSON_FILE: &str = ".cargo_vcs_info.json";

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
#[derive(Debug)]
struct KnownOwnersColored(usize);

impl fmt::Display for KnownOwnersColored {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

impl crev_lib::Colored for KnownOwnersColored {
    fn color(&self) -> Option<color::Color> {
        if self.0 > 0 {
            Some(color::GREEN)
        } else {
            None
        }
    }
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

    fn for_every_non_local_dependency_dir(
        &self,
        mut f: impl FnMut(&PackageId, &Path) -> Result<()>,
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
        let source_id = SourceId::crates_io(&self.config)?;
        let map = cargo::sources::SourceConfigMap::new(&self.config)?;
        let mut source = map.load(&source_id)?;

        let pkgs = package_set.get_many(package_set.package_ids())?;

        for pkg in pkgs {
            if !pkg.summary().source_id().is_registry() {
                continue;
            }

            if !pkg.root().exists() {
                source.download(pkg.package_id())?;
            }

            f(&pkg.package_id(), &pkg.root())?;
        }

        Ok(())
    }

    fn find_idependent_crate_dir(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<Option<(PathBuf, semver::Version)>> {
        let map = cargo::sources::SourceConfigMap::new(&self.config)?;
        let source_id = SourceId::crates_io(&self.config)?;
        let mut source = map.load(&source_id)?;
        let mut summaries = vec![];
        let dependency_request =
            Dependency::parse_no_deprecated(name, version, source.source_id())?;
        source.query(&dependency_request, &mut |summary| {
            summaries.push(summary.clone())
        })?;
        let summary = if let Some(version) = version {
            summaries
                .iter()
                .find(|s| s.version().to_string() == version)
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
        let package_set = cargo::core::PackageSet::new(
            &[summary.package_id().clone()],
            source_map,
            &self.config,
        )?;
        let pkg_id = summary.package_id();
        let pkg = package_set.get_one(pkg_id)?;

        Ok(Some((pkg.root().to_owned(), pkg_id.version().to_owned())))
    }

    fn find_dependency_dir(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<Option<(PathBuf, semver::Version)>> {
        let mut ret = vec![];

        self.for_every_non_local_dependency_dir(|pkg_id, path| {
            if name == pkg_id.name().as_str()
                && (version.is_none() || version == Some(&pkg_id.version().to_string()))
            {
                ret.push((path.to_owned(), pkg_id.version().to_owned()));
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
        version: Option<&str>,
        independent: bool,
    ) -> Result<(PathBuf, semver::Version)> {
        if independent {
            self.find_idependent_crate_dir(name, version)?
        } else {
            self.find_dependency_dir(name, version)?
        }
        .ok_or_else(|| format_err!("Could not find requested crate"))
    }
}

fn cargo_full_ignore_list() -> HashSet<PathBuf> {
    let mut ignore_list = HashSet::new();
    ignore_list.insert(PathBuf::from(".cargo-ok"));
    ignore_list.insert(PathBuf::from("Cargo.lock"));
    ignore_list.insert(PathBuf::from("target"));
    ignore_list
}

fn cargo_min_ignore_list() -> HashSet<PathBuf> {
    let mut ignore_list = HashSet::new();
    ignore_list.insert(PathBuf::from(".cargo-ok"));
    ignore_list
}

fn goto_crate_src(selector: &opts::CrateSelector, independent: bool) -> Result<()> {
    if env::var(GOTO_ORIGINAL_DIR_ENV).is_ok() {
        bail!("You're already in a `cargo crev goto` shell");
    };
    let repo = Repo::auto_open_cwd()?;
    let name = selector
        .name
        .clone()
        .ok_or_else(|| format_err!("Crate name argument required"))?;
    let (pkg_dir, crate_version) =
        repo.find_crate(&name, selector.version.as_deref(), independent)?;

    let shell = env::var_os("SHELL").ok_or_else(|| format_err!("$SHELL not set"))?;
    let cwd = env::current_dir()?;

    eprintln!("Opening shell in: {}", pkg_dir.display());
    eprintln!("Use `exit` or Ctrl-D to return to the original project.",);
    eprintln!("Use `review` and `flag` without any arguments to review this crate.");
    let status = process::Command::new(shell)
        .current_dir(pkg_dir)
        .env(GOTO_ORIGINAL_DIR_ENV, cwd)
        .env(GOTO_CRATE_NAME_ENV, name)
        .env(GOTO_CRATE_VERSION_ENV, &crate_version.to_string())
        .status()?;

    if !status.success() {
        bail!("Shell returned {}", status);
    }

    Ok(())
}

fn ensure_known_owners_exists(local: &crev_lib::Local) -> Result<()> {
    let path = local.get_proofs_dir_path()?.join(KNOWN_CARGO_OWNERS_FILE);
    if !path.exists() {
        crev_common::store_str_to_file(&path, include_str!("known_cargo_owners_defaults.txt"))?;
        local.proof_dir_git_add_path(&PathBuf::from(KNOWN_CARGO_OWNERS_FILE))?;
    }

    Ok(())
}

fn read_known_owners() -> Result<HashSet<String>> {
    let local = Local::auto_create_or_open()?;
    let content = if let Some(path) = local.get_proofs_dir_path_opt()? {
        let path = path.join(KNOWN_CARGO_OWNERS_FILE);
        crev_common::read_file_to_string(&path)?
    } else {
        include_str!("known_cargo_owners_defaults.txt").to_string()
    };
    Ok(content
        .lines()
        .map(|s| s.trim())
        .filter(|s| !s.starts_with('#'))
        .map(|s| s.to_string())
        .collect())
}

fn edit_known_owners() -> Result<()> {
    let local = Local::auto_create_or_open()?;
    let path = local.get_proofs_dir_path()?.join(KNOWN_CARGO_OWNERS_FILE);
    ensure_known_owners_exists(&local)?;
    crev_lib::util::edit_file(&path)?;
    Ok(())
}

fn clean_crate(name: &str, version: Option<&str>, independent: bool) -> Result<()> {
    let repo = Repo::auto_open_cwd()?;
    let (pkg_dir, _crate_version) = repo.find_crate(name, version, independent)?;

    assert!(!pkg_dir.starts_with(std::env::current_dir()?));

    if pkg_dir.is_dir() {
        std::fs::remove_dir_all(&pkg_dir)?;
    }
    let (_pkg_dir, _crate_version) = repo.find_crate(name, version, independent)?;
    Ok(())
}

/// Review a crate
///
/// * `independent` - the crate might not actually be a dependency
fn create_review_proof(
    name: &str,
    version: Option<&str>,
    independent: bool,
    trust: TrustOrDistrust,
    proof_create_opt: &opts::CommonProofCreate,
) -> Result<()> {
    let repo = Repo::auto_open_cwd()?;
    let (pkg_dir, crate_version) = repo.find_crate(name, version, independent)?;

    assert!(!pkg_dir.starts_with(std::env::current_dir()?));
    let local = Local::auto_open()?;

    // to protect from creating a digest from a crate in unclean state
    // we move the old directory, download a fresh one and double
    // check if the digest was the same
    // BUG: TODO: https://users.rust-lang.org/t/append-an-additional-extension/23586
    let reviewed_pkg_dir: PathBuf =
        crev_common::fs::append_to_path(pkg_dir.clone(), ".crev.reviewed");
    if reviewed_pkg_dir.is_dir() {
        std::fs::remove_dir_all(&reviewed_pkg_dir)?;
    }

    // to prevent user calling `crev review` from the source dir,
    // having the cwd pulled from under them and confusing their
    // shells, we move all the entries in a dir, instead of the whole
    // dir. this is not a perfect solution, but better than nothing.
    crev_common::fs::move_dir_content(&pkg_dir, &reviewed_pkg_dir)?;
    let (pkg_dir_second, crate_version_second) = repo.find_crate(name, version, independent)?;
    assert_eq!(pkg_dir, pkg_dir_second);
    assert_eq!(crate_version, crate_version_second);

    let digest_clean = crev_lib::get_recursive_digest_for_dir(&pkg_dir, &cargo_min_ignore_list())?;
    let digest_reviewed =
        crev_lib::get_recursive_digest_for_dir(&reviewed_pkg_dir, &cargo_full_ignore_list())?;

    if digest_clean != digest_reviewed {
        bail!(
            "The digest of the reviewed and freshly downloaded crate were different; {} != {}; {} != {}",
            digest_clean,
            digest_reviewed,
            pkg_dir.display(),
            reviewed_pkg_dir.display(),
        );
    }
    std::fs::remove_dir_all(&reviewed_pkg_dir)?;

    let vcs = VcsInfoJson::read_from_crate_dir(&pkg_dir)?;
    let id = local.read_current_unlocked_id(&crev_common::read_passphrase)?;

    let review = proof::review::PackageBuilder::default()
        .from(id.id.to_owned())
        .package(proof::PackageInfo {
            id: None,
            source: PROJECT_SOURCE_CRATES_IO.to_owned(),
            name: name.to_owned(),
            version: crate_version.to_string(),
            digest: digest_clean.into_vec(),
            digest_type: proof::default_digest_type(),
            revision: vcs
                .and_then(|vcs| vcs.get_git_revision())
                .unwrap_or_else(|| "".into()),
            revision_type: proof::default_revision_type(),
        })
        .review(trust.to_review())
        .build()
        .map_err(|e| format_err!("{}", e))?;

    let review = crev_lib::util::edit_proof_content_iteractively(&review.into())?;

    if proof_create_opt.print_unsigned {
        print!("{}", review);
    }

    let proof = review.sign_by(&id)?;

    if proof_create_opt.print_signed {
        print!("{}", proof);
    }

    if !proof_create_opt.no_store {
        local.insert(&proof)?;

        if !proof_create_opt.no_commit {
            let commit_msg = format!(
                "Add review for {crate} v{version}",
                crate = name,
                version = crate_version
            );
            local
                .proof_dir_commit(&commit_msg)
                .with_context(|_| format_err!("Could not not automatically commit"))?;
        }
    }

    Ok(())
}

fn find_reviews(
    crate_: &opts::CrateSelector,
    trust_params: &crev_lib::TrustDistanceParams,
) -> Result<impl Iterator<Item = proof::review::Package>> {
    let local = crev_lib::Local::auto_open()?;
    let (db, _trust_set) = local.load_db(&trust_params)?;
    Ok(db.get_package_reviews_for_package(
        PROJECT_SOURCE_CRATES_IO,
        crate_.name.as_ref().map(|s| s.as_str()),
        crate_.version.as_ref().map(|s| s.as_str()),
    ))
}

fn list_reviews(crate_: &opts::CrateSelector) -> Result<()> {
    // TODO: take trust params?
    for review in find_reviews(crate_, &default())? {
        println!("{}", review);
    }

    Ok(())
}

fn handle_goto_mode_command<F>(args: &opts::ReviewOrGotoCommon, f: F) -> Result<()>
where
    F: FnOnce(&str, Option<&str>, bool) -> Result<()>,
{
    if let Some(org_dir) = env::var_os(GOTO_ORIGINAL_DIR_ENV) {
        if args.crate_.name.is_some() {
            bail!("In `crev goto` mode no arguments can be given");
        } else {
            let name = env::var(GOTO_CRATE_NAME_ENV)
                .map_err(|_| format_err!("crate name env var not found"))?;
            let version = env::var(GOTO_CRATE_VERSION_ENV)
                .map_err(|_| format_err!("crate versoin env var not found"))?;

            env::set_current_dir(org_dir)?;
            f(&name, Some(&version), true)?;
        }
    } else {
        let name = args
            .crate_
            .name
            .clone()
            .ok_or_else(|| format_err!("Crate name required"))?;

        f(&name, args.crate_.version.as_deref(), args.independent)?;
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

    if proof_create_opt.print_unsigned {
        print!("{}", trust);
    }

    let proof = trust.sign_by(&own_id)?;

    if proof_create_opt.print_signed {
        print!("{}", proof);
    }

    if !proof_create_opt.no_store {
        local.insert(&proof)?;

        if !proof_create_opt.no_commit {
            let commit_msg = format!(
                "Add {t_or_d} for {ids}",
                t_or_d = trust_or_distrust,
                ids = ids.join(", ")
            );
            local
                .proof_dir_commit(&commit_msg)
                .with_context(|_| format_err!("Could not not automatically commit"))?;
        }
    }

    Ok(())
}

fn run_command(command: opts::Command) -> Result<()> {
    match command {
        opts::Command::New(cmd) => match cmd {
            opts::New::Id(args) => {
                let local = Local::auto_create_or_open()?;
                let res = local.generate_id(args.url, args.github_username, args.use_https_push);
                if res.is_err() {
                    eprintln!("Visit https://github.com/dpc/crev/wiki/Proof-Repository for help.");
                }
                let local = crev_lib::Local::auto_open()?;
                let _ = ensure_known_owners_exists(&local);
                res?;
            }
        },
        opts::Command::Switch(cmd) => match cmd {
            opts::Switch::Id(args) => {
                let local = Local::auto_open()?;
                local.switch_id(&args.id)?
            }
        },
        opts::Command::Edit(cmd) => match cmd {
            opts::Edit::Readme => {
                let local = crev_lib::Local::auto_open()?;
                local.edit_readme()?;
            }
            opts::Edit::Known => {
                edit_known_owners()?;
            }
        },
        opts::Command::Verify(cmd) => match cmd {
            opts::Verify::Deps(args) => {
                let mut term = term::Term::new();
                let local = crev_lib::Local::auto_create_or_open()?;
                let (db, trust_set) = local.load_db(&args.trust_params.clone().into())?;

                let repo = Repo::auto_open_cwd()?;
                let ignore_list = cargo_min_ignore_list();
                let cratesio = crates_io::Client::new(&local)?;

                if term.stderr_is_tty && term.stdout_is_tty {
                    if args.verbose {
                        eprint!("{:43} ", "digest");
                    }
                    eprint!(
                        "{:8} {:8} {:^13} {:6}",
                        "verifi.", "reviews", "downloads", "owners"
                    );
                    eprintln!(" {:<19} {:<15}", "crate", "version");
                }
                let known_owners = read_known_owners().unwrap_or_else(|_| HashSet::new());
                repo.for_every_non_local_dependency_dir(|pkg_id, path| {
                    let pkg_name = pkg_id.name().as_str();
                    let pkg_version = pkg_id.version().to_string();

                    let digest = crev_lib::get_dir_digest(&path, &ignore_list)?;
                    let result = db.verify_package_digest(&digest, &trust_set);

                    if result.is_verified() && args.skip_verified {
                        return Ok(());
                    }

                    let pkg_review_count =
                        db.get_package_review_count(PROJECT_SOURCE_CRATES_IO, Some(pkg_name), None);
                    let pkg_version_review_count = db.get_package_review_count(
                        PROJECT_SOURCE_CRATES_IO,
                        Some(pkg_name),
                        Some(&pkg_version),
                    );

                    let (version_downloads, total_downloads) = cratesio
                        .get_downloads_count(&pkg_name, &pkg_version)
                        .map(|(a, b)| (a.to_string(), b.to_string()))
                        .unwrap_or_else(|_e| ("err".into(), "err".into()));

                    let owners = cratesio.get_owners(&pkg_name).ok();
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
                    term.stdout(format_args!("{:8}", result), &result)?;
                    print!(
                        " {:2} {:2} {:>8} {:>9}",
                        pkg_version_review_count,
                        pkg_review_count,
                        version_downloads,
                        total_downloads,
                    );
                    let colored_count_color = KnownOwnersColored(known_owners_count.unwrap_or(0));
                    term.stdout(
                        format_args!(
                            " {}",
                            &known_owners_count
                                .map(|c| c.to_string())
                                .unwrap_or_else(|| "?".into())
                        ),
                        &colored_count_color,
                    )?;
                    print!(
                        "/{}",
                        total_owners_count
                            .map(|c| c.to_string())
                            .unwrap_or_else(|| "?".into())
                    );
                    println!(" {:<20} {:<15}", pkg_name, pkg_version);

                    Ok(())
                })?;
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
                // TODO: move to crev-lib
                opts::QueryId::Trusted { trust_params } => {
                    let local = crev_lib::Local::auto_open()?;
                    let (db, trust_set) = local.load_db(&trust_params.into())?;
                    for id in trust_set.trusted_ids() {
                        println!(
                            "{} {}",
                            id,
                            db.lookup_url(id).map(|url| url.url.as_str()).unwrap_or("")
                        );
                    }
                }
                // TODO: move to crev-lib
                opts::QueryId::All => {
                    let local = crev_lib::Local::auto_open()?;
                    let (db, _trust_set) = local.load_db(&default())?;

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
        },
        opts::Command::Review(args) => {
            handle_goto_mode_command(&args.common, |c, v, i| {
                create_review_proof(c, v, i, TrustOrDistrust::Trust, &args.common_proof_create)
            })?;
        }
        opts::Command::Goto(args) => {
            goto_crate_src(&args.crate_, args.independent)?;
        }
        opts::Command::Flag(args) => {
            handle_goto_mode_command(&args.common, |c, v, i| {
                create_review_proof(
                    c,
                    v,
                    i,
                    TrustOrDistrust::Distrust,
                    &args.common_proof_create,
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
            let mut status = local.run_git(vec!["commit".into(), "-a".into()])?;
            if status.code().unwrap_or(-1) == 0 {
                status = local.run_git(vec!["push".into()])?;
            }
            std::process::exit(status.code().unwrap_or(-159));
        }
        opts::Command::Pull => {
            let local = Local::auto_open()?;
            let status = local.run_git(vec!["pull".into()])?;
            std::process::exit(status.code().unwrap_or(-159));
        }
        opts::Command::Fetch(cmd) => match cmd {
            opts::Fetch::Trusted(params) => {
                let local = Local::auto_open()?;
                local.fetch_trusted(params.into())?;
            }
            opts::Fetch::Url(params) => {
                let local = Local::auto_open()?;
                local.fetch_url(&params.url)?;
            }
            opts::Fetch::All => {
                let local = Local::auto_open()?;
                local.fetch_all()?;
            }
        },
        opts::Command::Export(cmd) => match cmd {
            opts::Export::Id(params) => {
                let local = Local::auto_open()?;
                println!("{}", local.export_locked_id(params.id)?);
            }
        },
        opts::Command::Import(cmd) => match cmd {
            opts::Import::Id => {
                let local = Local::auto_create_or_open()?;
                let term = term::Term::new();
                if term.stdin_is_tty {
                    eprintln!("Paste in the text and press Ctrl+D.")
                }
                let mut s = vec![];

                std::io::stdin().lock().read_until(0, &mut s)?;
                let id = local.import_locked_id(&String::from_utf8(s)?)?;
                // Note: It's unclear how much of this should be done by
                // the library
                local.save_current_id(&id.id)?;

                let proof_dir_path = local.get_proofs_dir_path_for_url(&id.url)?;
                if !proof_dir_path.exists() {
                    local.clone_proof_dir_from_git(&id.url.url, false)?;
                }
            }
        },
    }

    Ok(())
}

fn main() {
    let opts = opts::Opts::from_args();
    let opts::MainCommand::Crev(command) = opts.command;
    run_command(command).unwrap_or_else(|e| eprintln!("{}", e.display_causes_and_backtrace()));
}
