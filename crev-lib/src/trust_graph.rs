use crev_data::proof;
use std::{ffi::OsStr, path::Path};
use walkdir::WalkDir;
use Result;

pub struct TrustGraph;

impl TrustGraph {
    fn new() -> Self {
        TrustGraph
    }

    fn add_proof(&mut self, _proof: &proof::Proof) {
        unimplemented!();
    }

    fn import_from_file(&mut self, path: &Path) -> Result<()> {
        let review_osext: &OsStr = "review.crev".as_ref();
        let trust_osext: &OsStr = "trust.crev".as_ref();
        match path.extension() {
            Some(osext) if osext == review_osext || osext == trust_osext => {
                let proofs = proof::Proof::parse_from(path)?;
                for proof in proofs.into_iter() {
                    self.add_proof(&proof);
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
