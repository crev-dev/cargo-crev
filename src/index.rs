use common_failures::prelude::*;
use std::path::Path;

pub struct Index;

impl Index {
    pub fn read_fom_file(path: &Path) -> Result<Self> {
        unimplemented!();
    }

    pub fn write_to_file(&self, path: &Path) -> Result<Self> {
        unimplemented!();
    }

    pub fn insert(&mut self, path: &Path) {
        unimplemented!();;
    }
}
