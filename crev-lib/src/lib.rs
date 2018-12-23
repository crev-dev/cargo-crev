#[macro_use]
extern crate serde_derive;
extern crate term;

use common_failures::prelude::*;

#[macro_use]
extern crate failure;

pub mod id;
pub mod local;
pub mod proof;
pub mod repo;
pub mod staging;
pub mod trustdb;
pub mod util;

pub use self::local::Local;
use crev_data::Digest;
use crev_data::Id;
use std::convert::AsRef;
use std::{
    collections::HashSet,
    fmt,
    path::{Path, PathBuf},
};

/// Trait representing a place that can keep proofs
///
/// Typically serialized and persisted.
pub trait ProofStore {
    fn insert(&self, proof: &crev_data::proof::Proof) -> Result<()>;
    fn proofs_iter(&self) -> Result<Box<dyn Iterator<Item = crev_data::proof::Proof>>>;
}

#[derive(Copy, Clone)]
pub enum TrustOrDistrust {
    Trust,
    Distrust,
}

impl TrustOrDistrust {
    pub fn is_trust(self) -> bool {
        if let TrustOrDistrust::Trust = self {
            return true;
        }
        false
    }

    pub fn to_review(self) -> crev_data::Review {
        use self::TrustOrDistrust::*;
        match self {
            Trust => crev_data::Review::new_positive(),
            Distrust => crev_data::Review::new_negative(),
        }
    }
}

/// Result of verification
///
/// Not named `Result` to avoid confusion with `Result` type.
pub enum VerificationStatus {
    Verified,
    Unknown,
    Flagged,
}

impl VerificationStatus {

    fn set_term_color(&self, t: &mut Box<term::StdoutTerminal>) -> Result<()> {
        if !t.supports_color() {
            return Ok(());
        }

        match *self {
            VerificationStatus::Verified => {
                t.fg(term::color::GREEN)?;
            },
            VerificationStatus::Flagged => {
                t.fg(term::color::RED)?;
            },
            _ => {}
        }
        Ok(())
    }

    pub fn write_colored_to_stdout(&self) -> Result<()> {
        match term::stdout() {
            Some(ref mut t) => {
                self.set_term_color(t)?;
                write!(t, "{:8}", *self)?;
                t.reset()?;
            }
            None => {
                print!("{:8}", *self);
            }
        }
        Ok(())
    }
}

impl fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerificationStatus::Verified => f.pad("verified"),
            VerificationStatus::Unknown => f.pad("unknown"),
            VerificationStatus::Flagged => f.pad("flagged"),
        }
    }
}

pub fn dir_or_git_repo_verify<H1, H2>(
    path: &Path,
    ignore_list: &HashSet<PathBuf, H1>,
    db: &trustdb::TrustDB,
    trusted_set: &HashSet<Id, H2>,
) -> Result<crate::VerificationStatus>
where
    H1: std::hash::BuildHasher + std::default::Default,
    H2: std::hash::BuildHasher + std::default::Default,
{
    let digest = if path.join(".git").exists() {
        get_recursive_digest_for_git_dir(path, ignore_list)?
    } else {
        Digest::from_vec(crev_recursive_digest::get_recursive_digest_for_dir::<
            crev_common::Blake2b256,
            H1,
        >(path, ignore_list)?)
    };

    Ok(db.verify_digest(&digest, trusted_set))
}

pub fn dir_verify<H1, H2>(
    path: &Path,
    ignore_list: &HashSet<PathBuf, H1>,
    db: &trustdb::TrustDB,
    trusted_set: &HashSet<Id, H2>,
) -> Result<crate::VerificationStatus>
where
    H1: std::hash::BuildHasher + std::default::Default,
    H2: std::hash::BuildHasher + std::default::Default,
{
    let digest = Digest::from_vec(crev_recursive_digest::get_recursive_digest_for_dir::<
        crev_common::Blake2b256,
        H1,
    >(path, ignore_list)?);
    Ok(db.verify_digest(&digest, trusted_set))
}

