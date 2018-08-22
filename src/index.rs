use blake2::{self, digest::FixedOutput, Digest};
use common_failures::prelude::*;
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
pub struct StagedPathInfo {
    blake_hash: Vec<u8>,
}

pub struct Staged {
    file_path: PathBuf,
    entries: HashMap<PathBuf, StagedPathInfo>,
}

impl Staged {
    pub fn auto_open() -> Result<Self> {
        let project_dir = util::project_dir_find()?;
        let index_file = project_dir.join("index");
        Self::read_from_file(&index_file)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn close(mut self) -> Result<()> {
        let path = self.file_path.clone();
        self.write_to_file(&path)
    }

    pub fn read_from_file(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self {
                file_path: path.to_owned(),
                entries: Default::default(),
            });
        }

        let file = fs::File::open(path)?;

        let path_info: HashMap<PathBuf, StagedPathInfo> = serde_cbor::from_reader(&file)?;

        Ok(Self {
            file_path: path.to_owned(),
            entries: path_info,
        })
    }

    pub fn write_to_file(&self, path: &Path) -> Result<()> {
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
            StagedPathInfo {
                blake_hash: blaze2sum(path)?,
            },
        );

        Ok(())
    }
}
