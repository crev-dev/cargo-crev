use chrono::{self, offset::Utc, DateTime};
use crev_data::{
    self,
    level::Level,
    proof::{self, Content},
};
use std::{
    collections::{hash_map, BTreeMap, BTreeSet, HashMap, HashSet},
    ffi::OsStr,
    path::Path,
};
use walkdir::WalkDir;
use Result;

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
    trust: crev_data::level::Level,
    understanding: crev_data::level::Level,
    thoroughness: crev_data::level::Level,
}

impl<'a> From<&'a proof::Review> for ReviewInfo {
    fn from(review: &proof::Review) -> Self {
        ReviewInfo {
            trust: review.trust,
            understanding: review.understanding,
            thoroughness: review.thoroughness,
            date: review.date().with_timezone(&Utc),
        }
    }
}

impl ReviewInfo {
    fn maybe_update_with(&mut self, review: &proof::Review) {
        if review.date().with_timezone(&Utc) > self.date {
            self.trust = review.trust;
            self.understanding = review.understanding;
            self.thoroughness = review.thoroughness;
        }
    }
}

pub struct TrustDB {
    #[allow(unused)]
    id_to_trust: HashMap<String, HashMap<String, TrustInfo>>, // who -(trusts)-> whom
    id_to_review: HashMap<String, HashMap<Vec<u8>, ReviewInfo>>, // who -(reviewed)-> what
    id_to_urls: HashMap<String, BTreeMap<chrono::DateTime<Utc>, String>>,
}

impl TrustDB {
    pub fn new() -> Self {
        TrustDB {
            id_to_trust: Default::default(),
            id_to_review: Default::default(),
            id_to_urls: Default::default(),
        }
    }

    fn add_review(&mut self, review: &proof::Review) {
        let from = &review.from;
        for file in &review.files {
            match self
                .id_to_review
                .entry(from.clone())
                .or_insert_with(|| HashMap::new())
                .entry(file.digest.to_owned())
            {
                hash_map::Entry::Occupied(mut entry) => entry.get_mut().maybe_update_with(&review),
                hash_map::Entry::Vacant(mut entry) => {
                    entry.insert(ReviewInfo::from(review));
                }
            }
        }
    }

    pub fn add_trust_raw(&mut self, from: &str, to: &str, date: DateTime<Utc>, trust: Level) {
        match self
            .id_to_trust
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

    pub fn add_trust(&mut self, trust: &proof::Trust) {
        let from = &trust.from;
        for to in &trust.trusted_ids {
            self.add_trust_raw(from, to, trust.date_utc(), trust.trust)
        }
    }

    fn add_proof(&mut self, proof: &proof::Proof) -> Result<()> {
        proof.verify()?;
        match proof.content {
            Content::Review(ref review) => self.add_review(&review),
            Content::Trust(ref trust) => self.add_trust(&trust),
        }

        Ok(())
    }

    fn import_file(&mut self, path: &Path) -> Result<()> {
        let osext_match: &OsStr = "crev".as_ref();
        match path.extension() {
            Some(osext) if osext == osext_match => {
                let proofs = proof::Proof::parse_from(path)?;
                for proof in proofs.into_iter() {
                    // TODO: report&ignore errors
                    self.add_proof(&proof)?;
                }
            }
            _ => bail!("Wrong type"),
        }

        Ok(())
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

            match self.import_file(&path) {
                Err(e) => eprintln!("Error importing {}: {}", path.display(), e),
                Ok(_) => {}
            }
        }

        Ok(())
    }

    fn get_id_trusted_by(&self, id: &str) -> impl Iterator<Item = (Level, &str)> {
        if let Some(map) = self.id_to_trust.get(id) {
            Some(
                map.iter()
                    .map(|(id, trust_info)| (trust_info.trust, id.as_str())),
            )
        } else {
            None
        }.into_iter()
        .flatten()
    }

    // Oh god, please someone verify this :D
    pub fn calculate_trust_set(
        &self,
        for_id: String,
        params: &TrustDistanceParams,
    ) -> HashSet<String> {
        #[derive(PartialOrd, Ord, Eq, PartialEq, Clone)]
        struct Visit {
            distance: u64,
            id: String,
        }
        let mut pending = BTreeSet::new();
        pending.insert(Visit {
            distance: 0,
            id: for_id,
        });

        let mut visited = HashMap::<&str, _>::new();
        while let Some(current) = pending.iter().next().cloned() {
            pending.remove(&current);

            if let Some(visited_distance) = visited.get(current.id.as_str()) {
                if *visited_distance < current.distance {
                    continue;
                }
            }

            for (level, candidate_id) in self.get_id_trusted_by(&current.id) {
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

        visited.keys().map(|s| s.to_string()).collect()
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
