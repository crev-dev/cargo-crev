//! Crev - Web of Trust implementation
//!
//! # Introduction
//!
//! It's important to mention that Crev does not mandate
//! any particular implementation of the Web of Trust. It only
//! loosely defines data-format to describe trust relationships
//! between users.
//!
//! How exactly is the trustworthiness in the wider network
//! calculated remains an open question, and subject for experimentation.
//!
//! `crev-wot` is just an initial, reference implementation, and might
//! evolve, be replaced or become just one of many available implementations.
use chrono::{self, offset::Utc, DateTime};
use crev_data::{
    self,
    proof::{self, review, trust::TrustLevel, CommonOps, Content},
    Digest, Id, Level, Url, Version,
};
use default::default;
use log::debug;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    sync,
};

mod trust_set;

pub use trust_set::*;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Unknown proof type '{}'", _0)]
    UnknownProofType(Box<str>),

    #[error("{}", _0)]
    Data(#[from] crev_data::Error),
}

type Result<T, E = Error> = std::result::Result<T, E>;

/// Where a proof has been fetched from
#[derive(Debug, Clone)]
pub enum FetchSource {
    /// Remote repository (other people's proof repos)
    Url(sync::Arc<Url>),
    /// One of user's own proof repos, which are assumed to contain only verified information
    LocalUser,
}

/// A `T` with a timestamp
///
/// This allows easily keeping track of a most recent version
/// of `T`. Typically `T` is some information from a timestamped
/// *proof* of some kind.
#[derive(Clone, Debug)]
pub struct Timestamped<T> {
    pub date: chrono::DateTime<Utc>,
    value: T,
}

impl<T> Timestamped<T> {
    // Return `true` if value was updated
    fn update_to_more_recent(&mut self, other: &Self)
    where
        T: Clone,
    {
        // in practice it doesn't matter, but in tests
        // it's convenient to overwrite even if the time
        // is exactly the same
        if self.date <= other.date {
            self.date = other.date;
            self.value = other.value.clone();
        }
    }
}

impl<T, Tz> From<(&DateTime<Tz>, T)> for Timestamped<T>
where
    Tz: chrono::TimeZone,
{
    fn from(from: (&DateTime<Tz>, T)) -> Self {
        Timestamped {
            date: from.0.with_timezone(&Utc),
            value: from.1,
        }
    }
}

pub type Signature = String;
type TimestampedUrl = Timestamped<Url>;
type TimestampedTrustLevel = Timestamped<TrustLevel>;
type TimestampedReview = Timestamped<review::Review>;
type TimestampedSignature = Timestamped<Signature>;
type TimestampedFlags = Timestamped<proof::Flags>;

impl From<proof::Trust> for TimestampedTrustLevel {
    fn from(trust: proof::Trust) -> Self {
        TimestampedTrustLevel {
            date: trust.date_utc(),
            value: trust.trust,
        }
    }
}

impl<'a, T: proof::WithReview + Content + CommonOps> From<&'a T> for TimestampedReview {
    fn from(review: &T) -> Self {
        TimestampedReview {
            value: review.review().to_owned(),
            date: review.date_utc(),
        }
    }
}

/// Unique package review id
///
/// Since package review can be overwritten, it's useful
/// to refer to a review by an unique combination of:
///
/// * author's ID
/// * pkg source
/// * pkg name
/// * pkg version
#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct PkgVersionReviewId {
    from: Id,
    package_version_id: proof::PackageVersionId,
}

impl From<review::Package> for PkgVersionReviewId {
    fn from(review: review::Package) -> Self {
        PkgVersionReviewId {
            from: review.from().id.clone(),
            package_version_id: review.package.id,
        }
    }
}

impl From<&review::Package> for PkgVersionReviewId {
    fn from(review: &review::Package) -> Self {
        PkgVersionReviewId {
            from: review.from().id.to_owned(),
            package_version_id: review.package.id.clone(),
        }
    }
}

/// An unique id for a review by a given author of a given package.
///
/// Similar to `PackageVersionReviewId`, but where
/// exact version is not important.
#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct PkgReviewId {
    from: Id,
    package_id: proof::PackageId,
}

impl From<review::Package> for PkgReviewId {
    fn from(review: review::Package) -> Self {
        PkgReviewId {
            from: review.from().id.clone(),
            package_id: review.package.id.id,
        }
    }
}

impl From<&review::Package> for PkgReviewId {
    fn from(review: &review::Package) -> Self {
        PkgReviewId {
            from: review.from().id.to_owned(),
            package_id: review.package.id.id.clone(),
        }
    }
}

pub type Source = String;
pub type Name = String;

/// Alternatives relationship
///
/// Derived from the data in the proofs
#[derive(Default)]
struct AlternativesData {
    derived_recalculation_counter: usize,
    for_pkg: HashMap<proof::PackageId, HashMap<Id, HashSet<proof::PackageId>>>,
    reported_by: HashMap<(proof::PackageId, proof::PackageId), HashMap<Id, Signature>>,
}

