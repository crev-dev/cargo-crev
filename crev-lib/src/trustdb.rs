use crate::VerificationStatus;
use chrono::{self, offset::Utc, DateTime};
use crev_data::{
    self,
    level::Level,
    proof::review::Rating,
    proof::{self, review, Content, ContentCommon},
    Digest, Id,
};
use default::default;
use std::collections::BTreeMap;
use std::collections::{hash_map, BTreeSet, HashMap, HashSet};

struct TrustInfo {
    #[allow(unused)]
    trust: crev_data::level::Level,
    date: chrono::DateTime<Utc>,
}

impl<'a> From<&'a proof::Trust> for TrustInfo {
    fn from(trust: &proof::Trust) -> Self {
        TrustInfo {
            trust: trust.trust,
            date: trust.date().with_timezone(&Utc),
        }
    }
}

impl TrustInfo {
    fn maybe_update_with(&mut self, date: &chrono::DateTime<Utc>, trust: Level) {
        if *date > self.date {
            self.trust = trust;
        }
    }
}

struct ReviewInfo {
    #[allow(unused)]
    date: chrono::DateTime<Utc>,
    review: proof::review::Review,
}

impl<'a, T: review::Common> From<&'a T> for ReviewInfo {
    fn from(review: &T) -> Self {
        ReviewInfo {
            review: review.review().to_owned(),
            date: review.date().with_timezone(&Utc),
        }
    }
}

impl ReviewInfo {
    fn maybe_update_with(&mut self, review: &dyn review::Common) {
        if review.date().with_timezone(&Utc) > self.date {
            self.review = review.review().to_owned()
        }
    }
}

struct UrlInfo {
    #[allow(unused)]
    date: chrono::DateTime<Utc>,
    url: crev_data::Url,
}

impl UrlInfo {
    fn maybe_update_with(&mut self, date: &DateTime<Utc>, url: &crev_data::Url) {
        if date > &self.date {
            self.url = url.clone()
        }
    }
}

/// In memory database tracking known trust relationships
///
/// After population, used for calculating the effcttive trust set.
pub struct TrustDB {
    #[allow(unused)]
    trust_id_to_id: HashMap<Id, HashMap<Id, TrustInfo>>, // who -(trusts)-> whom
    //trust_id_to_review: HashMap<String, HashMap<Vec<u8>, ReviewInfo>>, // who -(reviewed)-> what (file digest)
    digest_to_reviews: HashMap<Vec<u8>, HashMap<Id, ReviewInfo>>, // what (digest) -(reviewed)-> by whom
    url_by_id: HashMap<Id, UrlInfo>,
    url_by_id_secondary: HashMap<Id, UrlInfo>,
    trusted_ids: HashSet<Id>,

    project_review_by_signature: HashMap<String, review::Project>,
    project_reviews_by_source: BTreeMap<String, BTreeSet<String>>,
    project_reviews_by_name: BTreeMap<(String, String), BTreeSet<String>>,
    project_reviews_by_version: BTreeMap<(String, String, String), BTreeSet<String>>,
}

impl TrustDB {
    pub fn new() -> Self {
        TrustDB {
            trust_id_to_id: Default::default(),
            //trust_id_to_review: Default::default(),
            url_by_id: Default::default(),
            url_by_id_secondary: Default::default(),
            digest_to_reviews: Default::default(),
            trusted_ids: Default::default(),
            project_review_by_signature: default(),
            project_reviews_by_source: default(),
            project_reviews_by_name: default(),
            project_reviews_by_version: default(),
        }
    }

    fn add_code_review(&mut self, review: &review::Code) {
        let from = &review.from;
        self.record_url_from_from_field(&review.date_utc(), &from);
        for file in &review.files {
            match self
                .digest_to_reviews
                .entry(file.digest.to_owned())
                .or_insert_with(|| HashMap::new())
                .entry(from.id.clone())
            {
                hash_map::Entry::Occupied(mut entry) => entry.get_mut().maybe_update_with(review),
                hash_map::Entry::Vacant(entry) => {
                    entry.insert(ReviewInfo::from(review));
                }
            }
        }
    }

    fn add_project_review(&mut self, review: &review::Project, signature: &str) {
        let from = &review.from;
        self.record_url_from_from_field(&review.date_utc(), &from);
        match self
            .digest_to_reviews
            .entry(review.project.digest.to_owned())
            .or_insert_with(|| HashMap::new())
            .entry(from.id.clone())
        {
            hash_map::Entry::Occupied(mut entry) => entry.get_mut().maybe_update_with(review),
            hash_map::Entry::Vacant(entry) => {
                entry.insert(ReviewInfo::from(review));
            }
        }

        self.project_review_by_signature
            .entry(signature.to_owned())
            .or_insert_with(|| review.to_owned());

        self.project_reviews_by_source
            .entry(review.project.source.to_owned())
            .or_default()
            .insert(signature.to_owned());
        self.project_reviews_by_name
            .entry((
                review.project.source.to_owned(),
                review.project.name.to_owned(),
            ))
            .or_default()
            .insert(signature.to_owned());
        self.project_reviews_by_version
            .entry((
                review.project.source.to_owned(),
                review.project.name.to_owned(),
                review.project.version.to_owned(),
            ))
            .or_default()
            .insert(signature.to_owned());
    }

