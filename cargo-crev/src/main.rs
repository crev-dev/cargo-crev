//! `cargo-crev` - `crev` ecosystem fronted for Rusti (`cargo` integration)
//!
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![type_length_limit = "1932159"]
#![cfg_attr(
    feature = "documentation",
    doc = "See [user documentation module](./doc/user/index.html)."
)]
use crate::prelude::*;
use crev_data::{proof::ContentExt, UnlockedId, SOURCE_CRATES_IO};
use crev_lib::id::LockedId;
use crev_lib::{self, local::Local};
use log::info;
use opts::ReviewCrateSelector;
use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    fmt::Write as _,
    io::{self, BufRead, Write as _},
    panic,
    path::PathBuf,
};
use structopt::StructOpt;

#[cfg(feature = "documentation")]
/// Documentation
pub mod doc;

mod crates_io;
mod deps;
mod dyn_proof;
mod edit;
mod info;
mod opts;
mod prelude;
mod repo;
mod review;
mod shared;
mod term;
mod tokei;
mod wot;

use crate::{
    repo::Repo,
    review::{create_review_proof, list_reviews},
    shared::*,
};
use crev_data::{proof, Id, TrustLevel};
use crev_lib::{TrustProofType, Warning};
use crev_wot::{PkgVersionReviewId, ProofDB, TrustSet, UrlOfId};
use log::debug;

/// Additional functions to extend `Local` by behaviors
/// that are `cargo-crev` specific, like printing
/// helpful diagnostic messages.
trait LocalExt {
    fn run_git_verbose(&self, args: Vec<OsString>) -> Result<std::process::ExitStatus>;
}

impl LocalExt for Local {
    fn run_git_verbose(&self, args: Vec<OsString>) -> Result<std::process::ExitStatus> {
        let mut warnings = Vec::new();
        let res = self.run_git(args, &mut warnings);
        Warning::log_all(&warnings);
        match res {
            Ok(o) => Ok(o),
            Err(crev_lib::Error::GitUrlNotConfigured) => {
                bail!("Id has no public URL set. Run `cargo crev id set-url <your-public-git-proof-repo>` and try again. See https://github.com/crev-dev/cargo-crev/discussions for help.");
            }
            Err(e) => Err(e.into()),
        }
    }
}
pub fn repo_publish() -> Result<()> {
    let local = Local::auto_open()?;
    let mut status = local.run_git_verbose(vec!["diff".into(), "--exit-code".into()])?;

    if status.code().unwrap_or(-2) == 1 {
        status = local.run_git_verbose(vec![
            "commit".into(),
            "-a".into(),
            "-m".into(),
            "auto-commit on `crev publish`".into(),
        ])?;
    }

    if status.code().unwrap_or(-1) == 0 {
        status = local.run_git_verbose(vec!["pull".into(), "--rebase".into()])?;
    }
    if status.code().unwrap_or(-1) == 0 {
        status = local.run_git_verbose(vec!["push".into()])?;
    }
    std::process::exit(status.code().unwrap_or(-159));
}

fn repo_update(args: opts::Update, warnings: &mut Vec<Warning>) -> Result<()> {
    let local = Local::auto_open()?;
    let status = local.run_git_verbose(vec!["pull".into(), "--rebase".into()])?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(-159));
    }
    local.fetch_trusted(opts::TrustDistanceParams::default().into(), None, warnings)?;
    let repo = Repo::auto_open_cwd(args.cargo_opts)?;
    repo.update_counts()?;
    Ok(())
}

pub fn proof_find(args: opts::ProofFind) -> Result<()> {
    let local = crev_lib::Local::auto_open()?;
    let db = local.load_db()?;
    let mut iter = Box::new(db.get_pkg_reviews_for_source(SOURCE_CRATES_IO))
        as Box<dyn Iterator<Item = &proof::review::Package>>;

    if let Some(author) = args.author.as_ref() {
        let id = crev_data::id::Id::crevid_from_str(author)?;
        iter = Box::new(iter.filter(move |r| r.common.from.id == id));
    }

    if let Some(crate_) = args.crate_.as_ref() {
        iter = Box::new(iter.filter(move |r| &r.package.id.id.name == crate_));
    }

    if let Some(version) = args.version.as_ref() {
        iter = Box::new(iter.filter(move |r| &r.package.id.version == version));
    }

    if let Some(git_revision) = args.git_revision.as_ref() {
        iter = Box::new(iter.filter(move |r|
            r.package.revision_type == proof::default_revision_type()
            && (
                git_revision.is_empty() && &r.package.revision == git_revision
                || !git_revision.is_empty() && r.package.revision.starts_with(git_revision)
            )
        ));
    }

    for review in iter {
        println!("---\n{review}");
    }

    Ok(())
}