impl AlternativesData {
    fn new() -> Self {
        Default::default()
    }

    fn wipe(&mut self) {
        *self = Self::new();
    }

    fn record_from_proof(&mut self, review: &review::Package, signature: &Signature) {
        for alternative in &review.alternatives {
            let a = &review.package.id.id;
            let b = alternative;
            let id = &review.from().id;
            self.for_pkg
                .entry(a.clone())
                .or_default()
                .entry(id.clone())
                .or_default()
                .insert(b.clone());

            self.for_pkg
                .entry(b.clone())
                .or_default()
                .entry(id.clone())
                .or_default()
                .insert(a.clone());

            self.reported_by
                .entry((a.clone(), b.clone()))
                .or_default()
                .insert(id.clone(), signature.clone());

            self.reported_by
                .entry((b.clone(), a.clone()))
                .or_default()
                .insert(id.clone(), signature.clone());
        }
    }
}

pub type TimestampedTrustDetails = Timestamped<TrustDetails>;
#[derive(Debug, Clone)]
pub struct TrustDetails {
    level: TrustLevel,
    override_: HashSet<Id>,
}

/// In memory database tracking information from proofs
///
/// After population, used for calculating the effective trust set, etc.
///
/// Right now, for every invocation of crev, we just load it up with
/// all known proofs, and then query. If it ever becomes too slow,
/// all the logic here will have to be moved to a real embedded db
/// of some kind.
pub struct ProofDB {
    /// who -(trusts)-> whom
    trust_id_to_id: HashMap<Id, HashMap<Id, TimestampedTrustDetails>>,
    /// who -(is being trusted by -> whom
    reverse_trust_id_to_id: HashMap<Id, HashMap<Id, TimestampedTrustLevel>>,

    /// (source, target) -> -(trust via)-> trust proof certificate
    ids_to_trust_proof_signatures: HashMap<(Id, Id), TimestampedSignature>,

    /// Id->URL mapping verified by Id's signature
    /// boolean is whether it's been fetched from the same URL, or local trusted repo,
    /// so that URL->Id is also true.
    url_by_id_self_reported: HashMap<Id, (TimestampedUrl, bool)>,

    /// Id->URL relationship reported by someone else that this Id
    url_by_id_reported_by_others: HashMap<Id, TimestampedUrl>,

    // all reviews are here
    package_review_by_signature: HashMap<Signature, review::Package>,

    // all trust proofs here
    trust_proofs_by_signature: HashMap<Signature, proof::Trust>,

    // we can get the to the review through the signature from these two
    package_review_signatures_by_package_digest:
        HashMap<Vec<u8>, HashMap<PkgVersionReviewId, TimestampedSignature>>,
    package_review_signatures_by_pkg_review_id: HashMap<PkgVersionReviewId, TimestampedSignature>,

    // pkg_review_id by package information, nicely grouped
    package_reviews:
        BTreeMap<Source, BTreeMap<Name, BTreeMap<Version, HashSet<PkgVersionReviewId>>>>,

    package_flags: HashMap<proof::PackageId, HashMap<Id, TimestampedFlags>>,

    // given an Id of an author, get the list of all package version id that were produced by it
    from_id_to_package_reviews: HashMap<Id, HashSet<proof::PackageVersionId>>,

    // original data about pkg alternatives
    // for every package_id, we store a map of ids that had alternatives for it,
    // and a timestamped signature of the proof, so we keep track of only
    // the newest alternatives list for a `(PackageId, reporting Id)` pair
    package_alternatives: HashMap<proof::PackageId, HashMap<Id, TimestampedSignature>>,

    // derived data about pkg alternatives
    // it is hard to keep track of some data when proofs are being added
    // which can override previously stored information; because of that
    // we don't keep track of it, until needed, and only then we just lazily
    // recalculate it
    insertion_counter: usize,
    derived_alternatives: sync::RwLock<AlternativesData>,
}

impl Default for ProofDB {
    fn default() -> Self {
        ProofDB {
            trust_id_to_id: default(),
            reverse_trust_id_to_id: default(),
            ids_to_trust_proof_signatures: default(),
            trust_proofs_by_signature: default(),
            url_by_id_self_reported: default(),
            url_by_id_reported_by_others: default(),
            package_review_signatures_by_package_digest: default(),
            package_review_signatures_by_pkg_review_id: default(),
            package_review_by_signature: default(),
            package_reviews: default(),
            package_alternatives: default(),
            package_flags: default(),
            from_id_to_package_reviews: default(),

            insertion_counter: 0,
            derived_alternatives: sync::RwLock::new(AlternativesData::new()),
        }
    }
}

#[derive(Default, Debug)]
pub struct IssueDetails {
    pub severity: Level,
    /// Reviews that reported a given issue by `issues` field
    pub issues: HashSet<PkgVersionReviewId>,
    /// Reviews that reported a given issue by `advisories` field
    pub advisories: HashSet<PkgVersionReviewId>,
}

impl ProofDB {
    pub fn new() -> Self {
        default()
    }

