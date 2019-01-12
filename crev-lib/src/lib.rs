#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate failure;

pub mod id;
pub mod local;
pub(crate) mod prelude;
pub mod proof;
pub mod proofdb;
pub mod repo;
pub mod staging;
pub mod util;

use crate::{prelude::*, proofdb::TrustSet};
use crev_data::Digest;
use std::{
    collections::HashSet,
    fmt,
    path::{Path, PathBuf},
};

pub use self::local::Local;
pub use crate::proofdb::{ProofDB, TrustDistanceParams};

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

impl fmt::Display for TrustOrDistrust {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TrustOrDistrust::Trust => f.write_str("trust"),
            TrustOrDistrust::Distrust => f.write_str("distrust"),
        }
    }
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
#[derive(PartialEq, Eq, Debug)]
pub enum VerificationStatus {
    Verified(crev_data::proof::TrustLevel),
    None,
    Flagged,
    Dangerous,
}

impl VerificationStatus {
    pub fn is_verified(&self) -> bool {
        match self {
            VerificationStatus::Verified(_) => true,
            _ => false,
        }
    }
}

impl fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerificationStatus::Verified(level) => f.pad(&level.to_string()),
            VerificationStatus::None => f.pad("none"),
            VerificationStatus::Flagged => f.pad("flagged"),
            VerificationStatus::Dangerous => f.pad("danger"),
        }
    }
}

pub fn dir_or_git_repo_verify<H1>(
    path: &Path,
    ignore_list: &HashSet<PathBuf, H1>,
    db: &ProofDB,
    trusted_set: &TrustSet,
) -> Result<crate::VerificationStatus>
where
    H1: std::hash::BuildHasher + std::default::Default,
{
    let digest = if path.join(".git").exists() {
        get_recursive_digest_for_git_dir(path, ignore_list)?
    } else {
        Digest::from_vec(crev_recursive_digest::get_recursive_digest_for_dir::<
            crev_common::Blake2b256,
            H1,
        >(path, ignore_list)?)
    };

    Ok(db.verify_package_digest(&digest, trusted_set))
}

pub fn dir_verify<H1>(
    path: &Path,
    ignore_list: &HashSet<PathBuf, H1>,
    db: &ProofDB,
    trusted_set: &TrustSet,
) -> Result<crate::VerificationStatus>
where
    H1: std::hash::BuildHasher + std::default::Default,
{
    let digest = Digest::from_vec(crev_recursive_digest::get_recursive_digest_for_dir::<
        crev_common::Blake2b256,
        H1,
    >(path, ignore_list)?);
    Ok(db.verify_package_digest(&digest, trusted_set))
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

#[cfg(test)]
mod tests;