pub fn proof_reissue(args: opts::ProofReissue) -> Result<()> {
    let local = crev_lib::Local::auto_open()?;
    let db = local.load_db()?;

    let mut iter = Box::new(db.get_pkg_reviews_for_source(SOURCE_CRATES_IO))
        as Box<dyn Iterator<Item = &proof::review::Package>>;

    let author_id = crev_data::id::Id::crevid_from_str(&args.author)?;
    iter = Box::new(iter.filter(move |r| r.common.from.id == author_id));

    if let Some(crate_) = args.crate_.as_ref() {
        iter = Box::new(iter.filter(move |r| &r.package.id.id.name == crate_));
        if let Some(version) = args.version.as_ref() {
            iter = Box::new(iter.filter(move |r| &r.package.id.version == version));
        }
    }

    let sign_id = local.read_current_unlocked_id(&term::read_passphrase)?;

    for orig_review in iter {
        if !args.skip_reissue_check {
            // check if already reissued this review previously to prevent bloating the db
            if db.get_pkg_reviews_for_source(SOURCE_CRATES_IO).any(
                |review: &proof::review::Package| {
                    review.common.from.id == sign_id.id.id && review.package == orig_review.package
                },
            ) {
                println!(
                    "Review of crate {crate_} v{version} rev {rev} from id {orig_id} is already \
                     signed by current id. Skipping reissue. Use `--skip-reissue-check` to override.",
                    crate_ = &orig_review.package.id.id.name,
                    version = &orig_review.package.id.version,
                    rev = &orig_review.package.revision,
                    orig_id =  &orig_review.common.from.id
                );
                continue;
            }
        }

        let pkg_review_id = PkgVersionReviewId::from(orig_review);
        let orig_proof_digest = match db.get_proof_digest_by_pkg_review_id(&pkg_review_id) {
            Some(digest) => digest,
            None => {
                println!(
                    "Missing proof digest on review of {crate_} v{version}. Skipping",
                    crate_ = &orig_review.package.id.id.name,
                    version = &orig_review.package.id.version
                );
                continue;
            }
        };

        println!(
            "Reissuing review of crate {crate_} v{version} from crev id {id}",
            crate_ = &orig_review.package.id.id.name,
            version = &orig_review.package.id.version,
            id = &orig_review.common.from.id
        );

        let mut reissue_review = orig_review.clone();

        reissue_review.touch_date();
        reissue_review.change_from(sign_id.id.clone());
        reissue_review.ensure_kind_is_backfilled();
        reissue_review.set_original_reference(proof::content::OriginalReference {
            proof: orig_proof_digest.0.into(),
            comment: args.comment.clone(),
        });

        let proof = reissue_review.sign_by(&sign_id)?;

        let commit_msg = format!(
            "Signed existing review for {crate} v{version} with different id\n\n\
             New id: {new_id}\n\
             Previous id: {orig_id}\n\
             Previous proof digest: {digest_base64}\n",
            crate = &reissue_review.package.id.id.name,
            version = &reissue_review.package.id.version,
            new_id = &reissue_review.common.from.id,
            orig_id = &orig_review.common.from.id,
            digest_base64 = crev_common::base64_encode(&orig_proof_digest.0)
        );

        maybe_store(&local, &proof, &commit_msg, &args.common_proof_create)?;
    }

    Ok(())
}

fn crate_review(args: &opts::CrateReview, default_trust_type: TrustProofType) -> Result<()> {
    let local = ensure_crev_id_exists_or_make_one()?;

    handle_goto_mode_command(&args.common, Some(&local), |sel| {
        let is_advisory =
            args.advisory || args.affected.is_some() || (!args.issue && args.severity.is_some());
        create_review_proof(
            sel,
            if args.issue {
                Some(crev_data::Level::Medium)
            } else {
                None
            },
            if is_advisory {
                Some(opts::AdviseCommon {
                    severity: args.severity.unwrap_or(crev_data::Level::Medium),
                    affected: args
                        .affected
                        .unwrap_or(crev_data::proof::review::package::VersionRange::Major),
                })
            } else {
                None
            },
            if is_advisory || args.issue {
                TrustProofType::Distrust
            } else {
                default_trust_type
            },
            &args.common_proof_create,
            args.skip_activity_check || is_advisory || args.issue,
            args.overrides,
            args.cargo_opts.clone(),
        )?;
        let has_public_url = local
            .read_current_locked_id()
            .ok()
            .map_or(false, |l| l.to_public_id().url.is_some());
        if !has_public_url {
            eprintln!("Your review is not shared yet. You need to set up a proof repository.");
            eprintln!("Run `cargo crev publish` for more information.");
        }
        Ok(())
    })?;

    Ok(())
}

#[must_use]
pub fn cargo_registry_to_crev_source_id(source_id: &cargo::core::SourceId) -> String {
    let s = source_id.as_url().to_string();
    if &s == "registry+https://github.com/rust-lang/crates.io-index" {
        SOURCE_CRATES_IO.into()
    } else {
        s
    }
}

#[must_use]
pub fn cargo_pkg_id_to_crev_pkg_id(id: &cargo::core::PackageId) -> proof::PackageVersionId {
    proof::PackageVersionId {
        id: proof::PackageId {
            source: cargo_registry_to_crev_source_id(&id.source_id()),
            name: id.name().to_string(),
        },
        version: id.version().clone(),
    }
}

