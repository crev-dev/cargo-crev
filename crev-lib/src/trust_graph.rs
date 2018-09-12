use crev_data::{
    self,
    proof::{self, Content},
};
use std::{collections::HashMap, ffi::OsStr, path::Path};
use walkdir::WalkDir;
use Result;

struct TrustInfo {
    #[allow(unused)]
    level: crev_data::level::Level,
}

pub struct TrustGraph {
    #[allow(unused)]
    id_to_trusted: HashMap<String, HashMap<String, TrustInfo>>,
}

impl TrustGraph {
    pub fn new() -> Self {
        TrustGraph {
            id_to_trusted: Default::default(),
        }
    }

    fn add_review(&mut self, _review: &proof::Review) {
        unimplemented!();
    }

    fn add_trust(&mut self, _trust: &proof::Trust) {
        unimplemented!();
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
        let mut graph = TrustGraph::new();

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
