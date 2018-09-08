use crev_data::proof;
use std::{ffi::OsStr, path::Path};
use walkdir::WalkDir;
use Result;

pub struct TrustGraph;

impl TrustGraph {
    fn new() -> Self {
        TrustGraph
    }

    fn add_review_proof(&mut self, _review: &proof::ReviewProof) {
        unimplemented!();
    }

    fn add_trust_proof(&mut self, _review: &proof::TrustProof) {
        unimplemented!();
    }

    fn import_from_file(&mut self, path: &Path) -> Result<()> {
        let review_osext: &OsStr = proof::review::PROOF_EXTENSION.as_ref();
        let trust_osext: &OsStr = proof::trust::PROOF_EXTENSION.as_ref();
        match path.extension() {
            Some(osext) if osext == review_osext => {
                let proofs = proof::ReviewProof::parse_from(path)?;
                for proof in proofs.into_iter() {
                    self.add_review_proof(&proof);
                }
            }
            Some(osext) if osext == trust_osext => {
                let proofs = proof::TrustProof::parse_from(path)?;
                for proof in proofs.into_iter() {
                    self.add_trust_proof(&proof);
                }
            }
            _ => bail!("Wrong type"),
        }

        Ok(())
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        let mut graph = TrustGraph::new();

        for entry in WalkDir::new(path).into_iter().filter_map(|e| match e {
            Err(e) => {
                eprintln!("Error iterating {}: {}", path.display(), e);
                None
            }
            Ok(o) => Some(o),
        }) {
            let path = entry.path();

            match graph.import_from_file(&path) {
                Err(e) => eprintln!("Error importing {}: {}", path.display(), e),
                Ok(_) => {}
            }
        }
        unimplemented!();
    }
}
