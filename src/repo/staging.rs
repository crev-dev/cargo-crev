use blake2::{self, digest::FixedOutput, Digest};
use common_failures::prelude::*;
use proof::ReviewFile;
use serde_cbor;
use std::{
    collections::HashMap,
    ffi::OsString,
    fs, io,
    io::{BufRead, BufReader, Write},
    os::unix::ffi::OsStringExt,
    path::{Path, PathBuf},
};
use util;

fn blaze2sum(path: &Path) -> Result<Vec<u8>> {
    let file = fs::File::open(path)?;

    let mut reader = io::BufReader::new(file);
    let mut hasher = blake2::Blake2b::new();

    loop {
        let length = {
            let buffer = reader.fill_buf()?;
            hasher.input(buffer);
            buffer.len()
        };
        if length == 0 {
            break;
        }
        reader.consume(length);
    }
    Ok(hasher.fixed_result().to_vec())
}

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
    pub fn wipe(mut self) -> Result<()> {
        Ok(fs::remove_file(self.file_path)?)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn close(&mut self) -> Result<()> {
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
        let tmp_path = path.with_file_name("tmp");
        let mut file = fs::File::create(&tmp_path)?;
        serde_cbor::to_writer(&mut file, &self.entries)?;
        file.flush()?;
        drop(file);
        fs::rename(tmp_path, path)?;
        Ok(())
    }

    pub fn insert(&mut self, path: &Path) -> Result<()> {
        self.entries.insert(
            path.to_owned(),
            StagingPathInfo {
                blake_hash: blaze2sum(path)?,
            },
        );

        Ok(())
    }

    pub fn to_review_files(&self) -> Vec<ReviewFile> {
        self.entries
            .iter()
            .map(|(k, v)| ReviewFile {
                path: k.to_owned(),
                digest: v.blake_hash.clone(),
                digest_type: "blake2b".into(),
            }).collect()
    }
}
