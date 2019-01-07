use crate::id::PassphraseFn;
use crate::prelude::*;
use crate::proofdb::TrustSet;
use crate::ProofStore;
use crate::{local::Local, util};
use crev_common::convert::OptionDeref;
use crev_data::proof;
use crev_data::Digest;
use git2;
use serde_yaml;
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

#[derive(Fail, Debug)]
#[fail(display = "Package config not-initialized. Use `crev package init` to generate it.")]
struct PackageDirNotFound;

fn find_package_root_dir() -> Result<PathBuf> {
    let mut path = PathBuf::from(".").canonicalize()?;
    loop {
        if path.join(CREV_DOT_NAME).is_dir() {
            return Ok(path);
        }
        path = if let Some(parent) = path.parent() {
            parent.to_owned()
        } else {
            return Err(PackageDirNotFound.into());
        }
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
            bail!("`{}` already exists", config_path.display());
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
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "directory not found",
            ))?;
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
        config.ok_or_else(|| {
            format_err!("Package config not-initialized. Use `crev package init` to generate it.")
        })
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

    pub fn get_proof_rel_store_path(&self, proof: &proof::Proof) -> PathBuf {
        PathBuf::from("proofs").join(crate::proof::rel_package_path(&proof.content))
    }

    pub fn package_verify(
        &mut self,
        local: &Local,
        allow_dirty: bool,
        for_id: Option<String>,
        params: &crate::TrustDistanceParams,
    ) -> Result<crate::VerificationStatus> {
        if !allow_dirty && self.is_unclean()? {
            bail!("Git repository is not in a clean state");
        }

        let db = local.load_db()?;

        let trust_set = if let Some(id) = local.get_for_id_from_str_opt(for_id.as_deref())? {
            db.calculate_trust_set(&id, &params)
        } else {
            TrustSet::default()
        };
        let ignore_list = HashSet::new();
        let digest = crate::get_recursive_digest_for_git_dir(&self.root_dir, &ignore_list)?;
        Ok(db.verify_package_digest(&digest, &trust_set))
    }

    pub fn package_digest(&mut self, allow_dirty: bool) -> Result<Digest> {
        if !allow_dirty && self.is_unclean()? {
            bail!("Git repository is not in a clean state");
        }

        let ignore_list = HashSet::new();
        Ok(crate::get_recursive_digest_for_git_dir(
            &self.root_dir,
            &ignore_list,
        )?)
    }

    fn is_unclean(&self) -> Result<bool> {
        let git_repo = git2::Repository::open(&self.root_dir)?;
        if git_repo.state() != git2::RepositoryState::Clean {
            bail!("Git repository is not in a clean state");
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
            .ok_or_else(|| format_err!("HEAD target does not resolve to oid"))?
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
        bail!("Couldn't identify revision info");
    }

    pub fn trust_package(
        &mut self,
        passphrase_callback: PassphraseFn,
        allow_dirty: bool,
    ) -> Result<()> {
        if !self.staging()?.is_empty() {
            bail!("Can't review with uncommitted staged files.");
        }

        if !allow_dirty && self.is_unclean()? {
            bail!("Git repository is not in a clean state");
        }

        let local = Local::auto_open()?;
        let _revision = self.read_revision()?;

        let ignore_list = HashSet::new();
        let _digest = crate::get_recursive_digest_for_git_dir(&self.root_dir, &ignore_list)?;
        let id = local.read_current_unlocked_id(passphrase_callback)?;

        let review = proof::review::PackageBuilder::default()
            .from(id.id.to_owned())
            .build()
            .map_err(|e| format_err!("{}", e))?;

        let review = util::edit_proof_content_iteractively(&review.into())?;

        let proof = review.sign_by(&id)?;

        self.save_signed_review(&local, &proof)?;
        Ok(())
    }

    pub fn commit(&mut self, passphrase_callback: PassphraseFn, allow_dirty: bool) -> Result<()> {
        if self.staging()?.is_empty() && !allow_dirty {
            bail!("No reviews to commit. Use `add` first or use `-a` for the whole package.");
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
            .map_err(|e| format_err!("{}", e))?;

        let review = util::edit_proof_content_iteractively(&review.into())?;

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
