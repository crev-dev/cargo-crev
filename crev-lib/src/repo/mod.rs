use crate::{
    id::PassphraseFn, local::Local, util, verify_package_digest, Error, ProofStore, Result,
};
use crev_common::convert::OptionDeref;
use crev_data::{
    proof::{self, ContentExt},
    Digest,
};
use serde::{Deserialize, Serialize};

use std::{
    collections::HashSet,
    fs,
    io::Write,
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

    fn append_proof_at(&mut self, proof: &proof::Proof, rel_store_path: &Path) -> Result<()> {
        let path = self.dot_crev_path().join(rel_store_path);

        fs::create_dir_all(path.parent().expect("Not a root dir"))?;
        let mut file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .write(true)
            .open(path)?;

        file.write_all(proof.to_string().as_bytes())?;
        file.flush()?;

        Ok(())
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

    fn try_read_git_revision(&self) -> Result<Option<crev_data::proof::Revision>> {
        let dot_git_path = self.root_dir.join(".git");
        if !dot_git_path.exists() {
            return Ok(None);
        }
        let git_repo = git2::Repository::open(&self.root_dir)?;

        let head = git_repo.head()?;
        let rev = head
            .resolve()?
            .target()
            .ok_or_else(|| Error::HEADTargetDoesNotResolveToOid)?
            .to_string();
        Ok(Some(crev_data::proof::Revision {
            revision_type: "git".into(),
            revision: rev,
        }))
    }

    fn read_revision(&self) -> Result<crev_data::proof::Revision> {
        if let Some(info) = self.try_read_git_revision()? {
            return Ok(info);
        }
        Err(Error::CouldNotIdentifyRevisionInfo)
    }

    pub fn trust_package(
        &mut self,
        passphrase_callback: PassphraseFn<'_>,
        allow_dirty: bool,
    ) -> Result<()> {
        if !self.staging()?.is_empty() {
            Err(Error::CanTReviewWithUncommittedStagedFiles)?;
        }

        if !allow_dirty && self.is_unclean()? {
            Err(Error::GitRepositoryIsNotInACleanState)?;
        }

        let local = Local::auto_open()?;
        let _revision = self.read_revision()?;

        let ignore_list = fnv::FnvHashSet::default();
        let _digest = crate::get_recursive_digest_for_git_dir(&self.root_dir, &ignore_list)?;
        let id = local.read_current_unlocked_id(passphrase_callback)?;

        let review = proof::review::PackageBuilder::default()
            .from(id.id.to_owned())
            .build()
            .map_err(|e| Error::PackageBuilder(e.into()))?;

        let review = util::edit_proof_content_iteractively(&review, None, None)?;

        let proof = review.sign_by(&id)?;

        self.save_signed_review(&local, &proof)?;
        Ok(())
    }

    pub fn commit(
        &mut self,
        passphrase_callback: PassphraseFn<'_>,
        allow_dirty: bool,
    ) -> Result<()> {
        if self.staging()?.is_empty() && !allow_dirty {
            Err(Error::NoReviewsToCommitUseAddFirstOrUseAForTheWholePackage)?;
        }

        let local = Local::auto_open()?;
        let _revision = self.read_revision()?;
        self.staging()?.enforce_current()?;
        let files = self.staging()?.to_review_files();
        let id = local.read_current_unlocked_id(passphrase_callback)?;

        let review = proof::review::CodeBuilder::default()
            .from(id.id.to_owned())
            .files(files)
            .build()
            .map_err(|e| Error::CodeBuilder(e.into()))?;

        let review = util::edit_proof_content_iteractively(&review, None, None)?;

        let proof = review.sign_by(&id)?;

        self.save_signed_review(&local, &proof)?;
        self.staging()?.wipe()?;
        Ok(())
    }

    fn save_signed_review(&mut self, local: &Local, proof: &proof::Proof) -> Result<()> {
        let rel_store_path = self.get_proof_rel_store_path(&proof);

        println!("{}", proof);
        self.append_proof_at(proof, &rel_store_path)?;
        eprintln!(
            "Proof written to: {}",
            PathBuf::from(".crev").join(rel_store_path).display()
        );
        local.insert(proof)?;
        eprintln!("Proof added to your store");

        Ok(())
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