    pub fn get_project_reviews_for_project(
        &self,
        source: &str,
        name: Option<&str>,
        version: Option<&str>,
    ) -> impl Iterator<Item = proof::review::Project> {
        let mut proofs: Vec<_> = match (name, version) {
            (Some(name), Some(version)) => self
                .project_reviews_by_version
                .get(&(source.to_owned(), name.to_owned(), version.to_owned()))
                .map(|set| {
                    set.iter()
                        .map(|signature| {
                            self.project_review_by_signature
                                .get(signature)
                                .unwrap()
                                .clone()
                        })
                        .collect()
                })
                .unwrap_or(vec![]),

            (Some(name), None) => self
                .project_reviews_by_name
                .get(&(source.to_owned(), name.to_owned()))
                .map(|set| {
                    set.iter()
                        .map(|signature| {
                            self.project_review_by_signature
                                .get(signature)
                                .unwrap()
                                .clone()
                        })
                        .collect()
                })
                .unwrap_or(vec![]),
            (None, None) => self
                .project_reviews_by_source
                .get(source)
                .map(|set| {
                    set.iter()
                        .map(|signature| {
                            self.project_review_by_signature
                                .get(signature)
                                .unwrap()
                                .clone()
                        })
                        .collect()
                })
                .unwrap_or(vec![]),
            (None, Some(_)) => panic!("Wrong usage"),
        };

        proofs.sort_by(|a, b| a.date().cmp(&b.date()));

        proofs.into_iter()
    }

    fn add_trust_raw(&mut self, from: &Id, to: &Id, date: DateTime<Utc>, trust: Level) {
        match self
            .trust_id_to_id
            .entry(from.to_owned())
            .or_insert_with(|| HashMap::new())
            .entry(to.to_owned())
        {
            hash_map::Entry::Occupied(mut entry) => entry.get_mut().maybe_update_with(&date, trust),
            hash_map::Entry::Vacant(entry) => {
                entry.insert(TrustInfo { trust, date });
            }
        }
    }

    fn add_trust(&mut self, trust: &proof::Trust) {
        let from = &trust.from;
        self.record_url_from_from_field(&trust.date_utc(), &from);
        for to in &trust.ids {
            self.add_trust_raw(&from.id, &to.id, trust.date_utc(), trust.trust);
        }
        if self.trusted_ids.contains(&from.id) {
            for to in &trust.ids {
                self.record_url_from_to_field(&trust.date_utc(), &to)
            }
        }
    }

    fn get_reviews_of(&self, digest: &Digest) -> Option<&HashMap<Id, ReviewInfo>> {
        self.digest_to_reviews.get(digest.as_slice())
    }

    pub fn verify_digest<H>(
        &self,
        digest: &Digest,
        trust_set: &HashSet<Id, H>,
    ) -> VerificationStatus
    where
        H: std::hash::BuildHasher + std::default::Default,
    {
        if let Some(reviews) = self.get_reviews_of(digest) {
            // Faster somehow maybe?
            let reviews_by: HashSet<Id, H> = reviews.keys().map(|s| s.to_owned()).collect();
            let matching_reviewers = trust_set.intersection(&reviews_by);
            let mut trust_count = 0;
            let mut distrust_count = 0;
            for matching_reviewer in matching_reviewers {
                if reviews[matching_reviewer].review.rating > Rating::Negative {
                    trust_count += 1;
                }
                if reviews[matching_reviewer].review.rating < Rating::Neutral {
                    distrust_count += 1;
                }
            }

            if distrust_count > 0 {
                VerificationStatus::Flagged
            } else if trust_count > 0 {
                VerificationStatus::Trusted
            } else {
                VerificationStatus::Untrusted
            }
        } else {
            VerificationStatus::Untrusted
        }
    }

    fn record_url_from_to_field(&mut self, date: &DateTime<Utc>, to: &crev_data::PubId) {
        if let Some(url) = to.url.as_ref() {
            self.url_by_id_secondary
                .entry(to.id.clone())
                .or_insert_with(|| UrlInfo {
                    url: url.clone(),
                    date: date.clone(),
                });
        }
    }

    fn record_url_from_from_field(&mut self, date: &DateTime<Utc>, from: &crev_data::PubId) {
        if let Some(url) = from.url.as_ref() {
            match self.url_by_id.entry(from.id.clone()) {
                hash_map::Entry::Occupied(mut entry) => {
                    entry.get_mut().maybe_update_with(date, &url)
                }
                hash_map::Entry::Vacant(entry) => {
                    entry.insert(UrlInfo {
                        url: url.clone(),
                        date: date.clone(),
                    });
                }
            }
        }
    }

