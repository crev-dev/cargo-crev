use crate::{VerificationRequirements, VerificationStatus};
use chrono::{self, offset::Utc, DateTime};
use crev_data::{
    self,
    proof::{
        self,
        review::{self, Rating},
        trust::TrustLevel,
        Content, ContentCommon,
    },
    Digest, Id, Level, Url,
};
use default::default;
use semver::Version;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// A `T` with a timestamp
///
/// This allows easily keeping track of a most recent version
/// of `T`. Typically `T` is a *proof* of some kind.
#[derive(Clone, Debug)]
pub struct Timestamped<T> {
    pub date: chrono::DateTime<Utc>,
    value: T,
}

impl<T> Timestamped<T> {
    // Return `trude` if value was updated
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

pub type Signature = String;
type TimestampedUrl = Timestamped<Url>;
type TimestampedTrustLevel = Timestamped<TrustLevel>;
type TimestampedReview = Timestamped<review::Review>;

impl From<proof::Trust> for TimestampedTrustLevel {
    fn from(trust: proof::Trust) -> Self {
        TimestampedTrustLevel {
            date: trust.date().with_timezone(&Utc),
            value: trust.trust,
        }
    }
}

impl<'a, T: review::Common> From<&'a T> for TimestampedReview {
    fn from(review: &T) -> Self {
        TimestampedReview {
            value: review.review().to_owned(),
            date: review.date().with_timezone(&Utc),
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
pub struct PkgReviewId {
    from: Id,
    source: String,
    name: String,
    version: Version,
}

type TimestampedSignature = Timestamped<Signature>;

impl From<review::Package> for PkgReviewId {
    fn from(review: review::Package) -> Self {
        PkgReviewId {
            from: review.from.id,
            source: review.package.source,
            name: review.package.name,
            version: review.package.version,
        }
    }
}

impl From<&review::Package> for PkgReviewId {
    fn from(review: &review::Package) -> Self {
        PkgReviewId {
            from: review.from.id.to_owned(),
            source: review.package.source.to_owned(),
            name: review.package.name.to_owned(),
            version: review.package.version.to_owned(),
        }
    }
}

impl<Tz> From<(&DateTime<Tz>, String)> for TimestampedSignature
where
    Tz: chrono::TimeZone,
{
    fn from(args: (&DateTime<Tz>, String)) -> Self {
        Self {
            date: args.0.with_timezone(&Utc),
            value: args.1,
        }
    }
}

pub type Source = String;
pub type Name = String;

/// In memory database tracking information from proofs
///
/// After population, used for calculating the effcttive trust set, etc.
///
/// Right now, for every invocation of crev, we just load it up with
/// all known proofs, and then query. If it ever becomes too slow,
/// all the logic here will have to be moved to a real embedded db
/// of some kind.
pub struct ProofDB {
    /// who -(trusts)-> whom
    trust_id_to_id: HashMap<Id, HashMap<Id, TimestampedTrustLevel>>,

    url_by_id: HashMap<Id, TimestampedUrl>,
    url_by_id_secondary: HashMap<Id, TimestampedUrl>,

    // all reviews are here
    package_review_by_signature: HashMap<Signature, review::Package>,

    // we can get the to the review through the signature from these two
    package_review_signatures_by_package_digest:
        HashMap<Vec<u8>, HashMap<PkgReviewId, TimestampedSignature>>,
    package_review_signatures_by_pkg_review_id: HashMap<PkgReviewId, TimestampedSignature>,

    // pkg_review_id by package information, nicely grouped
    package_reviews: BTreeMap<Source, BTreeMap<Name, BTreeMap<Version, HashSet<PkgReviewId>>>>,
}

impl Default for ProofDB {
    fn default() -> Self {
        ProofDB {
            trust_id_to_id: default(),
            url_by_id: default(),
            url_by_id_secondary: default(),
            package_review_signatures_by_package_digest: default(),
            package_review_signatures_by_pkg_review_id: default(),
            package_review_by_signature: default(),
            package_reviews: default(),
        }
    }
}

#[derive(Default, Debug)]
pub struct IssueDetails {
    pub severity: Level,
    /// Reviews that reported a given issue by `issues` field
    pub issues: HashSet<PkgReviewId>,
    /// Reviews that reported a given issue by `advisories` field
    pub advisories: HashSet<PkgReviewId>,
}

impl ProofDB {
    pub fn new() -> Self {
        default()
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
            .flat_map(|v| v)
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
        uniq: &PkgReviewId,
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
            .find(|pkg_review| pkg_review.from.id == *id)
    }

    pub fn get_advisories<'a, 'b, 'c: 'a, 'd: 'a>(
        &'a self,
        source: &'b str,
        name: Option<&'c str>,
        version: Option<&'d Version>,
    ) -> impl Iterator<Item = &'a proof::review::Package> + 'a {
        match (name, version) {
            (Some(ref name), Some(ref version)) => {
                Box::new(self.get_advisories_for_version(source, name, version))
                    as Box<dyn Iterator<Item = _>>
            }

            (Some(ref name), None) => Box::new(self.get_advisories_for_package(source, name)),
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
            .filter(move |review| review.is_advisory_for(&version))
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
                let effective = trust_set.get_effective_trust_level(&review.from.id);
                effective >= trust_level_required
            })
            .flat_map(move |review| review.issues.iter().map(move |issue| (review, issue)))
            .filter(|(review, issue)| {
                issue.is_for_version_when_reported_in_version(
                    queried_version,
                    &review.package.version,
                )
            })
        {
            issue_reports_by_id
                .entry(issue.id.clone())
                .or_default()
                .issues
                .insert(PkgReviewId::from(review));
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
                let effective = trust_set.get_effective_trust_level(&review.from.id);
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
            if advisory
                .is_for_version_when_reported_in_version(&queried_version, &review.package.version)
            {
                for id in &advisory.ids {
                    issue_reports_by_id
                        .entry(id.clone())
                        .or_default()
                        .issues
                        .insert(PkgReviewId::from(review));
                }
            }

            // Remove the reports that are already fixed
            for id in &advisory.ids {
                if let Some(mut issue_marker) = issue_reports_by_id.get_mut(id) {
                    let issues = std::mem::replace(&mut issue_marker.issues, HashSet::new());
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
                                &issue_review.package.version,
                                &review.package.version,
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
                            &queried_version,
                            &review.package.version,
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
                let effective = trust_set.get_effective_trust_level(&review.from.id);
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
                let effective = trust_set.get_effective_trust_level(&review.from.id);
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

    fn add_code_review(&mut self, review: &review::Code) {
        let from = &review.from;
        self.record_url_from_from_field(&review.date_utc(), &from);
        for _file in &review.files {
            // not implemented right now; just ignore
        }
    }

    fn add_package_review(&mut self, review: &review::Package, signature: &str) {
        let from = &review.from;
        self.record_url_from_from_field(&review.date_utc(), &from);

        self.package_review_by_signature
            .entry(signature.to_owned())
            .or_insert_with(|| review.to_owned());

        let pkg_review_id = PkgReviewId::from(review);
        let timestamp_signature = TimestampedSignature::from((review.date(), signature.to_owned()));

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

        self.package_reviews
            .entry(review.package.source.clone())
            .or_default()
            .entry(review.package.name.clone())
            .or_default()
            .entry(review.package.version.clone())
            .or_default()
            .insert(pkg_review_id);
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
            (Some(ref name), Some(ref version)) => {
                Box::new(self.get_pkg_reviews_for_version(source, name, version))
                    as Box<dyn Iterator<Item = _>>
            }
            (Some(ref name), None) => Box::new(self.get_pkg_reviews_for_name(source, name)),
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

        proofs.sort_by(|a, b| a.date().cmp(&b.date()));

        proofs
    }

    fn add_trust_raw(&mut self, from: &Id, to: &Id, date: DateTime<Utc>, trust: TrustLevel) {
        let tl = TimestampedTrustLevel { value: trust, date };
        self.trust_id_to_id
            .entry(from.to_owned())
            .or_insert_with(HashMap::new)
            .entry(to.to_owned())
            .and_modify(|e| e.update_to_more_recent(&tl))
            .or_insert_with(|| tl);
    }

    fn add_trust(&mut self, trust: &proof::Trust) {
        let from = &trust.from;
        self.record_url_from_from_field(&trust.date_utc(), &from);
        for to in &trust.ids {
            self.add_trust_raw(&from.id, &to.id, trust.date_utc(), trust.trust);
        }
        for to in &trust.ids {
            self.record_url_from_to_field(&trust.date_utc(), &to)
        }
    }

    pub fn all_known_ids(&self) -> BTreeSet<Id> {
        self.url_by_id
            .keys()
            .chain(self.url_by_id_secondary.keys())
            .cloned()
            .collect()
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

    pub fn verify_package_digest(
        &self,
        digest: &Digest,
        trust_set: &TrustSet,
        requirements: &VerificationRequirements,
    ) -> VerificationStatus {
        let reviews: HashMap<Id, review::Package> = self
            .get_package_reviews_by_digest(digest)
            .map(|review| (review.from.id.clone(), review))
            .collect();
        // Faster somehow maybe?
        let reviews_by: HashSet<Id, _> = reviews.keys().cloned().collect();
        let trusted_ids: HashSet<_> = trust_set.trusted_ids().cloned().collect();
        let matching_reviewers = trusted_ids.intersection(&reviews_by);
        let mut trust_count = 0;
        let mut negative_count = 0;
        for matching_reviewer in matching_reviewers {
            let review = &reviews[matching_reviewer].review;
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
        &self,
        trust_set: &TrustSet,
        source: &str,
        name: &str,
        requirements: &crate::VerificationRequirements,
    ) -> Option<Version> {
        self.get_pkg_reviews_for_name(source, name)
            .filter(|review| {
                self.verify_package_digest(
                    &Digest::from_vec(review.package.digest.clone()),
                    trust_set,
                    requirements,
                )
                .is_verified()
            })
            .max_by(|a, b| a.package.version.cmp(&b.package.version))
            .map(|review| review.package.version.clone())
    }

    fn record_url_from_to_field(&mut self, date: &DateTime<Utc>, to: &crev_data::PubId) {
        self.url_by_id_secondary
            .entry(to.id.clone())
            .or_insert_with(|| TimestampedUrl {
                value: to.url.clone(),
                date: *date,
            });
    }

    fn record_url_from_from_field(&mut self, date: &DateTime<Utc>, from: &crev_data::PubId) {
        let tu = TimestampedUrl {
            value: from.url.clone(),
            date: date.to_owned(),
        };

        self.url_by_id
            .entry(from.id.clone())
            .and_modify(|e| e.update_to_more_recent(&tu))
            .or_insert_with(|| tu);
    }

    fn add_proof(&mut self, proof: &proof::Proof) {
        proof
            .verify()
            .expect("All proofs were supposed to be valid here");
        match proof.content {
            Content::Code(ref review) => self.add_code_review(&review),
            Content::Package(ref review) => self.add_package_review(&review, &proof.signature),
            Content::Trust(ref trust) => self.add_trust(&trust),
        }
    }

    pub fn import_from_iter(&mut self, i: impl Iterator<Item = proof::Proof>) {
        for proof in i {
            self.add_proof(&proof);
        }
    }

    fn get_trust_list_of_id(&self, id: &Id) -> impl Iterator<Item = (TrustLevel, &Id)> {
        if let Some(map) = self.trust_id_to_id.get(id) {
            Some(map.iter().map(|(id, trust)| (trust.value, id)))
        } else {
            None
        }
        .into_iter()
        .flatten()
    }

    pub fn calculate_trust_set(&self, for_id: &Id, params: &TrustDistanceParams) -> TrustSet {
        let mut distrusted = HashMap::new();

        // We keep retrying the whole thing, with more and more
        // distrusted Ids
        loop {
            let prev_distrusted_len = distrusted.len();
            let trust_set = self.calculate_trust_set_internal(for_id, params, distrusted);
            if trust_set.distrusted.len() <= prev_distrusted_len {
                return trust_set;
            }
            distrusted = trust_set.distrusted;
        }
    }

    /// Calculate the effective trust levels for IDs inside a WoT.
    ///
    /// This is one of the most important functions in `crev-lib`.
    fn calculate_trust_set_internal(
        &self,
        for_id: &Id,
        params: &TrustDistanceParams,
        distrusted: HashMap<Id, HashSet<Id>>,
    ) -> TrustSet {
        #[derive(PartialOrd, Ord, Eq, PartialEq, Clone, Debug)]
        struct Visit {
            distance: u64,
            id: Id,
        }

        let mut pending = BTreeSet::new();
        let mut visited = TrustSet::default();
        visited.distrusted = distrusted;

        pending.insert(Visit {
            distance: 0,
            id: for_id.clone(),
        });
        visited.record_trusted_id(for_id.clone(), for_id.clone(), 0, TrustLevel::High);

        while let Some(current) = pending.iter().next().cloned() {
            pending.remove(&current);

            for (level, candidate_id) in self.get_trust_list_of_id(&&current.id) {
                if level == TrustLevel::Distrust {
                    visited
                        .distrusted
                        .entry(candidate_id.clone())
                        .or_default()
                        .insert(current.id.clone());
                    continue;
                }

                let candidate_distance_from_current =
                    if let Some(v) = params.distance_by_level(level) {
                        v
                    } else {
                        continue;
                    };

                if visited.distrusted.contains_key(candidate_id) {
                    continue;
                }
                let candidate_total_distance = current.distance + candidate_distance_from_current;

                if candidate_total_distance > params.max_distance {
                    continue;
                }

                let candidate_effective_trust = std::cmp::min(
                    level,
                    visited
                        .get_effective_trust_level_opt(&current.id)
                        .expect("Id should have been inserted to `visited` beforehand"),
                );

                if candidate_effective_trust < TrustLevel::None {
                    unreachable!(
                        "this should not happen: candidate_effective_trust < TrustLevel::None"
                    );
                }

                if visited.record_trusted_id(
                    candidate_id.clone(),
                    current.id.clone(),
                    candidate_total_distance,
                    candidate_effective_trust,
                ) {
                    pending.insert(Visit {
                        distance: candidate_total_distance,
                        id: candidate_id.to_owned(),
                    });
                }
            }
        }

        visited
    }

    pub fn lookup_url(&self, id: &Id) -> Option<&Url> {
        self.url_by_id
            .get(id)
            .or_else(|| self.url_by_id_secondary.get(id))
            .map(|url| &url.value)
    }
}

/// Details of a one Id that is
#[derive(Debug, Clone)]
struct TrustedIdDetails {
    distance: u64,
    // effective, global trust from the root of the WoT
    effective_trust: TrustLevel,
    referers: HashMap<Id, TrustLevel>,
}

#[derive(Default, Debug, Clone)]
pub struct TrustSet {
    trusted: HashMap<Id, TrustedIdDetails>,
    distrusted: HashMap<Id, HashSet<Id>>,
}

impl TrustSet {
    pub fn trusted_ids(&self) -> impl Iterator<Item = &Id> {
        self.trusted.keys()
    }

    pub fn contains_trusted(&self, id: &Id) -> bool {
        self.trusted.contains_key(id)
    }

    pub fn contains_distrusted(&self, id: &Id) -> bool {
        self.distrusted.contains_key(id)
    }

    /// Record that an Id is considered trusted
    ///
    /// Returns `true` if this actually added or changed the `subject` details,
    /// which requires revising it's own downstream trusted Id details in the graph algorithm for it.
    fn record_trusted_id(
        &mut self,
        subject: Id,
        referer: Id,
        distance: u64,
        effective_trust: TrustLevel,
    ) -> bool {
        // TODO: turn into log or something
        // eprintln!(
        //     "{} -> {} {} ({})",
        //     referer, subject, distance, effective_trust
        // );
        use std::collections::hash_map::Entry;

        match self.trusted.entry(subject) {
            Entry::Vacant(entry) => {
                let mut referers = HashMap::default();
                referers.insert(referer, effective_trust);
                entry.insert(TrustedIdDetails {
                    distance,
                    effective_trust,
                    referers,
                });
                true
            }
            Entry::Occupied(mut entry) => {
                let mut changed = false;
                let details = entry.get_mut();
                if details.distance > distance {
                    details.distance = distance;
                    changed = true;
                }
                if details.effective_trust < effective_trust {
                    details.effective_trust = effective_trust;
                    changed = true;
                }
                match details.referers.entry(referer.clone()) {
                    Entry::Vacant(entry) => {
                        entry.insert(effective_trust);
                        changed = true;
                    }
                    Entry::Occupied(mut entry) => {
                        let level = entry.get_mut();
                        if *level < effective_trust {
                            *level = effective_trust;
                            changed = true;
                        }
                    }
                }
                changed
            }
        }
    }

    pub fn get_effective_trust_level(&self, id: &Id) -> TrustLevel {
        self.get_effective_trust_level_opt(id)
            .unwrap_or(TrustLevel::None)
    }

    pub fn get_effective_trust_level_opt(&self, id: &Id) -> Option<TrustLevel> {
        self.trusted.get(id).map(|details| details.effective_trust)
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
