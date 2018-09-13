use chrono::{self, offset::Utc};
use crev_data::{
    self,
    proof::{self, Content},
};
use std::{
    collections::{hash_map, HashMap},
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
    fn maybe_update_with(&mut self, trust: &proof::Trust) {
        if trust.date().with_timezone(&Utc) > self.date {
            self.trust = trust.trust;
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
}

impl TrustDB {
    pub fn new() -> Self {
        TrustDB {
            id_to_trust: Default::default(),
            id_to_review: Default::default(),
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

    fn add_trust(&mut self, trust: &proof::Trust) {
        let from = &trust.from;
        for to in &trust.trusted_ids {
            match self
                .id_to_trust
                .entry(from.clone())
                .or_insert_with(|| HashMap::new())
                .entry(to.clone())
            {
                hash_map::Entry::Occupied(mut entry) => entry.get_mut().maybe_update_with(&trust),
                hash_map::Entry::Vacant(mut entry) => {
                    entry.insert(TrustInfo::from(trust));
                }
            }
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

    pub fn import_recursively(path: &Path) -> Result<Self> {
        let mut graph = TrustDB::new();

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

            match graph.import_file(&path) {
                Err(e) => eprintln!("Error importing {}: {}", path.display(), e),
                Ok(_) => {}
            }
        }
        unimplemented!();
    }
}
