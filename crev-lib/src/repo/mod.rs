use crate::{local::Local, util, verify_package_digest, Error, Result};
use crev_common::convert::OptionDeref;
use crev_data::{proof, Digest};
use serde::{Deserialize, Serialize};

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

pub mod staging;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackageConfig {
    pub version: u64,
    #[serde(rename = "trust-root")]
    pub trust_root: String,
}

const CREV_DOT_NAME: &str = ".crev";

#[derive(thiserror::Error, Debug)]
#[error("Package config not-initialized. Use `crev package init` to generate it.")]
pub struct PackageDirNotFound;

fn find_package_root_dir() -> std::result::Result<PathBuf, PackageDirNotFound> {
    let path = Path::new(".")
        .canonicalize()
        .map_err(|_| PackageDirNotFound)?;
    let mut path = path.as_path();
    loop {
        if path.join(CREV_DOT_NAME).is_dir() {
            return Ok(path.to_owned());
        }
        path = path.parent().ok_or(PackageDirNotFound)?;
    }
}

/// `crev` repository dir inside a package dir
///
/// This represents the `.crev` directory and all
/// the internals of it.
pub struct Repo {
    /// root dir, where `.crev` subdiretory resides
    root_dir: PathBuf,
    /// lazily loaded `Staging`
    staging: Option<staging::Staging>,
}

impl Repo {
    pub fn init(path: &Path, id_str: String) -> Result<Self> {
        let repo = Self::new(path)?;

        fs::create_dir_all(repo.dot_crev_path())?;

        let config_path = repo.package_config_path();
        if config_path.exists() {
            Err(Error::PathAlreadyExists(config_path.as_path().into()))?;
        }
        util::store_to_file_with(&config_path, move |w| {
            serde_yaml::to_writer(
                w,
                &PackageConfig {
                    version: 0,
                    trust_root: id_str.clone(),
                },
            )
        })??;

        Ok(repo)
    }

    pub fn open(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(
                std::io::Error::new(std::io::ErrorKind::NotFound, "directory not found").into(),
            );
        }

        Self::new(path)
    }

    pub fn auto_open() -> Result<Self> {
        let root_path = find_package_root_dir()?;
        Self::open(&root_path)
    }

    #[allow(clippy::new_ret_no_self)]
    fn new(root_dir: &Path) -> Result<Self> {
        let root_dir = root_dir.canonicalize()?;
        Ok(Self {
            root_dir,
            staging: None,
        })
    }

    fn package_config_path(&self) -> PathBuf {
        self.dot_crev_path().join("config.yaml")
    }

    pub fn load_package_config(&self) -> Result<PackageConfig> {
        let config = self.try_load_package_config()?;
        config.ok_or_else(|| Error::PackageConfigNotInitialized)
    }

    pub fn try_load_package_config(&self) -> Result<Option<PackageConfig>> {
        let path = self.package_config_path();

        if !path.exists() {
            return Ok(None);
        }
        let config_str = util::read_file_to_string(&path)?;

        Ok(Some(serde_yaml::from_str(&config_str)?))
    }

    pub fn dot_crev_path(&self) -> PathBuf {
        self.root_dir.join(CREV_DOT_NAME)
    }

    pub fn staging(&mut self) -> Result<&mut staging::Staging> {
        if self.staging.is_none() {
            self.staging = Some(staging::Staging::open(&self.root_dir)?);
        }
        Ok(self.staging.as_mut().unwrap())
    }

    pub fn get_proof_rel_store_path(&self, _proof: &proof::Proof) -> PathBuf {
        unimplemented!();
    }

    pub fn package_verify(
        &mut self,
        local: &Local,
        allow_dirty: bool,
        for_id: Option<String>,
        params: &crate::TrustDistanceParams,
        requirements: &crate::VerificationRequirements,
    ) -> Result<crate::VerificationStatus> {
        if !allow_dirty && self.is_unclean()? {
            Err(Error::GitRepositoryIsNotInACleanState)?;
        }

        let db = local.load_db()?;

        let trust_set =
            if let Some(id) = local.get_for_id_from_str_opt(OptionDeref::as_deref(&for_id))? {
                db.calculate_trust_set(&id, &params)
            } else {
                crev_wot::TrustSet::default()
            };
        let ignore_list = fnv::FnvHashSet::default();
        let digest = crate::get_recursive_digest_for_git_dir(&self.root_dir, &ignore_list)?;
        Ok(verify_package_digest(
            &digest,
            &trust_set,
            requirements,
            &db,
        ))
    }

    pub fn package_digest(&mut self, allow_dirty: bool) -> Result<Digest> {
        if !allow_dirty && self.is_unclean()? {
            Err(Error::GitRepositoryIsNotInACleanState)?;
        }

        let ignore_list = HashSet::default();
        Ok(crate::get_recursive_digest_for_git_dir(
            &self.root_dir,
            &ignore_list,
        )?)
    }

    fn is_unclean(&self) -> Result<bool> {
        let git_repo = git2::Repository::open(&self.root_dir)?;
        if git_repo.state() != git2::RepositoryState::Clean {
            Err(Error::GitRepositoryIsNotInACleanState)?;
        }
        let mut status_opts = git2::StatusOptions::new();
        status_opts.include_unmodified(true);
        status_opts.include_untracked(false);
        let mut unclean_found = false;
        for entry in git_repo.statuses(Some(&mut status_opts))?.iter() {
            if entry.status() != git2::Status::CURRENT {
                unclean_found = true;
            }
        }

        Ok(unclean_found)
    }

    pub fn status(&mut self) -> Result<()> {
        let staging = self.staging()?;
        for (k, _v) in staging.entries.iter() {
            println!("{}", k.display());
        }

        Ok(())
    }

    pub fn add(&mut self, file_paths: Vec<PathBuf>) -> Result<()> {
        let staging = self.staging()?;
        for path in file_paths {
            staging.insert(&path)?;
        }
        staging.save()?;

        Ok(())
    }

    pub fn remove(&mut self, file_paths: Vec<PathBuf>) -> Result<()> {
        let staging = self.staging()?;
        for path in file_paths {
            staging.remove(&path)?;
        }
        staging.save()?;

        Ok(())
    }
}
