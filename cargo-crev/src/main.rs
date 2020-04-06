//! `cargo-crev` - `crev` ecosystem fronted for Rusti (`cargo` integration)
//!
#![type_length_limit = "1932159"]
#![cfg_attr(
    feature = "documentation",
    doc = "See [user documentation module](./doc/user/index.html)."
)]
#![cfg_attr(feature = "documentation", feature(external_doc))]
use self::prelude::*;

use crev_common::convert::OptionDeref;
use crev_lib::{self, local::Local};
use std::{io::BufRead, path::PathBuf};
use structopt::StructOpt;

#[cfg(feature = "documentation")]
/// Documentation
pub mod doc;

mod crates_io;
mod deps;
mod dyn_proof;
mod info;
mod opts;
mod prelude;
mod repo;
mod review;
mod shared;
mod term;
mod tokei;
mod tui;

use crate::{repo::*, review::*, shared::*};
use crev_data::{proof, Id};
use crev_lib::{
    proofdb::{ProofDB, TrustSet},
    TrustProofType::{self, *},
};

pub fn repo_publish() -> Result<()> {
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

fn repo_update(args: opts::Update) -> Result<()> {
    let local = Local::auto_open()?;
    let status = local.run_git(vec!["pull".into(), "--rebase".into()])?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(-159));
    }
    local.fetch_trusted(opts::TrustDistanceParams::default().into(), None)?;
    let repo = Repo::auto_open_cwd(args.cargo_opts)?;
    repo.update_source()?;
    repo.update_counts()?;
    Ok(())
}

pub fn proof_find(args: opts::ProofFind) -> Result<()> {
    let local = crev_lib::Local::auto_open()?;
    let db = local.load_db()?;
    let mut iter = Box::new(db.get_pkg_reviews_for_source(PROJECT_SOURCE_CRATES_IO))
        as Box<dyn Iterator<Item = &proof::review::Package>>;

    if let Some(author) = args.author.as_ref() {
        let id = crev_data::id::Id::crevid_from_str(author)?;
        iter = Box::new(iter.filter(move |r| r.common.from.id == id));
    }

    if let Some(crate_) = args.crate_.as_ref() {
        iter = Box::new(iter.filter(move |r| &r.package.id.id.name == crate_));
        if let Some(version) = args.version.as_ref() {
            iter = Box::new(iter.filter(move |r| &r.package.id.version == version));
        }
    }
    for review in iter {
        println!("---\n{}", review);
    }

    Ok(())
}

fn crate_review(args: opts::CrateReview) -> Result<()> {
    handle_goto_mode_command(&args.common, |sel| {
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
                TrustProofType::Trust
            },
            &args.common_proof_create,
            &args.diff,
            args.skip_activity_check || is_advisory || args.issue,
            args.cargo_opts.clone(),
        )
    })?;

    Ok(())
}

pub fn cargo_registry_to_crev_source_id(source_id: &cargo::core::SourceId) -> String {
    let s = source_id.into_url().to_string();
    if &s == "registry+https://github.com/rust-lang/crates.io-index" {
        crate::PROJECT_SOURCE_CRATES_IO.into()
    } else {
        s
    }
}

pub fn cargo_pkg_id_to_crev_pkg_id(id: &cargo::core::PackageId) -> proof::PackageVersionId {
    proof::PackageVersionId {
        id: proof::PackageId {
            source: cargo_registry_to_crev_source_id(&id.source_id()),
            name: id.name().to_string(),
        },
        version: id.version().to_owned(),
    }
}

fn print_ids<'a>(
    ids: impl Iterator<Item = &'a Id>,
    trust_set: &TrustSet,
    db: &ProofDB,
) -> Result<()> {
    for id in ids {
        let mut tmp = String::new();
        println!(
            "{} {:6} {}",
            id,
            trust_set.get_effective_trust_level(id),
            db.lookup_verified_url(id).map(|url| url.url.as_str()).or_else(|| {
                db.lookup_unverified_url(id).map(|url| {
                    tmp = format!("({})", url.url);
                    tmp.as_str()
                })
            })
            .unwrap_or("")
        );
    }
    Ok(())
}