    fn get_derived_alternatives<'s>(&'s self) -> sync::RwLockReadGuard<'s, AlternativesData> {
        {
            let read = self.derived_alternatives.read().expect("lock to work");

            if read.derived_recalculation_counter == self.insertion_counter {
                return read;
            }
        }

        {
            let mut write = self.derived_alternatives.write().expect("lock to work");

            write.wipe();

            for alt in self.package_alternatives.values() {
                for signature in alt.values() {
                    write.record_from_proof(
                        &self.package_review_by_signature[&signature.value],
                        &signature.value,
                    );
                }
            }

            write.derived_recalculation_counter = self.insertion_counter;
        }

        self.derived_alternatives.read().expect("lock to work")
    }

    pub fn get_pkg_alternatives_by_author<'s, 'a>(
        &'s self,
        from: &'a Id,
        pkg_id: &'a proof::PackageId,
    ) -> HashSet<proof::PackageId> {
        let from = from.to_owned();

        let alternatives = self.get_derived_alternatives();
        alternatives
            .for_pkg
            .get(pkg_id)
            .into_iter()
            .flat_map(move |i| i.get(&from))
            .flatten()
            .cloned()
            .collect()
    }

    pub fn get_pkg_alternatives<'s, 'a>(
        &'s self,
        pkg_id: &'a proof::PackageId,
    ) -> HashSet<(Id, proof::PackageId)> {
        let alternatives = self.get_derived_alternatives();

        alternatives
            .for_pkg
            .get(pkg_id)
            .into_iter()
            .flat_map(move |i| i.iter())
            .flat_map(move |(id, pkg_ids)| {
                pkg_ids.iter().map(move |v| (id.to_owned(), v.to_owned()))
            })
            .collect()
    }

    pub fn get_pkg_flags_by_author<'s, 'a>(
        &'s self,
        from: &'a Id,
        pkg_id: &'a proof::PackageId,
    ) -> Option<&'s proof::Flags> {
        let from = from.to_owned();
        self.package_flags
            .get(pkg_id)
            .and_then(move |i| i.get(&from))
            .map(move |timestampted| &timestampted.value)
    }

    pub fn get_pkg_flags<'s, 'a>(
        &'s self,
        pkg_id: &'a proof::PackageId,
    ) -> impl Iterator<Item = (&Id, &'s proof::Flags)> {
        self.package_flags
            .get(pkg_id)
            .into_iter()
            .flat_map(move |i| i.iter())
            .map(|(id, flags)| (id, &flags.value))
    }

    pub fn get_pkg_reviews_for_source<'a, 'b>(
        &'a self,
        source: &'b str,
    ) -> impl Iterator<Item = &'a proof::review::Package> {
        self.package_reviews
            .get(source)
            .into_iter()
            .flat_map(move |map| map.iter())
            .flat_map(move |(_, map)| map.iter())
            .flat_map(|(_, v)| v)
            .map(move |pkg_review_id| {
                self.get_pkg_review_by_pkg_review_id(pkg_review_id)
                    .expect("exists")
            })
    }

    pub fn get_pkg_reviews_for_name<'a, 'b, 'c: 'a>(
        &'a self,
        source: &'b str,
        name: &'c str,
    ) -> impl Iterator<Item = &'a proof::review::Package> {
        self.package_reviews
            .get(source)
            .into_iter()
            .flat_map(move |map| map.get(name))
            .flat_map(move |map| map.iter())
            .flat_map(|(_, v)| v)
            .map(move |pkg_review_id| {
                self.get_pkg_review_by_pkg_review_id(pkg_review_id)
                    .expect("exists")
            })
    }

    pub fn get_pkg_reviews_for_version<'a, 'b, 'c: 'a, 'd: 'a>(
        &'a self,
        source: &'b str,
        name: &'c str,
        version: &'d Version,
    ) -> impl Iterator<Item = &'a proof::review::Package> {
        self.package_reviews
            .get(source)
            .into_iter()
            .flat_map(move |map| map.get(name))
            .flat_map(move |map| map.get(version))
            .flatten()
            .map(move |pkg_review_id| {
                self.get_pkg_review_by_pkg_review_id(pkg_review_id)
                    .expect("exists")
            })
    }

    pub fn get_pkg_reviews_gte_version<'a, 'b, 'c: 'a, 'd: 'a>(
        &'a self,
        source: &'b str,
        name: &'c str,
        version: &'d Version,
    ) -> impl Iterator<Item = &'a proof::review::Package> {
        self.package_reviews
            .get(source)
            .into_iter()
            .flat_map(move |map| map.get(name))
            .flat_map(move |map| map.range(version..))
            .flat_map(move |(_, v)| v)
            .map(move |pkg_review_id| {
                self.get_pkg_review_by_pkg_review_id(pkg_review_id)
                    .expect("exists")
            })
    }

    pub fn get_pkg_reviews_lte_version<'a, 'b, 'c: 'a, 'd: 'a>(
        &'a self,
        source: &'b str,
        name: &'c str,
        version: &'d Version,
    ) -> impl Iterator<Item = &'a proof::review::Package> {
        self.package_reviews
            .get(source)
            .into_iter()
            .flat_map(move |map| map.get(name))
            .flat_map(move |map| map.range(..=version))
            .flat_map(|(_, v)| v)
            .map(move |pkg_review_id| {
                self.get_pkg_review_by_pkg_review_id(pkg_review_id)
                    .expect("exists")
            })
    }

    pub fn get_pkg_review_by_pkg_review_id(
        &self,
        uniq: &PkgVersionReviewId,
    ) -> Option<&proof::review::Package> {
        let signature = &self
            .package_review_signatures_by_pkg_review_id
            .get(uniq)?
            .value;
        self.package_review_by_signature.get(signature)
    }

    pub fn get_pkg_review<'a, 'b, 'c: 'a, 'd: 'a>(
        &'a self,
        source: &'b str,
        name: &'c str,
        version: &'d Version,
        id: &Id,
    ) -> Option<&proof::review::Package> {
        self.get_pkg_reviews_for_version(source, name, version)
            .find(|pkg_review| pkg_review.from().id == *id)
    }

    pub fn get_advisories<'a, 'b, 'c: 'a, 'd: 'a>(
        &'a self,
        source: &'b str,
        name: Option<&'c str>,
        version: Option<&'d Version>,
    ) -> impl Iterator<Item = &'a proof::review::Package> + 'a {
        match (name, version) {
            (Some(name), Some(version)) => {
                Box::new(self.get_advisories_for_version(source, name, version))
                    as Box<dyn Iterator<Item = _>>
            }

            (Some(name), None) => Box::new(self.get_advisories_for_package(source, name)),
            (None, None) => Box::new(self.get_advisories_for_source(source)),
            (None, Some(_)) => panic!("Wrong usage"),
        }
    }

    pub fn get_pkg_reviews_with_issues_for<'a, 'b, 'c: 'a, 'd: 'a>(
        &'a self,
        source: &'b str,
        name: Option<&'c str>,
        version: Option<&'c Version>,
        trust_set: &'d TrustSet,
        trust_level_required: TrustLevel,
    ) -> impl Iterator<Item = &proof::review::Package> {
        match (name, version) {
            (Some(name), Some(version)) => Box::new(self.get_pkg_reviews_with_issues_for_version(
                source,
                name,
                version,
                trust_set,
                trust_level_required,
            )) as Box<dyn Iterator<Item = _>>,
            (Some(name), None) => Box::new(self.get_pkg_reviews_with_issues_for_name(
                source,
                name,
                trust_set,
                trust_level_required,
            )),
            (None, None) => Box::new(self.get_pkg_reviews_with_issues_for_source(
                source,
                trust_set,
                trust_level_required,
            )),
            (None, Some(_)) => panic!("Wrong usage"),
        }
    }

    pub fn get_advisories_for_version<'a, 'b, 'c: 'a, 'd: 'a>(
        &'a self,
        source: &'b str,
        name: &'c str,
        version: &'d Version,
    ) -> impl Iterator<Item = &proof::review::Package> {
        self.get_pkg_reviews_gte_version(source, name, version)
            .filter(move |review| review.is_advisory_for(version))
    }

    pub fn get_advisories_for_package<'a, 'b, 'c: 'a>(
        &'a self,
        source: &'b str,
        name: &'c str,
    ) -> impl Iterator<Item = &proof::review::Package> {
        self.package_reviews
            .get(source)
            .into_iter()
            .flat_map(move |map| map.get(name))
            .flat_map(move |map| map.iter())
            .flat_map(|(_, v)| v)
            .flat_map(move |pkg_review_id| {
                let review = &self.package_review_by_signature
                    [&self.package_review_signatures_by_pkg_review_id[pkg_review_id].value];

                if !review.advisories.is_empty() {
                    Some(review)
                } else {
                    None
                }
            })
    }

    pub fn get_advisories_for_source(
        &self,
        source: &str,
    ) -> impl Iterator<Item = &proof::review::Package> {
        self.get_pkg_reviews_for_source(source)
            .filter(|review| !review.advisories.is_empty())
    }

    /// Get all issues affecting a given package version
    ///
    /// Collect a map of Issue ID -> `IssueReports`, listing
    /// all issues known to affect a given package version.
    ///
    /// These are calculated from `advisories` and `issues` fields
    /// of the package reviews of reviewers intside a given `trust_set`
    /// of at least given `trust_level_required`.
    pub fn get_open_issues_for_version(
        &self,
        source: &str,
        name: &str,
        queried_version: &Version,
        trust_set: &TrustSet,
        trust_level_required: TrustLevel,
    ) -> HashMap<String, IssueDetails> {
        // This is one of the most complicated calculations in whole crev. I hate this code
        // already, and I have barely put it together.

        // Here we track all the reported isue by issue id
        let mut issue_reports_by_id: HashMap<String, IssueDetails> = HashMap::new();

        // First we go through all the reports in previous versions with `issues` fields and collect these.
        // Easy.
        for (review, issue) in self
            .get_pkg_reviews_lte_version(source, name, queried_version)
            .filter(|review| {
                let effective = trust_set.get_effective_trust_level(&review.from().id);
                effective >= trust_level_required
            })
            .flat_map(move |review| review.issues.iter().map(move |issue| (review, issue)))
            .filter(|(review, issue)| {
                issue.is_for_version_when_reported_in_version(
                    queried_version,
                    &review.package.id.version,
                )
            })
        {
            issue_reports_by_id
                .entry(issue.id.clone())
                .or_default()
                .issues
                .insert(PkgVersionReviewId::from(review));
        }

        // Now the complicated part. We go through all the advisories for all the versions
        // of given package.
        //
        // Advisories itself have two functions: first, they might have report an issue
        // by advertising that a given version should be upgraded to a newer version.
        //
        // Second - they might cancel `issues` inside `issue_reports_by_id` because they
        // advertise a fix that happened somewhere between the `issue` report and
        // the current `queried_version`.
        for (review, advisory) in self
            .get_pkg_reviews_for_name(source, name)
            .filter(|review| {
                let effective = trust_set.get_effective_trust_level(&review.from().id);
                effective >= trust_level_required
            })
            .flat_map(move |review| {
                review
                    .advisories
                    .iter()
                    .map(move |advisory| (review, advisory))
            })
        {
            // Add new issue reports created by the advisory
            if advisory.is_for_version_when_reported_in_version(
                queried_version,
                &review.package.id.version,
            ) {
                for id in &advisory.ids {
                    issue_reports_by_id
                        .entry(id.clone())
                        .or_default()
                        .issues
                        .insert(PkgVersionReviewId::from(review));
                }
            }

            // Remove the reports that are already fixed
            for id in &advisory.ids {
                if let Some(mut issue_marker) = issue_reports_by_id.get_mut(id) {
                    let issues = std::mem::take(&mut issue_marker.issues);
                    issue_marker.issues = issues
                        .into_iter()
                        .filter(|pkg_review_id| {
                            let signature = &self
                                .package_review_signatures_by_pkg_review_id
                                .get(pkg_review_id)
                                .expect("review for this signature")
                                .value;
                            let issue_review = self
                                .package_review_by_signature
                                .get(signature)
                                .expect("review for this pkg_review_id");
                            !advisory.is_for_version_when_reported_in_version(
                                &issue_review.package.id.version,
                                &review.package.id.version,
                            )
                        })
                        .collect();
                }
            }
        }

        issue_reports_by_id
            .into_iter()
            .filter(|(_id, markers)| !markers.issues.is_empty() || !markers.advisories.is_empty())
            .collect()
    }

    pub fn get_pkg_reviews_with_issues_for_version<'a, 'b, 'c: 'a>(
        &'a self,
        source: &'b str,
        name: &'c str,
        queried_version: &'c Version,
        trust_set: &'c TrustSet,
        trust_level_required: TrustLevel,
    ) -> impl Iterator<Item = &proof::review::Package> {
        self.get_pkg_reviews_with_issues_for_name(source, name, trust_set, trust_level_required)
            .filter(move |review| {
                !review.issues.is_empty()
                    || review.advisories.iter().any(|advi| {
                        advi.is_for_version_when_reported_in_version(
                            queried_version,
                            &review.package.id.version,
                        )
                    })
            })
    }

    pub fn get_pkg_reviews_with_issues_for_name<'a, 'b, 'c: 'a>(
        &'a self,
        source: &'b str,
        name: &'c str,
        trust_set: &'c TrustSet,
        trust_level_required: TrustLevel,
    ) -> impl Iterator<Item = &proof::review::Package> {
        self.get_pkg_reviews_for_name(source, name)
            .filter(move |review| {
                let effective = trust_set.get_effective_trust_level(&review.from().id);
                effective >= trust_level_required
            })
            .filter(|review| !review.issues.is_empty() || !review.advisories.is_empty())
    }

    pub fn get_pkg_reviews_with_issues_for_source<'a, 'b, 'c: 'a>(
        &'a self,
        source: &'b str,
        trust_set: &'c TrustSet,
        trust_level_required: TrustLevel,
    ) -> impl Iterator<Item = &proof::review::Package> {
        self.get_pkg_reviews_for_source(source)
            .filter(move |review| {
                let effective = trust_set.get_effective_trust_level(&review.from().id);
                effective >= trust_level_required
            })
            .filter(|review| !review.issues.is_empty() || !review.advisories.is_empty())
    }

    pub fn unique_package_review_proof_count(&self) -> usize {
        self.package_review_signatures_by_pkg_review_id.len()
    }

    pub fn unique_trust_proof_count(&self) -> usize {
        self.trust_id_to_id
            .iter()
            .fold(0, |count, (_id, set)| count + set.len())
    }

    fn add_code_review(&mut self, review: &review::Code, fetched_from: FetchSource) {
        let from = &review.from();
        self.record_url_from_from_field(&review.date_utc(), from, &fetched_from);
        for _file in &review.files {
            // not implemented right now; just ignore
        }
    }

    fn add_package_review(
        &mut self,
        review: &review::Package,
        signature: &str,
        fetched_from: FetchSource,
    ) {
        self.insertion_counter += 1;

        let from = &review.from();
        self.record_url_from_from_field(&review.date_utc(), from, &fetched_from);

        self.package_review_by_signature
            .entry(signature.to_owned())
            .or_insert_with(|| review.to_owned());

        let pkg_review_id = PkgVersionReviewId::from(review);
        let timestamp_signature = TimestampedSignature::from((review.date(), signature.to_owned()));
        let timestamp_flags = TimestampedFlags::from((review.date(), review.flags.clone()));

        self.package_review_signatures_by_package_digest
            .entry(review.package.digest.to_owned())
            .or_default()
            .entry(pkg_review_id.clone())
            .and_modify(|s| s.update_to_more_recent(&timestamp_signature))
            .or_insert_with(|| timestamp_signature.clone());

        self.package_review_signatures_by_pkg_review_id
            .entry(pkg_review_id.clone())
            .and_modify(|s| s.update_to_more_recent(&timestamp_signature))
            .or_insert_with(|| timestamp_signature.clone());

        self.from_id_to_package_reviews
            .entry(review.common.from.id.clone())
            .or_default()
            .insert(pkg_review_id.package_version_id.clone());

        self.package_reviews
            .entry(review.package.id.id.source.clone())
            .or_default()
            .entry(review.package.id.id.name.clone())
            .or_default()
            .entry(review.package.id.version.clone())
            .or_default()
            .insert(pkg_review_id);

        self.package_alternatives
            .entry(review.package.id.id.clone())
            .or_default()
            .entry(review.from().id.clone())
            .and_modify(|a| a.update_to_more_recent(&timestamp_signature))
            .or_insert_with(|| timestamp_signature);

        self.package_flags
            .entry(review.package.id.id.clone())
            .or_default()
            .entry(review.from().id.clone())
            .and_modify(|f| f.update_to_more_recent(&timestamp_flags))
            .or_insert_with(|| timestamp_flags);
    }

    pub fn get_package_review_count(
        &self,
        source: &str,
        name: Option<&str>,
        version: Option<&Version>,
    ) -> usize {
        self.get_package_reviews_for_package(source, name, version)
            .count()
    }

    pub fn get_package_reviews_for_package<'a, 'b, 'c: 'a, 'd: 'a>(
        &'a self,
        source: &'b str,
        name: Option<&'c str>,
        version: Option<&'d Version>,
    ) -> impl Iterator<Item = &'a proof::review::Package> + 'a {
        match (name, version) {
            (Some(name), Some(version)) => {
                Box::new(self.get_pkg_reviews_for_version(source, name, version))
                    as Box<dyn Iterator<Item = _>>
            }
            (Some(name), None) => Box::new(self.get_pkg_reviews_for_name(source, name)),
            (None, None) => Box::new(self.get_pkg_reviews_for_source(source)),
            (None, Some(_)) => panic!("Wrong usage"),
        }
    }

    pub fn get_package_reviews_for_package_sorted<'a, 'b, 'c: 'a, 'd: 'a>(
        &'a self,
        source: &'b str,
        name: Option<&'c str>,
        version: Option<&'d Version>,
    ) -> Vec<proof::review::Package> {
        let mut proofs: Vec<_> = self
            .get_package_reviews_for_package(source, name, version)
            .cloned()
            .collect();

        proofs.sort_by_key(|a| a.date_utc());

        proofs
    }

    fn add_trust_raw(
        &mut self,
        from: &Id,
        to: &Id,
        date: DateTime<Utc>,
        trust_proof: &proof::Trust,
        signature: &str,
    ) {
        let trust = TrustDetails {
            level: trust_proof.trust,
            override_: trust_proof
                .override_
                .iter()
                .map(|o| o.id.id.clone())
                .collect(),
        };

        let tl = TimestampedTrustLevel {
            value: trust.level,
            date,
        };
        let td = TimestampedTrustDetails { value: trust, date };

        self.trust_proofs_by_signature
            .insert(signature.to_owned(), trust_proof.to_owned());

        let signature = TimestampedSignature {
            value: signature.to_owned(),
            date,
        };

        self.ids_to_trust_proof_signatures
            .entry((from.to_owned(), to.to_owned()))
            .and_modify(|e| e.update_to_more_recent(&signature))
            .or_insert_with(|| signature);

        self.trust_id_to_id
            .entry(from.to_owned())
            .or_insert_with(HashMap::new)
            .entry(to.to_owned())
            .and_modify(|e| e.update_to_more_recent(&td))
            .or_insert_with(|| td);

        self.reverse_trust_id_to_id
            .entry(to.to_owned())
            .or_insert_with(HashMap::new)
            .entry(from.to_owned())
            .and_modify(|e| e.update_to_more_recent(&tl))
            .or_insert_with(|| tl);
    }

    fn add_trust(&mut self, trust: &proof::Trust, signature: &str, fetched_from: FetchSource) {
        let from = &trust.from();
        self.record_url_from_from_field(&trust.date_utc(), from, &fetched_from);
        for to in &trust.ids {
            self.add_trust_raw(&from.id, &to.id, trust.date_utc(), trust, signature);
        }
        for to in &trust.ids {
            // Others should not be making verified claims about this URL,
            // regardless of where these proofs were fetched from, because only
            // owner of the Id is authoritative.
            self.record_url_from_to_field(&trust.date_utc(), to)
        }
    }

    pub fn all_known_ids(&self) -> BTreeSet<Id> {
        self.url_by_id_self_reported
            .keys()
            .chain(self.url_by_id_reported_by_others.keys())
            .cloned()
            .collect()
    }

    pub fn get_reverse_trust_for<'id, 's: 'id>(
        &'s self,
        id: &'id Id,
    ) -> impl Iterator<Item = (&'id Id, TrustLevel)> + 's {
        self.reverse_trust_id_to_id
            .get(id)
            .into_iter()
            .flat_map(|map| {
                map.into_iter()
                    .map(|(id, trust_level)| (id, trust_level.value))
            })
    }

    /// Get all Ids that authored a proof (with total count)
    pub fn all_author_ids(&self) -> BTreeMap<Id, usize> {
        let mut res = BTreeMap::new();
        for (id, set) in &self.trust_id_to_id {
            *res.entry(id.to_owned()).or_default() += set.len();
        }

        for uniq_rev in self.package_review_signatures_by_pkg_review_id.keys() {
            *res.entry(uniq_rev.from.clone()).or_default() += 1;
        }

        res
    }

    pub fn get_package_review_by_signature<'a>(
        &'a self,
        signature: &str,
    ) -> Option<&'a review::Package> {
        self.package_review_by_signature.get(signature)
    }

    pub fn get_package_reviews_by_digest<'a>(
        &'a self,
        digest: &Digest,
    ) -> impl Iterator<Item = review::Package> + 'a {
        self.package_review_signatures_by_package_digest
            .get(digest.as_slice())
            .into_iter()
            .flat_map(move |unique_reviews| {
                unique_reviews
                    .iter()
                    .map(move |(_unique_review, signature)| {
                        self.package_review_by_signature[&signature.value].clone()
                    })
            })
    }

    /// Record an untrusted mapping between a PublicId and a URL it declares
    fn record_url_from_to_field(&mut self, date: &DateTime<Utc>, to: &crev_data::PublicId) {
        if let Some(url) = &to.url {
            self.url_by_id_reported_by_others
                .entry(to.id.clone())
                .or_insert_with(|| TimestampedUrl {
                    value: url.clone(),
                    date: *date,
                });
        }
    }

    pub fn record_tusted_url_from_own_id(&mut self, own_id: &crev_data::PublicId) {
        self.record_url_from_from_field(&Utc::now(), own_id, &FetchSource::LocalUser);
    }

    /// Record mapping between a PublicId and a URL it declares, and trust it's correct only if it's been fetched from the same URL
    fn record_url_from_from_field(
        &mut self,
        date: &DateTime<Utc>,
        from: &crev_data::PublicId,
        fetched_from: &FetchSource,
    ) {
        if let Some(url) = &from.url {
            let tu = TimestampedUrl {
                value: url.clone(),
                date: date.to_owned(),
            };
            let fetch_matches = match fetched_from {
                FetchSource::LocalUser => true,
                FetchSource::Url(fetched_url) if **fetched_url == *url => true,
                _ => false,
            };
            self.url_by_id_self_reported
                .entry(from.id.clone())
                .and_modify(|e| {
                    e.0.update_to_more_recent(&tu);
                    if fetch_matches {
                        e.1 = true;
                    }
                })
                .or_insert_with(|| (tu, fetch_matches));
        }
    }

    fn add_proof(&mut self, proof: &proof::Proof, fetched_from: FetchSource) -> Result<()> {
        proof
            .verify()
            .expect("All proofs were supposed to be valid here");
        match proof.kind() {
            proof::CodeReview::KIND => self.add_code_review(&proof.parse_content()?, fetched_from),
            proof::PackageReview::KIND => {
                self.add_package_review(&proof.parse_content()?, proof.signature(), fetched_from)
            }
            proof::Trust::KIND => {
                self.add_trust(&proof.parse_content()?, proof.signature(), fetched_from)
            }
            other => return Err(Error::UnknownProofType(other.into())),
        }

        Ok(())
    }

    pub fn import_from_iter(&mut self, i: impl Iterator<Item = (proof::Proof, FetchSource)>) {
        for (proof, fetch_source) in i {
            // ignore errors
            if let Err(e) = self.add_proof(&proof, fetch_source) {
                debug!("Ignoring proof: {}", e);
            }
        }
    }

    fn get_trust_details_list_of_id(&self, id: &Id) -> impl Iterator<Item = (&TrustDetails, &Id)> {
        self.trust_id_to_id
            .get(id)
            .map(|map| map.iter().map(|(id, trust)| (&trust.value, id)))
            .into_iter()
            .flatten()
    }

    pub fn get_trust_proof_between(&self, from: &Id, to: &Id) -> Option<&proof::Trust> {
        self.ids_to_trust_proof_signatures
            .get(&(from.to_owned(), to.to_owned()))
            .and_then(|sig| self.trust_proofs_by_signature.get(&sig.value))
    }

    fn get_package_reviews_by_author<'iter, 's: 'iter, 'id: 'iter>(
        &'s self,
        id: &'id Id,
    ) -> impl Iterator<Item = &'s review::Package> + 'iter {
        self.from_id_to_package_reviews
            .get(id)
            .into_iter()
            .flat_map(move |set| {
                set.iter()
                    .map(move |package_version_id| PkgVersionReviewId {
                        from: id.clone(),
                        package_version_id: package_version_id.clone(),
                    })
            })
            .map(move |pkg_version_review_id| {
                &self.package_review_by_signature
                    [&self.package_review_signatures_by_pkg_review_id[&pkg_version_review_id].value]
            })
    }

    pub fn calculate_trust_set(&self, for_id: &Id, params: &TrustDistanceParams) -> TrustSet {
        TrustSet::from(self, for_id, params)
    }

    /// Finds which URL is the latest and claimed to belong to the given Id.
    /// The result indicates how reliable information this is.
    pub fn lookup_url(&self, id: &Id) -> UrlOfId<'_> {
        self.url_by_id_self_reported
            .get(id)
            .map(|(url, fetch_matches)| {
                if *fetch_matches {
                    UrlOfId::FromSelfVerified(&url.value)
                } else {
                    UrlOfId::FromSelf(&url.value)
                }
            })
            .or_else(|| {
                self.url_by_id_reported_by_others
                    .get(id)
                    .map(|url| UrlOfId::FromOthers(&url.value))
            })
            .unwrap_or(UrlOfId::None)
    }
}

