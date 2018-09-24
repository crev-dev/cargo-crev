use blake2;
use crate::Result;
use crev_common;
use digest::{Digest, FixedOutput};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    os::unix::ffi::OsStrExt,
    path::{Component, Path, PathBuf},
};

pub type Descendants = BTreeMap<OsString, Entry>;

/*
pub enum Entry {
    Dir(Descendants),
    Link { target: PathBuf },
    File { digest: Vec<u8> },
}
*/

#[derive(Default)]
pub struct Entry(Descendants);

pub struct RecursiveHasher {
    root_path: PathBuf,
    root: Entry,
}

impl RecursiveHasher {
    pub fn new_dir(path: PathBuf) -> Self {
        Self {
            root_path: path,
            root: Entry(Default::default()),
        }
    }

    pub fn insert_path(&mut self, path: &Path) {
        let mut cur_entry = &mut self.root;
        for comp in path.components() {
            match comp {
                Component::Normal(osstr) => {
                    cur_entry = cur_entry.0.entry(osstr.to_owned()).or_default();
                }
                _ => panic!("Didn't expect {:?}", comp),
            }
        }
    }

    pub fn get_digest(&self) -> Result<Vec<u8>> {
        let mut hasher = blake2::Blake2b::new();

        self.get_input_for(&self.root_path, &self.root, &mut hasher)?;

        Ok(hasher.fixed_result().to_vec())
    }

    fn get_input_for(
        &self,
        full_path: &Path,
        entry: &Entry,
        hasher: &mut blake2::Blake2b,
    ) -> Result<()> {
        let attr = fs::symlink_metadata(full_path)?;
        if attr.is_file() {
            self.get_input_for_file(full_path, entry, hasher)?;
        } else if attr.is_dir() {
            self.get_input_for_dir(full_path, entry, hasher)?;
        } else if attr.file_type().is_symlink() {
            self.get_input_for_symlink(full_path, entry, hasher)?;
        } else {
            eprintln!("Skipping {} - not supported file type", full_path.display());
        }

        Ok(())
    }

    fn get_input_for_dir(
        &self,
        full_path: &Path,
        entry: &Entry,
        parent_hasher: &mut blake2::Blake2b,
    ) -> Result<()> {
        parent_hasher.input("D\0".as_bytes());
        let mut hasher = blake2::Blake2b::new();
        for (k, v) in &entry.0 {
            hasher.input(k.as_bytes());
            hasher.input("\0".as_bytes());

            let full_path = full_path.join(k);
            let attr = fs::symlink_metadata(&full_path)?;

            if attr.is_file() {
                self.get_input_for_file(&full_path, &v, &mut hasher)?;
            } else if attr.is_dir() {
                self.get_input_for_dir(&full_path, &v, &mut hasher)?;
            } else if attr.file_type().is_symlink() {
                self.get_input_for_symlink(&full_path, &v, &mut hasher)?;
            }
        }
        parent_hasher.input(hasher.fixed_result().as_slice());

        Ok(())
    }

    fn get_input_for_file(
        &self,
        full_path: &Path,
        entry: &Entry,
        parent_hasher: &mut blake2::Blake2b,
    ) -> Result<()> {
        assert!(entry.0.is_empty());
        parent_hasher.input("F\0".as_bytes());
        crev_common::read_file_to_digest_input(full_path, parent_hasher)?;
        Ok(())
    }

    fn get_input_for_symlink(
        &self,
        full_path: &Path,
        entry: &Entry,
        parent_hasher: &mut blake2::Blake2b,
    ) -> Result<()> {
        assert!(entry.0.is_empty());
        parent_hasher.input("L\0".as_bytes());
        parent_hasher.input(full_path.read_link()?.as_os_str().as_bytes());
        Ok(())
    }
}