fn run_command(command: opts::Command) -> Result<CommandExitStatus> {
    match command {
        opts::Command::Id(args) => match args {
            opts::Id::New(args) => {
                let local = Local::auto_create_or_open()?;
                let res = local.generate_id(args.url, args.github_username, args.use_https_push);
                if res.is_err() {
                    eprintln!(
                        "Visit https://github.com/crev-dev/crev/wiki/Proof-Repository for help."
                    );
                }
                let local = crev_lib::Local::auto_open()?;
                let _ = ensure_known_owners_list_exists(&local);
                res?;
            }
            opts::Id::Switch(args) => {
                let local = Local::auto_open()?;
                local.switch_id(&args.id)?
            }
            opts::Id::Current => {
                let local = Local::auto_open()?;
                local.show_own_ids()?;
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

                let url = &id.url.as_ref().expect("own Id must have a URL");
                let proof_dir_path = local.get_proofs_dir_path_for_url(url)?;
                if !proof_dir_path.exists() {
                    local.clone_proof_dir_from_git(&url.url, false)?;
                }
            }
            opts::Id::Trust(args) => {
                create_trust_proof(args.pub_ids, Trust, &args.common_proof_create)?;
            }
            opts::Id::Untrust(args) => {
                create_trust_proof(args.pub_ids, Untrust, &args.common_proof_create)?;
            }
            opts::Id::Distrust(args) => {
                create_trust_proof(args.pub_ids, Distrust, &args.common_proof_create)?;
            }
            opts::Id::Query(cmd) => match cmd {
                opts::IdQuery::Current { trust_params } => {
                    let local = Local::auto_open()?;
                    if let Some(id) = local.read_current_locked_id_opt()? {
                        let id = id.to_pubid();
                        let db = local.load_db()?;
                        let trust_set = db.calculate_trust_set(&id.id, &trust_params.into());

                        print_ids(Some(id.id).as_ref().into_iter(), &trust_set, &db)?;
                    }
                }
                opts::IdQuery::Own { trust_params } => {
                    let local = Local::auto_open()?;
                    if let Some(id) = local.read_current_locked_id_opt()? {
                        let id = id.to_pubid();
                        let db = local.load_db()?;
                        let trust_set = db.calculate_trust_set(&id.id, &trust_params.into());
                        // local.list_own_ids()?
                        print_ids(
                            local.list_ids()?.iter().map(|pub_id| &pub_id.id),
                            &trust_set,
                            &db,
                        )?;
                    }
                }
                opts::IdQuery::Trusted {
                    trust_params,
                    for_id,
                    trust_level,
                } => {
                    let local = crev_lib::Local::auto_open()?;
                    let db = local.load_db()?;
                    let for_id = local.get_for_id_from_str(OptionDeref::as_deref(&for_id))?;
                    let trust_set = db.calculate_trust_set(&for_id, &trust_params.into());

                    print_ids(
                        trust_set.trusted_ids().filter(|id| {
                            trust_set.get_effective_trust_level(id)
                                >= trust_level.trust_level.into()
                        }),
                        &trust_set,
                        &db,
                    )?;
                }
                // TODO: move to crev-lib
                opts::IdQuery::All {
                    trust_params,
                    for_id,
                } => {
                    let local = crev_lib::Local::auto_create_or_open()?;
                    let db = local.load_db()?;
                    let for_id = local.get_for_id_from_str(OptionDeref::as_deref(&for_id))?;
                    let trust_set = db.calculate_trust_set(&for_id, &trust_params.into());

                    print_ids(db.all_known_ids().iter(), &trust_set, &db)?;
                }
            },
        },

        opts::Command::Crate(args) => match args {
            opts::Crate::Diff(args) => {
                let status = run_diff(&args)?;
                std::process::exit(status.code().unwrap_or(-159));
            }
            opts::Crate::Verify { crate_, opts } => {
                return if opts.interactive {
                    tui::verify_deps(crate_, opts)
                } else {
                    deps::verify_deps(crate_, opts)
                };
            }
            opts::Crate::Mvp { crate_, opts } => {
                deps::crate_mvps(crate_, opts)?;
            }
            opts::Crate::Info { crate_, opts } => {
                info::print_crate_info(crate_, opts)?;
            }
            opts::Crate::Goto(args) => {
                goto_crate_src(&args.crate_)?;
            }
            opts::Crate::Open(args) => {
                handle_goto_mode_command(&args.common.clone(), |sel| {
                    crate_open(sel, args.cmd, args.cmd_save)
                })?;
            }
            opts::Crate::Clean(args) => {
                if args.crate_.is_empty() && are_we_called_from_goto_shell().is_none() {
                    clean_all_unclean_crates()?;
                } else {
                    handle_goto_mode_command(&args, |sel| clean_crate(sel))?;
                }
            }
            opts::Crate::Dir(args) => show_dir(&args.common.crate_)?,

            opts::Crate::Review(args) => crate_review(args)?,
            opts::Crate::Unreview(args) => {
                handle_goto_mode_command(&args.common, |sel| {
                    let is_advisory = args.advisory
                        || args.affected.is_some()
                        || (!args.issue && args.severity.is_some());
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
                                affected: args.affected.unwrap_or(
                                    crev_data::proof::review::package::VersionRange::Major,
                                ),
                            })
                        } else {
                            None
                        },
                        if is_advisory || args.issue {
                            TrustProofType::Distrust
                        } else {
                            TrustProofType::Untrust
                        },
                        &args.common_proof_create,
                        &args.diff,
                        args.skip_activity_check || is_advisory || args.issue,
                        args.cargo_opts.clone(),
                    )
                })?;
            }
            opts::Crate::Search(args) => {
                lookup_crates(&args.query, args.count)?;
            }
        },
        opts::Command::Config(args) => match args {
            opts::Config::Dir => {
                let local = crev_lib::Local::auto_create_or_open()?;
                println!("{}", local.get_root_path().display());
            }
            opts::Config::Edit => {
                let local = crev_lib::Local::auto_create_or_open()?;
                local.edit_user_config()?;
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
                let status = local.run_git(git.args)?;
                std::process::exit(status.code().unwrap_or(-159));
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
                    local.fetch_trusted(distance_params.into(), OptionDeref::as_deref(&for_id))?;
                }
                opts::RepoFetch::Url(params) => {
                    let local = Local::auto_create_or_open()?;
                    local.fetch_url(&params.url)?;
                }
                opts::RepoFetch::All => {
                    let local = Local::auto_create_or_open()?;
                    local.fetch_all()?;
                }
            },
            opts::Repo::Update(args) => repo_update(args)?,
            opts::Repo::Edit(cmd) => match cmd {
                opts::RepoEdit::Readme => {
                    let local = crev_lib::Local::auto_open()?;
                    local.edit_readme()?;
                }
                opts::RepoEdit::Known => {
                    edit_known_owners_list()?;
                }
            },

            opts::Repo::Import(args) => {
                let local = Local::auto_create_or_open()?;
                let id = local.read_current_unlocked_id(&crev_common::read_passphrase)?;

                let s = load_stdin_with_prompt()?;
                let proofs = crev_data::proof::Proof::parse_from(s.as_slice())?;
                let commit_msg = "Import proofs";

                for proof in proofs {
                    let now = crev_common::now();
                    match self::dyn_proof::parse_dyn_content(&proof) {
                        Ok(mut content) => {
                            if args.reset_date {
                                content.set_date(&now);
                            }
                            content.set_author(&id.as_pubid());
                            let proof = content.sign_by(&id)?;
                            maybe_store(&local, &proof, &commit_msg, &args.common)?;
                        }
                        Err(e) => {
                            eprintln!("Ignoried unknwon proof - {}", e);
                        }
                    }
                }
            }
        },
        opts::Command::Proof(args) => match args {
            opts::Proof::Find(args) => {
                proof_find(args)?;
            }
        },
        opts::Command::Goto(args) => {
            goto_crate_src(&args.crate_)?;
        }
        opts::Command::Open(args) => {
            handle_goto_mode_command(&args.common.clone(), |sel| {
                crate_open(sel, args.cmd, args.cmd_save)
            })?;
        }
        opts::Command::Publish => repo_publish()?,
        opts::Command::Review(args) => crate_review(args)?,
        opts::Command::Update(args) => repo_update(args)?,

        opts::Command::Verify { crate_, opts } => {
            return if opts.interactive {
                tui::verify_deps(crate_, opts)
            } else {
                deps::verify_deps(crate_, opts)
            };
        }
    }

    Ok(CommandExitStatus::Success)
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
    env_logger::init();
    let opts = opts::Opts::from_args();
    let opts::MainCommand::Crev(command) = opts.command;
    match run_command(command) {
        Ok(CommandExitStatus::Success) => {}
        Ok(CommandExitStatus::VerificationFailed) => std::process::exit(-1),
        Err(e) => {
            eprintln!("{}", e.display_causes_and_backtrace());
            std::process::exit(-2)
        }
    }
}