fn print_ids<'a>(ids: impl Iterator<Item = &'a Id>, trust_set: &TrustSet, db: &ProofDB) {
    for id in ids {
        let (status, url) = match db.lookup_url(id) {
            UrlOfId::None => ("", ""),
            UrlOfId::FromSelfVerified(url) => ("==", url.url.as_str()),
            UrlOfId::FromSelf(url) => ("~=", url.url.as_str()),
            UrlOfId::FromOthers(url) => ("??", url.url.as_str()),
        };
        println!(
            "{} {:6} {} {}",
            id,
            trust_set.get_effective_trust_level(id),
            status,
            url,
        );
    }
}

fn url_to_status_str<'a>(id_url: &UrlOfId<'a>) -> (&'static str, &'a str) {
    match id_url {
        UrlOfId::None => ("", ""),
        UrlOfId::FromSelfVerified(url) => ("==", url.url.as_str()),
        UrlOfId::FromSelf(url) => ("~=", url.url.as_str()),
        UrlOfId::FromOthers(url) => ("??", url.url.as_str()),
    }
}

fn print_mvp_ids<'a>(ids: impl Iterator<Item = (&'a Id, u64)>, trust_set: &TrustSet, db: &ProofDB) {
    for (id, count) in ids {
        let (status, url) = url_to_status_str(&db.lookup_url(id));
        println!(
            "{:>3} {} {:6} {} {}",
            count,
            id,
            trust_set.get_effective_trust_level(id),
            status,
            url,
        );
    }
}