    fn add_proof(&mut self, proof: &proof::Proof) {
        proof
            .verify()
            .expect("All proofs were supposed to be valid here");
        match proof.content {
            Content::Code(ref review) => self.add_code_review(&review),
            Content::Project(ref review) => self.add_project_review(&review, &proof.signature),
            Content::Trust(ref trust) => self.add_trust(&trust),
        }
    }

    /*
        fn import_file(&mut self, path: &Path) -> Result<()> {
            let proofs = proof::Proof::parse_from(path)?;
            for proof in proofs.into_iter() {
                util::err_eprint_and_ignore(self.add_proof(&proof).compat());
            }

            Ok(())
        }

        fn maybe_import_file(&mut self, path: &Path) -> Option<Result<()>> {
            let osext_match: &OsStr = "crev".as_ref();
            match path.extension() {
                Some(osext) if osext == osext_match => Some(self.import_file(path)),
                _ => None,
            }
        }

        pub fn import_recursively(&mut self, path: &Path) -> Result<()> {
            for entry in WalkDir::new(path).into_iter().filter_map(|e| match e {
                Err(e) => {
                    eprintln!("Error iterating {}: {}", path.display(), e);
                    None
                }
                Ok(o) => Some(o),
            }) {
                let path = entry.path();

                if !path.is_file() {
                    continue;
                }

                let res = self.maybe_import_file(&path).unwrap_or(Ok(()));
                util::err_eprint_and_ignore(
                    res.with_context(|e| format!("Failed to import file: {}; {}", path.display(), e))
                        .compat(),
                );
            }

            Ok(())
        }

    */

    pub fn import_from_iter(&mut self, i: impl Iterator<Item = proof::Proof>) {
        for proof in i {
            self.add_proof(&proof);
        }
    }

    fn get_ids_trusted_by(&self, id: &Id) -> impl Iterator<Item = (Level, &Id)> {
        if let Some(map) = self.trust_id_to_id.get(id) {
            Some(map.iter().map(|(id, trust_info)| (trust_info.trust, id)))
        } else {
            None
        }
        .into_iter()
        .flatten()
    }

    // Oh god, please someone verify this :D
    pub fn calculate_trust_set(&self, for_id: Id, params: &TrustDistanceParams) -> HashSet<Id> {
        #[derive(PartialOrd, Ord, Eq, PartialEq, Clone, Debug)]
        struct Visit {
            distance: u64,
            id: Id,
        }
        let mut pending = BTreeSet::new();
        pending.insert(Visit {
            distance: 0,
            id: for_id.clone(),
        });

        let mut visited = HashMap::<&Id, _>::new();
        visited.insert(&for_id, 0);
        while let Some(current) = pending.iter().next().cloned() {
            pending.remove(&current);

            if let Some(visited_distance) = visited.get(&current.id) {
                if *visited_distance < current.distance {
                    continue;
                }
            }

            for (level, candidate_id) in self.get_ids_trusted_by(&&current.id) {
                let candidate_distance_from_current =
                    if let Some(v) = params.distance_by_level(level) {
                        v
                    } else {
                        continue;
                    };
                let candidate_total_distance = current.distance + candidate_distance_from_current;
                if candidate_total_distance > params.max_distance {
                    continue;
                }

                if let Some(prev_candidate_distance) = visited.get(candidate_id).cloned() {
                    if prev_candidate_distance > candidate_total_distance {
                        visited.insert(candidate_id, candidate_total_distance);
                        pending.insert(Visit {
                            distance: candidate_total_distance,
                            id: candidate_id.to_owned(),
                        });
                    }
                } else {
                    visited.insert(candidate_id, candidate_total_distance);
                    pending.insert(Visit {
                        distance: candidate_total_distance,
                        id: candidate_id.to_owned(),
                    });
                }
            }
        }

        visited.keys().map(|id| (*id).clone()).collect()
    }

    pub fn lookup_url(&self, id: &Id) -> Option<&str> {
        self.url_by_id
            .get(id)
            .or_else(|| self.url_by_id_secondary.get(id))
            .map(|url_info| url_info.url.url.as_str())
    }
}

pub struct TrustDistanceParams {
    pub max_distance: u64,
    pub high_trust_distance: u64,
    pub medium_trust_distance: u64,
    pub low_trust_distance: u64,
}

impl TrustDistanceParams {
    fn distance_by_level(&self, level: Level) -> Option<u64> {
        Some(match level {
            Level::None => return None,
            Level::Low => self.low_trust_distance,
            Level::Medium => self.medium_trust_distance,
            Level::High => self.high_trust_distance,
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