pub fn get_dir_digest<H1>(path: &Path, ignore_list: &HashSet<PathBuf, H1>) -> Result<Digest>
where
    H1: std::hash::BuildHasher + std::default::Default,
{
    Ok(Digest::from_vec(
        crev_recursive_digest::get_recursive_digest_for_dir::<crev_common::Blake2b256, H1>(
            path,
            ignore_list,
        )?,
    ))
}

pub fn show_current_id() -> Result<()> {
    let local = Local::auto_open()?;
    let id = local.read_current_locked_id()?;
    let id = id.to_pubid();
    println!("{} {}", id.id, id.url.url);
    Ok(())
}

pub fn get_recursive_digest_for_git_dir<H>(
    root_path: &Path,
    ignore_list: &HashSet<PathBuf, H>,
) -> Result<Digest>
where
    H: std::hash::BuildHasher + std::default::Default,
{
    let git_repo = git2::Repository::open(root_path)?;

    let mut status_opts = git2::StatusOptions::new();
    let mut paths = HashSet::default();

    status_opts.include_unmodified(true);
    status_opts.include_untracked(false);
    for entry in git_repo.statuses(Some(&mut status_opts))?.iter() {
        let entry_path = PathBuf::from(
            entry
                .path()
                .ok_or_else(|| format_err!("Git entry without a path"))?,
        );
        if ignore_list.contains(&entry_path) {
            continue;
        };

        paths.insert(entry_path);
    }

    Ok(Digest::from_vec(
        crev_recursive_digest::get_recursive_digest_for_paths::<crev_common::Blake2b256, H>(
            root_path, paths,
        )?,
    ))
}

pub fn get_recursive_digest_for_paths<H>(
    root_path: &Path,
    paths: HashSet<PathBuf, H>,
) -> Result<Vec<u8>>
where
    H: std::hash::BuildHasher,
{
    Ok(crev_recursive_digest::get_recursive_digest_for_paths::<
        crev_common::Blake2b256,
        H,
    >(root_path, paths)?)
}

pub fn get_recursive_digest_for_dir<H>(
    root_path: &Path,
    rel_path_ignore_list: &HashSet<PathBuf, H>,
) -> Result<Digest>
where
    H: std::hash::BuildHasher,
{
    Ok(Digest::from_vec(
        crev_recursive_digest::get_recursive_digest_for_dir::<crev_common::Blake2b256, H>(
            root_path,
            rel_path_ignore_list,
        )?,
    ))
}

pub fn generate_id(
    url: Option<String>,
    github_username: Option<String>,
    use_https_push: bool,
) -> Result<()> {
    let url = match (url, github_username) {
        (Some(url), None) => url,
        (None, Some(username)) => format!("https://github.com/{}/crev-proofs", username),
        (Some(_), Some(_)) => bail!("Can't provide both username and url"),
        (None, None) => bail!("Must provide github username or url"),
    };

    if !url.starts_with("https://") {
        bail!("URL must start with 'https://");
    }

    let local = Local::auto_create_or_open()?;
    local.clone_proof_dir_from_git(&url, use_https_push)?;

    let id = crev_data::id::OwnId::generate(crev_data::Url::new_git(url.clone()));
    eprintln!("CrevID will be protected by a passphrase.");
    eprintln!("There's no way to recover your CrevID if you forget your passphrase.");
    let passphrase = crev_common::read_new_passphrase()?;
    let locked = id::LockedId::from_own_id(&id, &passphrase)?;

    local.save_locked_id(&locked)?;
    local.save_current_id(id.as_ref())?;

    eprintln!("");
    eprintln!("Your CrevID was created and will be printed below in an encrypted form.");
    eprintln!("Make sure to back it up on another device, to prevent loosing it.");

    eprintln!("");
    println!("{}", locked);

    local.init_readme_using_this_repo_file()?;

    Ok(())
}

pub fn switch_id(id_str: &str) -> Result<()> {
    let id: Id = Id::crevid_from_str(id_str)?;
    let local = Local::auto_open()?;
    local.save_current_id(&id)?;

    Ok(())
}

pub fn list_own_ids() -> Result<()> {
    let local = Local::auto_open()?;
    for id in local.list_ids()? {
        println!("{} {}", id.id, id.url.url);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