fn run_command(command: opts::Command) -> Result<CommandExitStatus> {
    match command {
        opts::Command::Id(args) => match args {
            opts::Id::New(args) => {
                let url = match (args.url, args.github_username) {
                    (None, Some(username)) => {
                        Some(format!("https://github.com/{username}/crev-proofs"))
                    }
                    (Some(url), None) => Some(url),
                    (None, None) => None,
                    _ => bail!("Must provide either a github username or url, but not both."),
                };

                generate_new_id_interactively(url.as_deref(), args.use_https_push)?;
            }
            opts::Id::Switch(args) => {
                let local = Local::auto_open()?;
                local.switch_id(&args.id)?;
            }
            opts::Id::Passwd => {
                current_id_change_passphrase()?;
            }
            opts::Id::Current => {
                let local = Local::auto_open()?;
                let current = local
                    .read_current_locked_id_opt()?
                    .map(|id| id.to_public_id());
                for id in local.get_current_user_public_ids()? {
                    let is_current = current.as_ref().map_or(false, |c| c.id == id.id);
                    println!(
                        "{} {}{}",
                        id.id,
                        id.url_display(),
                        if is_current { " (current)" } else { "" }
                    );
                }
            }
            opts::Id::SetUrl(args) => {
                validate_public_repo_url(&args.url)?;
                match current_id_set_url(&args.url, args.use_https_push) {
                    Err(
                        crev_lib::Error::CurrentIDNotSet
                        | crev_lib::Error::IDNotSpecifiedAndCurrentIDNotSet
                        | crev_lib::Error::UserConfigNotInitialized,
                    ) => {
                        eprintln!("set-url requires a CrevID set up, so we'll set up one now.");
                        generate_new_id_interactively(Some(&args.url), args.use_https_push)?;
                    }
                    res => res?,
                }
            }
            opts::Id::Export(args) => {
                let local = Local::auto_open()?;
                println!("{}", local.export_locked_id(args.id)?);
            }
            opts::Id::Import => {
                let local = Local::auto_create_or_open()?;
                let s = load_stdin_with_prompt()?;
                let id = local.import_locked_id(&String::from_utf8(s)?)?;
                // Note: It's unclear how much of this should be done by
                // the library
                local.save_current_id(&id.id)?;

                let url = &id
                    .url
                    .as_ref()
                    .expect("A public id must have an associated URL");
                let proof_dir_path = local.get_proofs_dir_path_for_url(url)?;
                if !proof_dir_path.exists() {
                    let mut warnings = Vec::new();
                    local.clone_proof_dir_from_git(&url.url, false, &mut warnings)?;
                    Warning::log_all(&warnings);
                }
            }
            opts::Id::Trust(args) => {
                set_trust_level_for_ids(
                    &ids_from_string(&args.public_ids)?,
                    &args.common_proof_create,
                    args.level.unwrap_or(TrustLevel::Medium),
                    args.level.is_none(),
                    args.overrides,
                )?;
            }
            opts::Id::Untrust(args) => {
                set_trust_level_for_ids(
                    &ids_from_string(&args.public_ids)?,
                    &args.common_proof_create,
                    TrustLevel::None,
                    true,
                    args.overrides,
                )?;
            }
            opts::Id::Distrust(args) => {
                set_trust_level_for_ids(
                    &ids_from_string(&args.public_ids)?,
                    &args.common_proof_create,
                    TrustLevel::Distrust,
                    true,
                    args.overrides,
                )?;
            }
            opts::Id::Query(cmd) => match cmd {
                opts::IdQuery::Current { trust_params } => {
                    let local = Local::auto_open()?;
                    if let Some(id) = local.read_current_locked_id_opt()? {
                        let id = id.to_public_id();
                        let db = local.load_db()?;
                        let trust_set = db.calculate_trust_set(&id.id, &trust_params.into());

                        print_ids(Some(id.id).as_ref().into_iter(), &trust_set, &db);
                    }
                }
                opts::IdQuery::Own { trust_params } => {
                    let local = Local::auto_open()?;
                    if let Some(id) = local.read_current_locked_id_opt()? {
                        let id = id.to_public_id();
                        let db = local.load_db()?;
                        let trust_set = db.calculate_trust_set(&id.id, &trust_params.into());
                        print_ids(
                            local
                                .get_current_user_public_ids()?
                                .iter()
                                .map(|public_id| &public_id.id),
                            &trust_set,
                            &db,
                        );
                    }
                }
                opts::IdQuery::Trusted {
                    trust_params,
                    for_id,
                    trust_level,
                } => {
                    let local = crev_lib::Local::auto_open()?;
                    let db = local.load_db()?;
                    let for_id = local.get_for_id_from_str(for_id.as_deref())?;
                    let trust_set = db.calculate_trust_set(&for_id, &trust_params.into());

                    print_ids(
                        trust_set.iter_trusted_ids().filter(|id| {
                            trust_set.get_effective_trust_level(id)
                                >= trust_level.trust_level.into()
                        }),
                        &trust_set,
                        &db,
                    );
                }
                // TODO: move to crev-lib
                opts::IdQuery::All {
                    trust_params,
                    for_id,
                } => {
                    let local = crev_lib::Local::auto_create_or_open()?;
                    let db = local.load_db()?;
                    let for_id = local.get_for_id_from_str(for_id.as_deref())?;
                    let trust_set = db.calculate_trust_set(&for_id, &trust_params.into());

                    let mut tmp = db
                        .all_known_ids()
                        .into_iter()
                        .map(|id| {
                            let trust = trust_set.get_effective_trust_level(&id);
                            let url = db
                                .lookup_url(&id)
                                .any_unverified()
                                .map(|url| url.url.as_str());
                            (std::cmp::Reverse(trust), url, id)
                        })
                        .collect::<Vec<_>>();
                    tmp.sort();

                    print_ids(tmp.iter().map(|(_, _, id)| id), &trust_set, &db);
                }
            },
        },
        opts::Command::Trust(args) => {
            let (urls, ids): (Vec<_>, Vec<_>) = args
                .public_ids_or_urls
                .into_iter()
                .partition(|arg| arg.starts_with("https://"));
            let mut ids = ids_from_string(&ids)?;

            let local = crev_lib::Local::auto_create_or_open()?;
            let mut db = local.load_db()?;

            // Fetch the URLs
            for url in &urls {
                local.fetch_url_into(url, &mut db)?;
            }

            // Make reverse lookup for Ids based on their URLs
            let mut lookup = HashMap::new();
            for (id, _) in db.all_author_ids() {
                if let Some(url) = db.lookup_url(&id).from_self() {
                    lookup
                        .entry(url.url.as_str())
                        .or_insert_with(HashSet::new)
                        .insert(id);
                }
            }
            for url in &urls {
                if let Some(set) = lookup.remove(url.as_str()) {
                    for id in set {
                        ids.push(id);
                    }
                } else {
                    eprintln!("warning: Could not find Id for URL {url}");
                }
            }
            set_trust_level_for_ids(
                &ids,
                &args.common_proof_create,
                args.level.unwrap_or(TrustLevel::Medium),
                args.level.is_none(),
                args.overrides,
            )?;
            let mut warnings = Vec::new();
            // Make sure we have reviews for the new Ids we're trusting
            local.fetch_new_trusted(Default::default(), None, &mut warnings)?;

            // only warn about the new ids, don't scare about old problems.
            for w in &warnings {
                if let Warning::IdUrlNotKnonw(id) = w {
                    if ids.contains(id) {
                        w.log();
                    }
                }
            }
        }
        opts::Command::Crate(args) => match args {
            opts::Crate::Diff(args) => {
                let status = run_diff(&args)?;
                std::process::exit(status.code().unwrap_or(-159));
            }
            opts::Crate::Verify(opts) => {
                return deps::verify_deps(opts.crate_, opts.opts);
            }
            opts::Crate::Mvp { crate_, opts, wot } => {
                deps::crate_mvps(crate_, opts, wot)?;
            }
            opts::Crate::Info { crate_, opts, wot } => {
                info::print_crate_info(crate_.auto_unrelated()?, opts, wot)?;
            }
            opts::Crate::Goto(args) => {
                goto_crate_src(&args.auto_unrelated()?)?;
            }
            opts::Crate::Expand(args) => {
                expand_crate_src(&args.crate_.auto_unrelated()?)?;
            }
            opts::Crate::Open(args) => {
                handle_goto_mode_command(&args.common.clone(), None, |sel| {
                    crate_open(&sel.clone().auto_unrelated()?, args.cmd, args.cmd_save)
                })?;
            }
            opts::Crate::Clean(args) => {
                if args.is_empty() && are_we_called_from_goto_shell().is_none() {
                    clean_all_crates_with_digest_mismatch()?;
                } else {
                    handle_goto_mode_command(
                        &ReviewCrateSelector {
                            crate_: args.clone(),
                            diff: None,
                        },
                        None,
                        |sel| clean_crate(&sel.crate_),
                    )?;
                }
            }
            opts::Crate::Dir(args) => show_dir(&args.common.crate_.auto_unrelated()?)?,

            opts::Crate::Review(args) => crate_review(&args, TrustProofType::Trust)?,
            opts::Crate::Unreview(args) => crate_review(&args, TrustProofType::Untrust)?,
            opts::Crate::Search(args) => {
                lookup_crates(&args.query, args.count)?;
            }
        },
        opts::Command::Config(args) => match args {
            opts::Config::Dir => {
                let local = crev_lib::Local::auto_create_or_open()?;
                println!("{}", local.config_root().display());
            }
            opts::Config::DataDir => {
                let local = crev_lib::Local::auto_create_or_open()?;
                println!("{}", local.data_root().display());
            }
            opts::Config::CacheDir => {
                let local = crev_lib::Local::auto_create_or_open()?;
                println!("{}", local.cache_root().display());
            }
            opts::Config::Edit => {
                let local = crev_lib::Local::auto_create_or_open()?;
                edit::edit_user_config(&local)?;
            }
            opts::Config::Completions { shell } => {
                use structopt::clap::Shell;
                let shell = match shell
                    .unwrap_or(
                        PathBuf::from(std::env::var("SHELL")?)
                            .file_name()
                            .ok_or_else(|| format_err!("$SHELL corrupted?"))?
                            .to_string_lossy()
                            .to_string(),
                    )
                    .as_str()
                {
                    "bash" => Shell::Bash,
                    "zsh" => Shell::Zsh,
                    "powershell" => Shell::PowerShell,
                    "elvish" => Shell::Elvish,
                    "fish" => Shell::Fish,
                    other => {
                        bail!("{} shell not supported", other);
                    }
                };
                opts::Opts::clap().gen_completions_to(
                    // we have to pretend, we're generating for main cargo binary
                    "cargo",
                    shell,
                    &mut std::io::stdout(),
                );
            }
        },
        opts::Command::Repo(args) => match args {
            opts::Repo::Dir => {
                let local = crev_lib::Local::auto_create_or_open()?;
                println!("{}", local.get_proofs_dir_path()?.display());
            }
            opts::Repo::Git(git) => {
                let local = Local::auto_open()?;
                let status = local.run_git_verbose(git.args)?;
                return Ok(CommandExitStatus::CommandExitCode(
                    status.code().unwrap_or(-159),
                ));
            }
            opts::Repo::Query(args) => match args {
                opts::RepoQuery::Review(args) => list_reviews(&args.crate_)?,
                opts::RepoQuery::Advisory(args) => list_advisories(&args.crate_)?,
                opts::RepoQuery::Issue(args) => list_issues(&args)?,
            },
            opts::Repo::Publish => repo_publish()?,
            opts::Repo::Fetch(cmd) => match cmd {
                opts::RepoFetch::Trusted {
                    distance_params,
                    for_id,
                } => {
                    let local = Local::auto_create_or_open()?;
                    local.fetch_trusted(
                        distance_params.into(),
                        for_id.as_deref(),
                        &mut Warning::auto_log(),
                    )?;
                }
                opts::RepoFetch::Url(params) => {
                    let local = Local::auto_create_or_open()?;
                    local.fetch_url(&params.url)?;
                }
                opts::RepoFetch::All => {
                    let local = Local::auto_create_or_open()?;
                    info!("Fetching...");
                    local.fetch_all(&mut Warning::auto_log())?;
                }
            },
            opts::Repo::Update(args) => repo_update(args, &mut Warning::auto_log())?,
            opts::Repo::Edit(cmd) => match cmd {
                opts::RepoEdit::Readme => {
                    let local = crev_lib::Local::auto_open()?;
                    edit::edit_readme(&local)?;
                }
                opts::RepoEdit::Known => {
                    edit_known_owners_list()?;
                }
            },

            opts::Repo::Import(args) => {
                let local = Local::auto_create_or_open()?;
                let id = local.read_current_unlocked_id(&term::read_passphrase)?;

                let s = load_stdin_with_prompt()?;
                let proofs = crev_data::proof::Proof::parse_from(s.as_slice())?;
                let commit_msg = "Import proofs";

                for proof in proofs {
                    let now = crev_common::now();
                    match dyn_proof::parse_dyn_content(&proof) {
                        Ok(mut content) => {
                            if args.reset_date {
                                content.set_date(&now);
                            }
                            content.set_author(id.as_public_id());
                            let proof = content.sign_by(&id)?;
                            maybe_store(&local, &proof, commit_msg, &args.common)?;
                        }
                        Err(e) => {
                            eprintln!("Ignoried unknown proof - {e}");
                        }
                    }
                }
            }
        },
        opts::Command::Proof(args) => match args {
            opts::Proof::Find(args) => {
                proof_find(args)?;
            }
            opts::Proof::Reissue(args) => {
                proof_reissue(args)?;
            }
        },
        opts::Command::Goto(args) => {
            goto_crate_src(&args.auto_unrelated()?)?;
        }
        opts::Command::Open(args) => {
            handle_goto_mode_command(&args.common.clone(), None, |crate_| {
                crate_open(&crate_.clone().auto_unrelated()?, args.cmd, args.cmd_save)
            })?;
        }
        opts::Command::Publish => repo_publish()?,
        opts::Command::Review(args) => crate_review(&args, TrustProofType::Trust)?,
        opts::Command::Update(args) => repo_update(args, &mut Warning::auto_log())?,

        opts::Command::Wot(args) => match args {
            opts::Wot::Log { wot } => {
                crate::wot::print_log(wot)?;
            }
        },
        opts::Command::Verify(opts) => {
            return deps::verify_deps(opts.crate_, opts.opts);
        }
    }

    Ok(CommandExitStatus::Success)
}

