#![type_length_limit = "10709970"]
#![allow(clippy::manual_range_contains)]

pub mod activity;
pub mod id;
pub mod local;
pub mod proof;
pub mod repo;
pub mod staging;
pub mod util;
pub use crate::local::Local;
pub use activity::{ReviewActivity, ReviewMode};
use crev_data::{
    self,
    id::IdError,
    proof::{
        review::{self, Rating},
        trust::TrustLevel,
        CommonOps,
    },
    Digest, Id, Version,
};
use crev_wot::PkgVersionReviewId;
pub use crev_wot::TrustDistanceParams;
use std::{
    collections::{HashMap, HashSet},
    fmt,
    path::{Path, PathBuf},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("`{}` already exists", _0.display())]
    PathAlreadyExists(Box<Path>),

    #[error("Git repository is not in a clean state")]
    GitRepositoryIsNotInACleanState,

    #[error("Unsupported version {}", _0)]
    UnsupportedVersion(i64),

    #[error("PubKey mismatch")]
    PubKeyMismatch,

    #[error("User config not-initialized. Use `crev id new` to generate CrevID.")]
    UserConfigNotInitialized,

    #[error("User config already exists")]
    UserConfigAlreadyExists,

    #[error("User config loading error '{}': {}", _0.0.display(), _0.1)]
    UserConfigLoadError(Box<(PathBuf, std::io::Error)>),

    #[error("No valid home directory path could be retrieved from the operating system")]
    NoHomeDirectory,

    #[error("Id loading error '{}': {}", _0.0.display(), _0.1)]
    IdLoadError(Box<(PathBuf, std::io::Error)>),

    #[error("Id file not found.")]
    IDFileNotFound,

    #[error("Couldn't clone {}: {}", _0.0, _0.1)]
    CouldNotCloneGitHttpsURL(Box<(String, String)>),

    #[error("No ids given.")]
    NoIdsGiven,

    #[error("Incorrect passphrase")]
    IncorrectPassphrase,

    #[error("Current Id not set")]
    CurrentIDNotSet,

    #[error("Id not specified and current id not set")]
    IDNotSpecifiedAndCurrentIDNotSet,

    #[error("origin has no url")]
    OriginHasNoURL,

    #[error("current Id has been created without a git URL")]
    GitUrlNotConfigured,

    #[error("Error iterating local ProofStore at {}: {}", _0.0.display(), _0.1)]
    ErrorIteratingLocalProofStore(Box<(PathBuf, String)>),

    #[error("File {} not current. Review again use `crev add` to update.", _0.display())]
    FileNotCurrent(Box<Path>),

    #[error("Package config not-initialized. Use `crev package init` to generate it.")]
    PackageConfigNotInitialized,

    #[error("Can't stage path from outside of the staging root")]
    PathNotInStageRootPath,

    #[error("Git entry without a path")]
    GitEntryWithoutAPath,

    #[error(transparent)]
    YAML(#[from] serde_yaml::Error),

    #[error(transparent)]
    CBOR(#[from] serde_cbor::Error),

    #[error(transparent)]
    PackageDirNotFound(#[from] repo::PackageDirNotFound),

    #[error(transparent)]
    Cancelled(#[from] crev_common::CancelledError),

    #[error(transparent)]
    Data(#[from] crev_data::Error),

    #[error("Passphrase: {}", _0)]
    Passphrase(#[from] argon2::Error),

    #[error("Review activity parse error: {}", _0)]
    ReviewActivity(#[source] Box<crev_common::YAMLIOError>),

    #[error("Error parsing user config: {}", _0)]
    UserConfigParse(#[source] serde_yaml::Error),

    #[error(transparent)]
    Digest(#[from] crev_recursive_digest::DigestError),

    #[error(transparent)]
    Git(#[from] git2::Error),

    #[error("I/O: {}", _0)]
    IO(#[from] std::io::Error),

    #[error("Error while copying crate sources: {}", _0)]
    CrateSourceSanitizationError(std::io::Error),

    #[error("Error writing to {}: {}", _1.display(), _0)]
    FileWrite(std::io::Error, PathBuf),

    #[error(transparent)]
    Id(#[from] IdError),
}

type Result<T, E = Error> = std::result::Result<T, E>;

/// Trait representing a place that can keep proofs (all reviews and trust proofs)
///
/// See `ProofDb`.
///
/// Typically serialized and persisted.
pub trait ProofStore {
    fn insert(&self, proof: &crev_data::proof::Proof) -> Result<()>;
    fn proofs_iter(&self) -> Result<Box<dyn Iterator<Item = crev_data::proof::Proof>>>;
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TrustProofType {
    Trust,
    Untrust,
    Distrust,
}

impl fmt::Display for TrustProofType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrustProofType::Trust => f.write_str("trust"),
            TrustProofType::Distrust => f.write_str("distrust"),
            TrustProofType::Untrust => f.write_str("untrust"),
        }
    }
}

impl TrustProofType {
    pub fn is_trust(self) -> bool {
        if let TrustProofType::Trust = self {
            return true;
        }
        false
    }

    pub fn to_review(self) -> crev_data::Review {
        use TrustProofType::*;
        match self {
            Trust => crev_data::Review::new_positive(),
            Distrust => crev_data::Review::new_negative(),
            Untrust => crev_data::Review::new_none(),
        }
    }
}

/// Verification requirements
#[derive(Clone, Debug)]
pub struct VerificationRequirements {
    pub trust_level: crev_data::Level,
    pub understanding: crev_data::Level,
    pub thoroughness: crev_data::Level,
    pub redundancy: u64,
}

impl Default for VerificationRequirements {
    fn default() -> Self {
        VerificationRequirements {
            trust_level: Default::default(),
            understanding: Default::default(),
            thoroughness: Default::default(),
            redundancy: 1,
        }
    }
}
/// Result of verification
///
/// Not named `Result` to avoid confusion with `Result` type.
#[derive(Copy, Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum VerificationStatus {
    Negative,
    Insufficient,
    Verified,
    Local,
}

impl VerificationStatus {
    pub fn is_verified(self) -> bool {
        self == VerificationStatus::Verified
    }

    pub fn min(self, other: Self) -> Self {
        if self < other {
            self
        } else if other < self {
            other
        } else {
            self
        }
    }
}

impl fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerificationStatus::Local => f.pad("locl"),
            VerificationStatus::Verified => f.pad("pass"),
            VerificationStatus::Insufficient => f.pad("none"),
            VerificationStatus::Negative => f.pad("warn"),
        }
    }
}

pub fn verify_package_digest(
    digest: &Digest,
    trust_set: &crev_wot::TrustSet,
    requirements: &VerificationRequirements,
    db: &crev_wot::ProofDB,
) -> VerificationStatus {
    let reviews: HashMap<Id, review::Package> = db
        .get_package_reviews_by_digest(digest)
        .filter(|review| {
            match trust_set
                .package_review_ignore_override
                .get(&PkgVersionReviewId::from(review))
            {
                Some(reporters) => {
                    reporters.max_level().unwrap_or(TrustLevel::None)
                        <= trust_set.get_effective_trust_level(&review.common.from.id)
                }
                None => true,
            }
        })
        .map(|review| (review.from().id.clone(), review))
        .collect();
    // Faster somehow maybe?
    let reviews_by: HashSet<Id, _> = reviews.keys().cloned().collect();
    let trusted_ids: HashSet<_> = trust_set.get_trusted_ids();
    let matching_reviewers = trusted_ids.intersection(&reviews_by);
    let mut trust_count = 0;
    let mut negative_count = 0;
    for matching_reviewer in matching_reviewers {
        let review = &reviews[matching_reviewer].review_possibly_none();
        if !review.is_none()
            && Rating::Neutral <= review.rating
            && requirements.thoroughness <= review.thoroughness
            && requirements.understanding <= review.understanding
        {
            if TrustLevel::from(requirements.trust_level)
                <= trust_set.get_effective_trust_level(matching_reviewer)
            {
                trust_count += 1;
            }
        } else if review.rating <= Rating::Negative {
            negative_count += 1;
        }
    }

    if negative_count > 0 {
        VerificationStatus::Negative
    } else if trust_count >= requirements.redundancy {
        VerificationStatus::Verified
    } else {
        VerificationStatus::Insufficient
    }
}

pub fn find_latest_trusted_version(
    trust_set: &crev_wot::TrustSet,
    source: &str,
    name: &str,
    requirements: &crate::VerificationRequirements,
    db: &crev_wot::ProofDB,
) -> Option<Version> {
    db.get_pkg_reviews_for_name(source, name)
        .filter(|review| {
            verify_package_digest(
                &Digest::from_vec(review.package.digest.clone()).unwrap(),
                trust_set,
                requirements,
                db,
            )
            .is_verified()
        })
        .max_by(|a, b| a.package.id.version.cmp(&b.package.id.version))
        .map(|review| review.package.id.version.clone())
}

/// Check whether code at this path has reviews, and the reviews meet the requirements
pub fn dir_or_git_repo_verify(
    path: &Path,
    ignore_list: &fnv::FnvHashSet<PathBuf>,
    db: &crev_wot::ProofDB,
    trusted_set: &crev_wot::TrustSet,
    requirements: &VerificationRequirements,
) -> Result<crate::VerificationStatus> {
    let digest = if path.join(".git").exists() {
        get_recursive_digest_for_git_dir(path, ignore_list)?
    } else {
        Digest::from_vec(util::get_recursive_digest_for_dir(path, ignore_list)?).unwrap()
    };

    Ok(verify_package_digest(
        &digest,
        trusted_set,
        requirements,
        db,
    ))
}

/// Check whether code at this path has reviews, and the reviews meet the requirements
///
/// Same as `dir_or_git_repo_verify`, except it doesn't handle .git dirs
pub fn dir_verify(
    path: &Path,
    ignore_list: &fnv::FnvHashSet<PathBuf>,
    db: &crev_wot::ProofDB,
    trusted_set: &crev_wot::TrustSet,
    requirements: &VerificationRequirements,
) -> Result<crate::VerificationStatus> {
    let digest = Digest::from_vec(util::get_recursive_digest_for_dir(path, ignore_list)?).unwrap();
    Ok(verify_package_digest(
        &digest,
        trusted_set,
        requirements,
        db,
    ))
}

pub fn get_dir_digest(path: &Path, ignore_list: &fnv::FnvHashSet<PathBuf>) -> Result<Digest> {
    Ok(Digest::from_vec(util::get_recursive_digest_for_dir(path, ignore_list)?).unwrap())
}

pub fn get_recursive_digest_for_git_dir(
    root_path: &Path,
    ignore_list: &fnv::FnvHashSet<PathBuf>,
) -> Result<Digest> {
    let git_repo = git2::Repository::open(root_path)?;

    let mut status_opts = git2::StatusOptions::new();
    let mut paths = HashSet::default();

    status_opts.include_unmodified(true);
    status_opts.include_untracked(false);
    for entry in git_repo.statuses(Some(&mut status_opts))?.iter() {
        let entry_path = PathBuf::from(entry.path().ok_or(Error::GitEntryWithoutAPath)?);
        if ignore_list.contains(&entry_path) {
            continue;
        };

        paths.insert(entry_path);
    }

    Ok(util::get_recursive_digest_for_paths(root_path, paths)?)
}

pub fn get_recursive_digest_for_paths(
    root_path: &Path,
    paths: fnv::FnvHashSet<PathBuf>,
) -> Result<crev_data::Digest> {
    Ok(util::get_recursive_digest_for_paths(root_path, paths)?)
}

pub fn get_recursive_digest_for_dir(
    root_path: &Path,
    rel_path_ignore_list: &fnv::FnvHashSet<PathBuf>,
) -> Result<Digest> {
    Ok(Digest::from_vec(util::get_recursive_digest_for_dir(
        root_path,
        rel_path_ignore_list,
    )?)
    .unwrap())
}

#[cfg(test)]
mod tests;
