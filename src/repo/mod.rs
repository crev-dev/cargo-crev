use std::{
    fs,
    path::{Path, PathBuf},
};
use util;
pub mod staging;
use Result;

const CREV_DOT_NAME: &str = ".crev";

#[derive(Fail, Debug)]
#[fail(display = "`.crew` project dir not found")]
struct ProjectDirNotFound;

fn find_project_root_dir() -> Result<PathBuf> {
    let mut path = PathBuf::from(".").canonicalize()?;
    loop {
        if path.join(CREV_DOT_NAME).is_dir() {
            return Ok(path);
        }
        path = if let Some(parent) = path.parent() {
            parent.to_owned()
        } else {
            return Err(ProjectDirNotFound.into());
        }
    }
}

/// `crev` repository
///
/// This represents the `.crev` directory and all
/// the internals of it.
pub struct Repo {
    root_dir: PathBuf,
}

impl Repo {
    pub fn init(path: PathBuf) -> Result<Self> {
        fs::create_dir_all(CREV_DOT_NAME)?;
        Self::open(path)
    }

    pub fn auto_open() -> Result<Self> {
        let root_dir = find_project_root_dir()?;
        Self::open(root_dir)
    }

    pub fn open(root_dir: PathBuf) -> Result<Self> {
        let root_dir = root_dir.canonicalize()?;
        Ok(Self { root_dir })
    }

    pub fn staging(&mut self) -> &mut staging::Staging {
        unimplemented!();
    }
}
