use crev_data::{self, trust::Trust, review::Review};
use std::path::PathBuf;
use chrono::prelude::*;

pub trait ContentExt {
    fn extension(&self) -> String {
        format!("{}.crev", Self::PROOF_EXTENSION)
    }

    /// The path to use under project `.crev/`
    fn rel_project_path(&self) -> PathBuf {
        PathBuf::from(self.from_pubid())
            .join(Self::CONTENT_TYPE_NAME)
            .join(self.date().with_timezone(&Utc).format("%Y-%m").to_string())
            .with_extension(format!("{}.crev", Self::PROOF_EXTENSION))
    }

    /// The path to use under user store
    fn rel_store_path(&self) -> PathBuf {
        let mut path = PathBuf::from(self.from_pubid()).join(Self::CONTENT_TYPE_NAME);

        if let Some(project_id) = self.project_id() {
            path = path.join(project_id)
        }

        path.join(self.date().with_timezone(&Utc).format("%Y-%m").to_string())
            .with_extension()
    }
}

impl<T: crev_data::proof::Content> ContentExt for T { }


