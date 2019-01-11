use crate::VerificationStatus;
use chrono::{self, offset::Utc, DateTime};
use crev_data::{
    self,
    proof::{
        self,
        review::{self, Rating},
        trust::TrustLevel,
        Content, ContentCommon,
    },
    Digest, Id, Url,
};
use default::default;
use std::collections::{hash_map, BTreeMap, BTreeSet, HashMap, HashSet};

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
    fn update_to_more_recent(&mut self, date: &chrono::DateTime<Utc>, value: T) {
        if self.date < *date {
            self.value = value;
        }
    }

    fn insert_into_or_update_to_more_recent<K>(self, entry: hash_map::Entry<K, Timestamped<T>>) {
        match entry {
            hash_map::Entry::Occupied(mut entry) => entry
                .get_mut()
                .update_to_more_recent(&self.date, self.value),
            hash_map::Entry::Vacant(entry) => {
                entry.insert(self);
            }
        }
    }
}

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

/// Unique package review
///
/// Since package review can be overwritten, it's useful
/// to refer to a review by an unique combination of
///
/// * author's ID
/// * source
/// * crate
/// * version
#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct UniquePackageReview {
    from: Id,
    source: String,
    name: String,
    version: String,
}

type TimestampedSignature = Timestamped<String>;

impl From<review::Package> for UniquePackageReview {
    fn from(review: review::Package) -> Self {
        Self {
            from: review.from.id,
            source: review.package.source,
            name: review.package.name,
            version: review.package.version,
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
/// In memory database tracking information from proofs
///
/// After population, used for calculating the effcttive trust set, etc.
///
/// Right now, for every invocation of crev, we just load it up with
/// all known proofs, and then query. If it ever becomes too slow,
/// all the logic here will have to be moved to a real embedded db
/// of some kind.
pub struct ProofDB {
    trust_id_to_id: HashMap<Id, HashMap<Id, TimestampedTrustLevel>>, // who -(trusts)-> whom
    url_by_id: HashMap<Id, TimestampedUrl>,
    url_by_id_secondary: HashMap<Id, TimestampedUrl>,

    package_review_by_signature: HashMap<String, review::Package>,

    package_review_signatures_by_package_digest:
        HashMap<Vec<u8>, HashMap<UniquePackageReview, TimestampedSignature>>,
    package_review_signatures_by_unique_package_review:
        HashMap<UniquePackageReview, TimestampedSignature>,

    package_reviews_by_source: BTreeMap<String, HashSet<UniquePackageReview>>,
    package_reviews_by_name: BTreeMap<(String, String), HashSet<UniquePackageReview>>,
    package_reviews_by_version: BTreeMap<(String, String, String), HashSet<UniquePackageReview>>,
}

impl Default for ProofDB {
    fn default() -> Self {
        ProofDB {
            trust_id_to_id: default(),
            url_by_id: default(),
            url_by_id_secondary: default(),
            package_review_signatures_by_package_digest: default(),
            package_review_signatures_by_unique_package_review: default(),
            package_review_by_signature: default(),
            package_reviews_by_source: default(),
            package_reviews_by_name: default(),
            package_reviews_by_version: default(),
        }
    }
}

impl ProofDB {
    pub fn new() -> Self {
        default()
    }

    pub fn unique_package_review_proof_count(&self) -> usize {
        self.package_review_signatures_by_unique_package_review
            .len()
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

        let unique = UniquePackageReview::from(review.clone());
        let timestamp_signature = TimestampedSignature::from((review.date(), signature.to_owned()));

        timestamp_signature
            .clone()
            .insert_into_or_update_to_more_recent(
                self.package_review_signatures_by_package_digest
                    .entry(review.package.digest.to_owned())
                    .or_insert_with(|| default())
                    .entry(unique.clone()),
            );

        timestamp_signature.insert_into_or_update_to_more_recent(
            self.package_review_signatures_by_unique_package_review
                .entry(unique.clone()),
        );

        self.package_reviews_by_source
            .entry(review.package.source.to_owned())
            .or_default()
            .insert(unique.clone());
        self.package_reviews_by_name
            .entry((
                review.package.source.to_owned(),
                review.package.name.to_owned(),
            ))
            .or_default()
            .insert(unique.clone());
        self.package_reviews_by_version
            .entry((
                review.package.source.to_owned(),
                review.package.name.to_owned(),
                review.package.version.to_owned(),
            ))
            .or_default()
            .insert(unique);
    }

    pub fn get_package_review_count(
        &self,
        source: &str,
        name: Option<&str>,
        version: Option<&str>,
    ) -> usize {
        self.get_package_reviews_for_package(source, name, version)
            .count()
    }

    pub fn get_package_reviews_for_package(
        &self,
        source: &str,
        name: Option<&str>,
        version: Option<&str>,
    ) -> impl Iterator<Item = proof::review::Package> {
        let mut proofs: Vec<_> = match (name, version) {
            (Some(name), Some(version)) => self.package_reviews_by_version.get(&(
                source.to_owned(),
                name.to_owned(),
                version.to_owned(),
            )),

            (Some(name), None) => self
                .package_reviews_by_name
                .get(&(source.to_owned(), name.to_owned())),

            (None, None) => self.package_reviews_by_source.get(source),

            (None, Some(_)) => panic!("Wrong usage"),
        }
        .into_iter()
        .flat_map(|s| s)
        .map(|unique_package_review| {
            self.package_review_by_signature[&self
                .package_review_signatures_by_unique_package_review[unique_package_review]
                .value]
                .clone()
        })
        .collect();

        proofs.sort_by(|a, b| a.date().cmp(&b.date()));

        proofs.into_iter()
    }

    fn add_trust_raw(&mut self, from: &Id, to: &Id, date: DateTime<Utc>, trust: TrustLevel) {
        TimestampedTrustLevel { value: trust, date }.insert_into_or_update_to_more_recent(
            self.trust_id_to_id
                .entry(from.to_owned())
                .or_insert_with(HashMap::new)
                .entry(to.to_owned()),
        );
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

        for uniq_rev in self
            .package_review_signatures_by_unique_package_review
            .keys()
        {
            *res.entry(uniq_rev.from.clone()).or_default() += 1;
        }

        res
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
    ) -> VerificationStatus {
        let reviews: HashMap<Id, review::Package> = self
            .get_package_reviews_by_digest(digest)
            .map(|review| (review.from.id.clone(), review))
            .collect();
        // Faster somehow maybe?
        let reviews_by: HashSet<Id, _> = reviews.keys().map(|s| s.to_owned()).collect();
        let trusted_ids: HashSet<_> = trust_set.trusted_ids().cloned().collect();
        let matching_reviewers = trusted_ids.intersection(&reviews_by);
        let mut trust_count = 0;
        let mut trust_level = TrustLevel::None;
        let mut flagged_count = 0;
        let mut dangerous_count = 0;
        for matching_reviewer in matching_reviewers {
            let rating = &reviews[matching_reviewer].review.rating;
            if Rating::Neutral <= *rating {
                trust_count += 1;
                trust_level = std::cmp::max(
                    trust_level,
                    trust_set
                        .get_effective_trust_level(matching_reviewer)
                        .expect("Id should have been there"),
                );
            } else if *rating <= Rating::Dangerous {
                dangerous_count += 1;
            } else if *rating < Rating::Neutral {
                flagged_count += 1;
            }
        }

        if dangerous_count > 0 {
            VerificationStatus::Dangerous
        } else if flagged_count > 0 {
            VerificationStatus::Flagged
        } else if trust_count > 0 {
            VerificationStatus::Verified(trust_level)
        } else {
            VerificationStatus::Unknown
        }
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
        TimestampedUrl {
            value: from.url.clone(),
            date: date.to_owned(),
        }
        .insert_into_or_update_to_more_recent(self.url_by_id.entry(from.id.clone()));
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
                        .get_effective_trust_level(&current.id)
                        .expect("Id should have been inserted to `visited` beforehand"),
                );

                if candidate_effective_trust < TrustLevel::None {
                    // can this even happen?
                    debug_assert!(false);
                    continue;
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
struct TrustedIdDetails {
    distance: u64,
    // effective, global trust from the root of the WoT
    effective_trust: TrustLevel,
    referers: HashMap<Id, TrustLevel>,
}

#[derive(Default)]
pub struct TrustSet {
    trusted: HashMap<Id, TrustedIdDetails>,
    distrusted: HashMap<Id, HashSet<Id>>,
}

impl TrustSet {
    pub fn trusted_ids(&self) -> impl Iterator<Item = &Id> {
        self.trusted.keys()
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

    pub fn get_effective_trust_level(&self, id: &Id) -> Option<TrustLevel> {
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