/// Result of URL lookup
#[derive(Debug, Copy, Clone)]
pub enum UrlOfId<'a> {
    /// Verified both ways: Id->URL via signature,
    /// and URL->Id by fetching, or trusting local user
    FromSelfVerified(&'a Url),
    /// Self-reported (signed by this Id)
    FromSelf(&'a Url),
    /// Reported by someone else (unverified)
    FromOthers(&'a Url),
    /// Unknown
    None,
}

impl<'a> UrlOfId<'a> {
    /// Only if this URL has been signed by its Id and verified by fetching
    pub fn verified(self) -> Option<&'a Url> {
        match self {
            Self::FromSelfVerified(url) => Some(url),
            _ => None,
        }
    }

    /// Only if this URL has been signed by its Id
    pub fn from_self(self) -> Option<&'a Url> {
        match self {
            Self::FromSelfVerified(url) | Self::FromSelf(url) => Some(url),
            _ => None,
        }
    }

    /// Any URL available, even if reported by someone else
    pub fn any_unverified(self) -> Option<&'a Url> {
        match self {
            Self::FromSelfVerified(url) | Self::FromSelf(url) | Self::FromOthers(url) => Some(url),
            _ => None,
        }
    }
}

pub struct TrustDistanceParams {
    pub max_distance: u64,
    pub high_trust_distance: u64,
    pub medium_trust_distance: u64,
    pub low_trust_distance: u64,
}

