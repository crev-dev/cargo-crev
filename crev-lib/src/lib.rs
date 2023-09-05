#![type_length_limit = "10709970"]
#![allow(clippy::implicit_hasher)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::redundant_closure_for_method_calls)]

pub mod activity;
pub mod id;
pub mod local;
pub mod proof;
pub mod repo;
pub mod staging;
pub mod util;
pub use crate::local::Local;
use log::warn;
pub use activity::{ReviewActivity, ReviewMode};
use crev_data::{
    self,
    id::IdError,
    proof::{
        review::{self, Rating},
        trust::TrustLevel,
        CommonOps,
    },
    Digest, Id, RegistrySource, Version,
};
use crev_wot::PkgVersionReviewId;
pub use crev_wot::TrustDistanceParams;
use std::{
    collections::{HashMap, HashSet},
    fmt,
    path::{Path, PathBuf},
};
use std::error::Error as _;

/// Failures that can happen in this library
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Trying to init a directory that is already there
    #[error("`{}` already exists", _0.display())]
    PathAlreadyExists(Box<Path>),

    /// There are manual modifications in the git repo. Commit or reset them?
    #[error("Git repository is not in a clean state")]
    GitRepositoryIsNotInACleanState,

    /// Found data from the future. Your version of crev is too old.
    #[error("Unsupported version {}", _0)]
    UnsupportedVersion(i64),

    /// Your crev-id changed unexpectedly
    #[error("PubKey mismatch")]
    PubKeyMismatch,

    /// You need to make a crev Id to perform most operations
    #[error("User config not-initialized. Use `crev id new` to generate CrevID.")]
    UserConfigNotInitialized,

    /// Use `auto_create_or_open` or fix potentially messed up config directory
    #[error("User config already exists")]
    UserConfigAlreadyExists,

    /// User config loading error
    #[error("User config loading error '{}': {}", _0.0.display(), _0.1)]
    UserConfigLoadError(Box<(PathBuf, std::io::Error)>),

    /// You've sandboxed too hard? We need to run Cargo
    #[error("No valid home directory path could be retrieved from the operating system")]
    NoHomeDirectory,

    /// This stores your private key
    #[error("Id loading error '{}': {}", _0.0.display(), _0.1)]
    IdLoadError(Box<(PathBuf, std::io::Error)>),

    /// Create a new Id
    #[error("Id file not found.")]
    IDFileNotFound,

    /// Crev repos must be public
    #[error("Couldn't clone {}: {}", _0.0, _0.1)]
    CouldNotCloneGitHttpsURL(Box<(String, String)>),

    /// We don't support anonymous reviews
    #[error("No ids given.")]
    NoIdsGiven,

    /// There's no password reset. If you don't remember it, start over!
    #[error("Incorrect passphrase")]
    IncorrectPassphrase,

    /// crev has a concept of a default/current Id
    #[error("Current Id not set")]
    CurrentIDNotSet,

    /// crev has a concept of a default/current Id
    #[error("Id not specified and current id not set")]
    IDNotSpecifiedAndCurrentIDNotSet,

    /// crev uses git checkouts, and needs to know their URLs. Delete the repo and try again.
    #[error("origin has no url at {}", _0.display())]
    OriginHasNoURL(Box<Path>),

    /// crev created a dummy Id for you, but you still need to configure it
    #[error("current Id has been created without a git URL")]
    GitUrlNotConfigured,

    /// Error iterating local db
    #[error("Error iterating local ProofStore at {}: {}", _0.0.display(), _0.1)]
    ErrorIteratingLocalProofStore(Box<(PathBuf, String)>),

    /// blake_hash mismatch
    #[error("File {} not current. Review again use `crev add` to update.", _0.display())]
    FileNotCurrent(Box<Path>),

    /// Needs config.yaml
    #[error("Package config not-initialized. Use `crev package init` to generate it.")]
    PackageConfigNotInitialized,

    /// Wrong path given to git
    #[error("Can't stage path from outside of the staging root")]
    PathNotInStageRootPath,

    /// Git is cursed
    #[error("Git entry without a path")]
    GitEntryWithoutAPath,

    /// Sorry about YAML syntax
    #[error(transparent)]
    YAML(#[from] serde_yaml::Error),

    /// Used for staging temp file
    #[error(transparent)]
    CBOR(#[from] serde_cbor::Error),

    /// See [`repo::PackageDirNotFound`]
    #[error(transparent)]
    PackageDirNotFound(#[from] repo::PackageDirNotFound),

    /// See [`crev_common::CancelledError`]
    #[error(transparent)]
    Cancelled(#[from] crev_common::CancelledError),

    /// See [`crev_data::Error`]
    #[error(transparent)]
    Data(#[from] crev_data::Error),

    /// See [`argon2::Error`]
    #[error("Passphrase: {}", _0)]
    Passphrase(#[from] argon2::Error),

    /// YAML ;(
    #[error("Review activity parse error: {}", _0)]
    ReviewActivity(#[source] Box<crev_common::YAMLIOError>),

    /// YAML ;(
    #[error("Error parsing user config: {}", _0)]
    UserConfigParse(#[source] serde_yaml::Error),

    /// See [`crev_recursive_digest::DigestError`]
    #[error(transparent)]
    Digest(#[from] crev_recursive_digest::DigestError),

    /// Misc problems with git repos
    #[error(transparent)]
    Git(#[from] git2::Error),

    /// Misc problems with file I/O
    #[error("I/O: {}", _0)]
    IO(#[from] std::io::Error),

    /// crev open makes cargo projects that don't run the code
    #[error("Error while copying crate sources: {}", _0)]
    CrateSourceSanitizationError(std::io::Error),

    /// Misc problems with file I/O
    #[error("Error writing to {}: {}", _1.display(), _0)]
    FileWrite(std::io::Error, PathBuf),

    /// See [`IdError`]
    #[error(transparent)]
    Id(#[from] IdError),
}

/// [`crate::Error`]
type Result<T, E = Error> = std::result::Result<T, E>;

/// Trait representing a place that can keep proofs (all reviews and trust proofs)
///
/// See [`::crev_wot::ProofDb`] and [`crate::Local`].
///
/// Typically serialized and persisted.
#[doc(hidden)]
pub trait ProofStore {
    fn insert(&self, proof: &crev_data::proof::Proof) -> Result<()>;
    fn proofs_iter(&self) -> Result<Box<dyn Iterator<Item = crev_data::proof::Proof>>>;
}

/// Your relationship to the person
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TrustProofType {
    /// Positive
    Trust,
    /// Neutral (undo Trust)
    Untrust,
    /// Very negative. This is an attacker. Block everything by them.
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
    /// Is this person trusted at all? (regardless of level of trust)
    #[must_use]
    pub fn is_trust(self) -> bool {
        if let TrustProofType::Trust = self {
            return true;
        }
        false
    }

    /// Make review template. See [`crev_data::Review`]
    #[must_use]
    pub fn to_review(self) -> crev_data::Review {
        use TrustProofType::{Distrust, Trust, Untrust};
        match self {
            Trust => crev_data::Review::new_positive(),
            Distrust => crev_data::Review::new_negative(),
            Untrust => crev_data::Review::new_none(),
        }
    }
}

/// Verification requirements for filtering out low quality reviews
///
/// See [`crev_wot::TrustDistanceParams`]
#[derive(Clone, Debug)]
pub struct VerificationRequirements {
    /// How much the reviewer must be trusted
    pub trust_level: crev_data::Level,
    /// How much code understanding reviewer has reported
    pub understanding: crev_data::Level,
    /// How much thoroughness reviewer has reported
    pub thoroughness: crev_data::Level,
    /// How many different reviews are required
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
    /// That's bad!
    Negative,
    /// VerificationRequirements set too high
    Insufficient,
    /// Okay
    Verified,
    /// This is your package, trust yourself.
    Local,
}

impl VerificationStatus {
    /// Is it VerificationStatus::Verified?
    #[must_use]
    pub fn is_verified(self) -> bool {
        self == VerificationStatus::Verified
    }

    /// Pick worse of both
    #[must_use]
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

/// Find reviews matching `Digest` (exact data of the crate)
/// and see if there are enough positive reviews for it.
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

/// Warnings gathered during operation, errors downgraded to warnings.
#[derive(Debug, thiserror::Error)]
pub enum Warning {
    #[error(transparent)]
    Error(#[from] Error),

    #[error("Repo checkout without origin URL at {}", _0.display())]
    NoRepoUrlAtPath(PathBuf, #[source] Error),

    #[error("URL for {0} is not known yet")]
    IdUrlNotKnonw(Id),

    #[error("Could not deduce `ssh` push url for {0}. Call:\ncargo crev repo git remote set-url --push origin <url>\nmanually after the id is generated.")]
    GitPushUrl(String),

    #[error("Failed to fetch {0} into {}", _2.display())]
    FetchError(String, #[source] Error, PathBuf),
}

impl Warning {
    pub fn auto_log() -> LogOnDrop {
        LogOnDrop(Vec::new())
    }

    pub fn log_all(warnings: &[Warning]) {
        warnings.iter().for_each(|w| w.log());
    }

    pub fn log(&self) {
        warn!("{}", self);
        let mut s = self.source();
        while let Some(w) = s {
            warn!("  - {}", w);
            s = w.source();
        }
    }
}

pub struct LogOnDrop(pub Vec<Warning>);
impl Drop for LogOnDrop {
    fn drop(&mut self) {
        Warning::log_all(&self.0);
    }
}

impl std::ops::Deref for LogOnDrop {
    type Target = Vec<Warning>;
    fn deref(&self) -> &Vec<Warning> { &self.0 }
}
impl std::ops::DerefMut for LogOnDrop {
    fn deref_mut(&mut self) -> &mut Vec<Warning> { &mut self.0 }
}

/// Scan through known reviews of the crate (source is "https://crates.io")
/// and report semver you can safely use according to `requirements`
///
/// See also `verify_package_digest`
pub fn find_latest_trusted_version(
    trust_set: &crev_wot::TrustSet,
    source: RegistrySource<'_>,
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

/// Check whether code at this path has reviews, and the reviews meet the requirements.
///
/// See also `verify_package_digest`
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

/// Scan dir and hash everything in it, to get a unique identifier of the package's source code
pub fn get_dir_digest(path: &Path, ignore_list: &fnv::FnvHashSet<PathBuf>) -> Result<Digest> {
    Ok(Digest::from_vec(util::get_recursive_digest_for_dir(path, ignore_list)?).unwrap())
}

/// See get_dir_digest
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

/// See get_dir_digest
pub fn get_recursive_digest_for_paths(
    root_path: &Path,
    paths: fnv::FnvHashSet<PathBuf>,
) -> Result<crev_data::Digest> {
    Ok(util::get_recursive_digest_for_paths(root_path, paths)?)
}

/// See get_dir_digest
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
