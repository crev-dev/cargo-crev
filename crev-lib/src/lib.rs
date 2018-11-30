#[macro_use]
extern crate serde_derive;

use common_failures::prelude::*;

#[macro_use]
extern crate failure;

pub mod id;
pub mod local;
pub mod proof;
pub mod recursive_digest;
pub mod repo;
pub mod staging;
pub mod trustdb;

pub mod util;

pub use self::local::Local;
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
    fn proofs_iter(&self) -> Box<dyn Iterator<Item = crev_data::proof::Proof>>;
}

/// Result of verification
///
/// Not named `Result` to avoid confusion with `Result` type.
pub enum VerificationStatus {
    Trusted,
    NotTrusted,
    Distrusted,
}

use crev_data::Id;

impl fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerificationStatus::Trusted => f.write_str("trusted"),
            VerificationStatus::NotTrusted => f.write_str("not trusted"),
            VerificationStatus::Distrusted => f.write_str("distrusted"),
        }
    }
}

pub fn calculate_recursive_digest_for_dir(
    root_path: &Path,
    ignore_list: HashSet<PathBuf>,
) -> Result<Vec<u8>> {
    let mut hasher = recursive_digest::RecursiveHasher::new_dir(root_path.into());

    hasher.set_ignore_list(ignore_list);

    for entry in walkdir::WalkDir::new(root_path) {
        let entry = entry.unwrap();
        let path = entry
            .path()
            .strip_prefix(&root_path)
            .unwrap_or_else(|_| entry.path());
        hasher.insert_path(path)
    }

    Ok(hasher.get_digest()?)
}

pub fn calculate_recursive_digest_for_git_dir(
    root_path: &Path,
    ignore_list: HashSet<PathBuf>,
) -> Result<Vec<u8>> {
    let git_repo = git2::Repository::open(root_path)?;

    let mut hasher = recursive_digest::RecursiveHasher::new_dir(root_path.to_owned());

    hasher.set_ignore_list(ignore_list);

    let mut status_opts = git2::StatusOptions::new();
    status_opts.include_unmodified(true);
    status_opts.include_untracked(false);
    for entry in git_repo.statuses(Some(&mut status_opts))?.iter() {
        hasher.insert_path(&PathBuf::from(
            entry
                .path()
                .ok_or_else(|| format_err!("Git entry without a path"))?,
        ))
    }

    Ok(hasher.get_digest()?)
}
pub fn dir_verify(
    path: &Path,
    ignore_list: HashSet<PathBuf>,
    db: &trustdb::TrustDB,
    trusted_set: &HashSet<Id>,
) -> Result<crate::VerificationStatus> {
    let digest = if path.join(".git").exists() {
        calculate_recursive_digest_for_git_dir(path, ignore_list)?
    } else {
        calculate_recursive_digest_for_dir(path, ignore_list)?
    };
    Ok(db.verify_digest(&digest, trusted_set))
}

pub fn show_id() -> Result<()> {
    let local = Local::auto_open()?;
    let id = local.read_locked_id()?;
    let id = id.to_pubid();
    print!("{}", id.id);
    Ok(())
}

pub fn generate_id() -> Result<()> {
    eprintln!("Crev relies on personal, publicly accessible repositories to circulate proofs.");
    eprintln!("Enter public git address you're planing to use for your CrevID.");
    eprintln!("E.g.: https://github.com/<myusername>/crev-proofs");
    eprintln!("Changing it later will require manual config file editing.");
    let mut url;
    loop {
        url = rprompt::prompt_reply_stdout("Git URL: ")?;
        eprintln!("");
        eprintln!("You've entered: {}", url);
        if crev_common::yes_or_no_was_y("Is this correct? (y/n) ")? {
            break;
        }
    }

    let id = crev_data::id::OwnId::generate(url);
    eprintln!("Your CrevID will be protected by a passphrase.");
    eprintln!("There's no way to recover your CrevID if you forget your passphrase.");
    let passphrase = crev_common::read_new_passphrase()?;
    let locked = id::LockedId::from_own_id(&id, &passphrase)?;

    let local = Local::auto_create()?;
    local.save_locked_id(&locked)?;
    local.save_current_id(&id)?;

    eprintln!("Your CrevID was created and will be printed below in an encrypted form.");
    eprintln!("Make sure to back it up on another device, to prevent loosing it.");

    println!("{}", locked);
    Ok(())
}
#[cfg(test)]
mod tests;