impl TrustDistanceParams {
    pub fn new_no_wot() -> Self {
        Self {
            max_distance: 0,
            high_trust_distance: 1,
            medium_trust_distance: 1,
            low_trust_distance: 1,
        }
    }

    fn distance_by_level(&self, level: TrustLevel) -> Option<u64> {
        use crev_data::proof::trust::TrustLevel::*;
        Some(match level {
            Distrust => return Option::None,
            None => return Option::None,
            Low => self.low_trust_distance,
            Medium => self.medium_trust_distance,
            High => self.high_trust_distance,
        })
    }
}

impl Default for TrustDistanceParams {
    fn default() -> Self {
        Self {
            max_distance: 10,
            high_trust_distance: 0,
            medium_trust_distance: 1,
            low_trust_distance: 5,
        }
    }
}

/// List of authors recommending override (ignore) trust / package review with their effective
/// trust level.
#[derive(Debug, Clone, Default)]
pub struct OverrideSourcesDetails(HashMap<Id, TrustLevel>);

impl OverrideSourcesDetails {
    pub fn insert(&mut self, id: Id, level: TrustLevel) {
        self.0
            .entry(id)
            .and_modify(|prev_level| *prev_level = level.max(*prev_level))
            .or_insert(level);
    }

    pub fn max_level(&self) -> Option<TrustLevel> {
        self.0.iter().map(|e| e.1).max().copied()
    }
}

#[test]
fn db_is_send_sync() {
    fn is<T: Send + Sync>() {}
    is::<ProofDB>();
}

#[cfg(test)]
mod tests;
