use common_failures::prelude::*;
use crev_common;
use crev_data::proof;
use failure::bail;
use serde::{Deserialize, Serialize};
use serde_cbor;
use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize, Debug)]
pub struct StagingPathInfo {
    blake_hash: Vec<u8>,
}

pub struct Staging {
    root_path: PathBuf,
    file_path: PathBuf,
    pub entries: HashMap<PathBuf, StagingPathInfo>,
}

const STAGING_FILE_NAME: &str = "staging";

impl Staging {
    pub fn wipe(&mut self) -> Result<()> {
        fs::remove_file(&self.file_path)?;
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn save(&mut self) -> Result<()> {
        self.write_to_file(&self.file_path)
    }

    pub fn open(repo_path: &Path) -> Result<Self> {
        let path = repo_path.join(super::CREV_DOT_NAME).join(STAGING_FILE_NAME);
        if !path.exists() {
            return Ok(Self {
                root_path: repo_path.to_owned(),
                file_path: path,
                entries: Default::default(),
            });
        }

        let file = fs::File::open(&path)?;

        let path_info: HashMap<PathBuf, StagingPathInfo> = serde_cbor::from_reader(&file)?;

        Ok(Self {
            root_path: repo_path.to_owned(),
            file_path: path,
            entries: path_info,
        })
    }

    fn write_to_file(&self, path: &Path) -> Result<()> {
        let tmp_path = path.with_extension("tmp");
        let mut file = fs::File::create(&tmp_path)?;
        serde_cbor::to_writer(&mut file, &self.entries)?;
        file.flush()?;
        drop(file);
        fs::rename(tmp_path, path)?;
        Ok(())
    }

    pub fn insert(&mut self, path: &Path) -> Result<()> {
        let full_path = path.canonicalize()?;

        let path = full_path.strip_prefix(&self.root_path)?.to_owned();
        println!("Adding {}", path.display());
        self.entries.insert(
            path.to_owned(),
            StagingPathInfo {
                blake_hash: crev_common::blake2b256sum_file(&full_path)?,
            },
        );

        Ok(())
    }

    pub fn remove(&mut self, path: &Path) -> Result<()> {
        let full_path = path.canonicalize()?;

        let path = full_path.strip_prefix(&self.root_path)?.to_owned();
        println!("Removing {}", path.display());

        self.entries.remove(&path);

        Ok(())
    }

    pub fn to_review_files(&self) -> Vec<proof::review::code::File> {
        self.entries
            .iter()
            .map(|(k, v)| proof::review::code::File {
                path: k.to_owned(),
                digest: v.blake_hash.clone(),
                digest_type: "blake2b".into(),
            })
            .collect()
    }

    pub fn enforce_current(&self) -> Result<()> {
        for (rel_path, info) in self.entries.iter() {
            let path = self.root_path.join(rel_path);
            if crev_common::blake2b256sum_file(&path)? != info.blake_hash {
                bail!(
                    "File {} not current. Review again use `crev add` to
                      update.",
                    rel_path.display()
                );
            }
        }

        Ok(())
    }
}