fn validate_public_repo_url(url: &str) -> Result<()> {
    if !url.starts_with("https://") {
        bail!("Proof repositories are for sharing reviews publicly, therefore they must be 'https://' git URLs\n\
        If you need to use a different URL for pushing to the repository, you may change it later with\n
        cargo crev repo git remote set-url --push origin <url>");
    }
    Ok(())
}

fn current_id_set_url(url: &str, use_https_push: bool) -> Result<(), crev_lib::Error> {
    let local = Local::auto_open()?;
    let mut locked_id = local.read_current_locked_id()?;
    let pub_id = locked_id.to_public_id().id;
    local.change_locked_id_url(
        &mut locked_id,
        url,
        use_https_push,
        &mut Warning::auto_log(),
    )?;
    local.save_current_id(&pub_id)?;
    local.fetch_trusted(
        opts::TrustDistanceParams::default().into(),
        None,
        &mut Warning::auto_log(),
    )?;

    if locked_id.has_no_passphrase() {
        eprintln!("warning: there is no passphrase set. Use `cargo crev id passwd` to fix.");
    }
    Ok(())
}

/// Interactive process of setting up a new `CrevID`
fn generate_new_id_interactively(url: Option<&str>, use_https_push: bool) -> Result<()> {
    // Avoid creating new CrevID if it's not necessary
    if let Ok(local) = Local::auto_open() {
        if let Ok(existing) = local.get_current_user_public_ids() {
            let existing_usable = existing
                .iter()
                .filter(|id| id.url.is_some())
                .collect::<Vec<_>>();
            if !existing_usable.is_empty() {
                for id in &existing_usable {
                    eprintln!(
                        "warning: you already have a CrevID {} {}",
                        id.id,
                        id.url_display()
                    );
                }
            }

            // only try configuring existing Id if there is a URL to set,
            // otherwise it'd remain in the unconfigured limbo
            if let Some(url) = url {
                validate_public_repo_url(url)?;

                let reusable_id = existing
                    .iter()
                    .filter(|id| id.url.is_none())
                    .filter_map(|id| local.read_locked_id(&id.id).ok())
                    .find(|id| id.has_no_passphrase());
                if let Some(mut locked_id) = reusable_id {
                    let id = locked_id.to_public_id().id;
                    eprintln!(
                        "Instead of setting up a new CrevID we'll reconfigure the existing one {id}"
                    );
                    local.change_locked_id_url(
                        &mut locked_id,
                        url,
                        use_https_push,
                        &mut Warning::auto_log(),
                    )?;
                    let unlocked_id = local.read_unlocked_id(&id, &|| Ok(String::new()))?;
                    change_passphrase(&local, &unlocked_id, &read_new_passphrase()?)?;
                    local.save_current_id(&id)?;
                    return Ok(());
                }
            }

            // if an old one couldn't be reconfigured automatically, help how to do it manually
            if let Some(example) = existing_usable.get(0) {
                if local
                    .get_current_userid()
                    .ok()
                    .map_or(false, |cur| cur == example.id)
                {
                    eprintln!("You can configure the existing CrevID with `cargo crev set-url` and `cargo crev id passwd`\n");
                } else {
                    eprintln!(
                        "You can use existing CrevID with `cargo crev id switch {}`",
                        example.id
                    );
                    eprintln!(
                        "and set it up with `cargo crev set-url` and `cargo crev id passwd`\n"
                    );
                }
            }
        }
    }

    if url.is_none() {
        print_crev_proof_repo_fork_help();
        bail!("Then again with `cargo crev id new --url <new repo URL>`\nor `cargo crev id new --github-username <you>`");
    }

    let local = Local::auto_create_or_open()?;
    let res = local
        .generate_id(
            url,
            use_https_push,
            read_new_passphrase,
            &mut Warning::auto_log(),
        )
        .map_err(|e| {
            print_crev_proof_repo_fork_help();
            e
        })?;
    if !res.has_no_passphrase() {
        println!("Your CrevID was created and will be printed below in an encrypted form.");
        println!("Make sure to back it up on another device, to prevent losing it.");
        println!("{res}");
    } else {
        println!("Your CrevID is not protected with a passphrase. You should fix that with `cargo crev id passwd`");
    }

    let local = crev_lib::Local::auto_open()?;
    let _ = ensure_known_owners_list_exists(&local);
    Ok(())
}

fn set_trust_level_for_ids(
    ids: &[Id],
    common_proof_create: &crate::opts::CommonProofCreate,
    trust_level: TrustLevel,
    edit_interactively: bool,
    show_override_suggestions: bool,
) -> Result<()> {
    let local = ensure_crev_id_exists_or_make_one()?;
    let unlocked_id = local.read_current_unlocked_id(&term::read_passphrase)?;

    let overrides = if ids.len() == 1 {
        let db = local.load_db()?;

        db.get_trust_proof_between(&unlocked_id.id.id, &ids[0])
            .map(|trust_proof| trust_proof.override_.clone())
            .unwrap_or_default()
    } else {
        vec![]
    };

    let mut trust = local.build_trust_proof(
        unlocked_id.as_public_id(),
        ids.to_vec(),
        trust_level,
        overrides,
    )?;

    if edit_interactively {
        let extra_comment = if trust_level == TrustLevel::Distrust {
            Some("WARNING: Distrust has severe consequences. Read documentation below.")
        } else {
            None
        };
        trust = edit::edit_proof_content_iteractively(&trust, None, None, extra_comment, |text| {
            if show_override_suggestions && trust.override_.is_empty() {
                writeln!(text, "# override:")?;
            }

            if show_override_suggestions {
                let db = local.load_db()?;
                for (id, trust_level) in ids.iter().flat_map(|id| db.get_reverse_trust_for(id)) {
                    let (status, url) = url_to_status_str(&db.lookup_url(id));
                    writeln!(text, "# - id-type: crev")?; // TODO: support other ids?
                    writeln!(text, "#   id: {id} # level: {trust_level}")?;
                    writeln!(text, "#   url: {url} # {status}")?;
                    writeln!(text, "#   comment: \"\"")?;
                }
            }

            Ok(())
        })?;
    }

    trust.touch_date();
    let proof = trust.sign_by(&unlocked_id)?;

    if common_proof_create.print_unsigned {
        print!("{}", proof.body());
    }
    if common_proof_create.print_signed {
        print!("{proof}");
    }
    if !common_proof_create.no_store {
        crev_lib::proof::store_id_trust_proof(
            &proof,
            ids,
            trust_level,
            !common_proof_create.no_commit,
        )?;
    }
    Ok(())
}

fn ensure_crev_id_exists_or_make_one() -> Result<Local> {
    let local = Local::auto_create_or_open()?;

    if local.get_current_userid().is_err() {
        let existing = local.get_current_user_public_ids().unwrap_or_default();
        if existing.is_empty() {
            eprintln!(
                "note: Setting up a default CrevID. Run `cargo crev id new` to customize it."
            );
            local.generate_id(None, false, || Ok(String::new()), &mut Warning::auto_log())?;
        } else {
            eprintln!("You need to select current CrevID. Try:");
            for id in existing {
                eprintln!("`cargo crev id switch {}`", id.id);
            }
            eprintln!("or `cargo crev id new` to create a new one");
        }
    }
    Ok(local)
}

fn ids_from_string(id_strings: &[String]) -> Result<Vec<Id>> {
    id_strings
        .iter()
        .map(|s| match Id::crevid_from_str(s) {
            Ok(s) => Ok(s),
            Err(e) => bail!("'{}' is not a valid crev Id: {}", s, e),
        })
        .collect()
}

fn load_stdin_with_prompt() -> Result<Vec<u8>> {
    let term = term::Term::new();

    if term.is_input_interactive() {
        eprintln!("Paste in the text and press Ctrl+D.");
    }
    let mut s = vec![];

    std::io::stdin().lock().read_until(0, &mut s)?;
    Ok(s)
}

fn print_crev_proof_repo_fork_help() {
    eprintln!("Each CrevID is associated with a public git repository which stores reviews and trust proofs.");
    eprintln!(
        "To create your proof repository, fork the template:\n\
    https://github.com/crev-dev/crev-proofs/fork\n\n\
    For help visit: https://github.com/crev-dev/crev/wiki/Proof-Repository\n"
    );
}

fn read_new_passphrase() -> io::Result<String> {
    println!("CrevID will be protected by a passphrase.");
    println!("You can change it later with `cargo crev id passwd`.");
    println!("There's no way to recover your CrevID if you forget your passphrase.");
    term::read_new_passphrase()
}

fn current_id_change_passphrase() -> Result<LockedId> {
    let local = Local::auto_open()?;
    eprintln!(
        "Please enter the OLD passphrase. If you don't know it, you will need to create a new Id."
    );
    let unlocked_id = local.read_current_unlocked_id(&term::read_passphrase)?;
    eprintln!("Now please enter the NEW passphrase.");
    change_passphrase(&local, &unlocked_id, &term::read_new_passphrase()?)
}

fn change_passphrase(
    local: &Local,
    unlocked_id: &UnlockedId,
    passphrase: &str,
) -> Result<LockedId> {
    let locked_id = LockedId::from_unlocked_id(unlocked_id, passphrase)?;

    local.save_locked_id(&locked_id)?;
    local.save_current_id(unlocked_id.as_ref())?;

    if locked_id.has_no_passphrase() {
        eprintln!("Passphrase disabled.");
    } else {
        eprintln!("Passphrase changed successfully.");
    }

    println!("Your CrevID has been updated and will be printed below in the reencrypted form.");
    println!("Make sure to back it up on another device, to prevent losing it.");
    println!("{locked_id}");

    Ok(locked_id)
}

fn main() {
    let mut builder = env_logger::builder();
    let default_log_settings = std::env::var_os("RUST_LOG").is_none();
    if default_log_settings {
        builder
            .filter_level(log::LevelFilter::Off)
            .filter_module("crev_wot", log::LevelFilter::Info)
            .filter_module("crev_lib", log::LevelFilter::Info)
            .filter_module("crev_data", log::LevelFilter::Info)
            .filter_module("crev_common", log::LevelFilter::Info)
            .filter_module("cargo_crev", log::LevelFilter::Info);
    }
    builder.parse_default_env();
    if default_log_settings {
        builder
            .filter_module("cargo", log::LevelFilter::Off)
            .filter_module("tokei", log::LevelFilter::Off)
            .filter_module("ignore", log::LevelFilter::Off)
            .filter_module("globset", log::LevelFilter::Off)
            .filter_module("reqwest", log::LevelFilter::Off);
    }

    builder.format(|buf, record| {
            if record.level() == log::Level::Info {
                writeln!(buf, "{}", record.args())
            } else if record.level() > log::Level::Info {
                writeln!(
                    buf,
                    "[{}:{}] {}",
                    record
                        .module_path()
                        .or_else(|| record.file())
                        .unwrap_or("?"),
                    record.line().unwrap_or(0),
                    record.args()
                )
            } else {
                writeln!(buf, "{}: {}", record.level(), record.args())
            }
        })
        .init();
    debug!("Starting cargo-crev");
    let opts = opts::Opts::from_args();
    let opts::MainCommand::Crev(command) = opts.command;
    handle_command_result_and_panics(|| run_command(command))
}

fn is_possibly_broken_pipe_msg(s: &str) -> bool {
    s.contains("Broken pipe") || s.contains("os error 32")
}

/**
 * Handle command exit code and broken pipe IO errors.
 *
 * Set appropriate error code to the execution result.
 *
 * Broken pipe usually means that the user left `less` or some other pager
 * and it's best to ignore such errors in both results and panics.
 */
// See https://github.com/crev-dev/cargo-crev/issues/287
fn handle_command_result_and_panics(
    f: impl FnOnce() -> Result<CommandExitStatus> + panic::UnwindSafe,
) -> ! {
    let hook = panic::take_hook();

    // skip printing panic msg on broken pipe panics
    panic::set_hook(Box::new(move |panic_info| {
        if !is_possibly_broken_pipe_msg(&panic_info.to_string()) {
            (hook)(panic_info);
        }
    }));

    if let Err(panic_err) = panic::catch_unwind(|| match (f)() {
        Ok(CommandExitStatus::Success) => {}
        Ok(CommandExitStatus::VerificationFailed) => std::process::exit(-1),
        Ok(CommandExitStatus::CommandExitCode(code)) => std::process::exit(code),
        Err(e) => {
            if let Some(io_error) = e.root_cause().downcast_ref::<std::io::Error>() {
                if io_error.kind() == std::io::ErrorKind::BrokenPipe {
                    return;
                }
            }
            eprintln!("{e:?}");
            std::process::exit(-2)
        }
    }) {
        let panic_str = if let Some(io_error) = panic_err.downcast_ref::<Box<&'static str>>() {
            io_error.to_string()
        } else if let Some(io_error) = panic_err.downcast_ref::<String>() {
            io_error.to_string()
        } else if let Some(io_error) = panic_err.downcast_ref::<&'static str>() {
            (*io_error).to_string()
        } else {
            String::new()
        };

        if !is_possibly_broken_pipe_msg(&panic_str) {
            panic::resume_unwind(panic_err);
        }
    }
    std::process::exit(0)
}
