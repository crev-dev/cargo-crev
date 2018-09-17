use crate::{local::Local, trustdb, util, Result};
use crev_data::{proof, review};
use git2;
use serde_yaml;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub mod staging;

struct RevisionInfo {
    pub type_: String,
    pub revision: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ProjectConfig {
    pub version: u64,
    #[serde(rename = "project-id")]
    pub project_id: String,
    #[serde(rename = "project-trust-root")]
    pub project_trust_root: String,
}

const CREV_DOT_NAME: &str = ".crev";

#[derive(Fail, Debug)]
#[fail(display = "Project config not-initialized. Use `crev init` to generate it.")]
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
    // root dir, where `.crev` subdiretory resides
    root_dir: PathBuf,
    // lazily loaded `Staging`
    staging: Option<staging::Staging>,
}

impl Repo {
    pub fn init(path: PathBuf, id_str: String) -> Result<Self> {
        let repo = Self::new(path)?;

        fs::create_dir_all(repo.dot_crev_path())?;

        let config_path = repo.project_config_path();
        if config_path.exists() {
            bail!("`{}` already exists", config_path.display());
        }
        util::store_to_file_with(&config_path, move |w| {
            serde_yaml::to_writer(
                w,
                &ProjectConfig {
                    version: 0,
                    project_id: util::random_id_str(),
                    project_trust_root: id_str.clone(),
                },
            )?;

            Ok(())
        })?;

        Ok(repo)
    }

    pub fn auto_open() -> Result<Self> {
        let root_path = find_project_root_dir()?;
        let res = Self::new(root_path)?;

        if !res.project_config_path().exists() {
            bail!("Project config not-initialized. Use `crev init` to generate it.");
        }

        Ok(res)
    }

    pub fn new(root_dir: PathBuf) -> Result<Self> {
        let root_dir = root_dir.canonicalize()?;
        Ok(Self {
            root_dir,
            staging: None,
        })
    }

    fn project_config_path(&self) -> PathBuf {
        self.dot_crev_path().join("config.yaml")
    }

    fn load_project_config(&self) -> Result<ProjectConfig> {
        let path = self.project_config_path();

        let config_str = util::read_file_to_string(&path)?;

        Ok(serde_yaml::from_str(&config_str)?)
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

    fn append_proof_at(&mut self, proof: proof::Proof, rel_store_path: &Path) -> Result<()> {
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
        PathBuf::from("proofs").join(crate::proof::rel_project_path(&proof.content))
    }

    pub fn verify(&mut self) -> Result<()> {
        let local = Local::auto_open()?;
        let user_config = local.load_user_config()?;
        let _cur_id = user_config.current_id;
        let _graph = trustdb::TrustDB::new(); /* TODO: calculate trust graph */
        /*
        let user_config = Local::read_unlocked_id
        let trustdb = Local::calculate_trustdb_for(&id);
        */

        unimplemented!();
    }

    fn try_read_git_revision(&self) -> Result<Option<RevisionInfo>> {
        let dot_git_path = self.root_dir.join(".git");
        if !dot_git_path.exists() {
            return Ok(None);
        }
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
                eprintln!("{}", entry.path().unwrap());
                unclean_found = true;
            }
        }
        if unclean_found {
            bail!("Git repository is not in a clean state");
        }
        let head = git_repo.head()?;
        let rev = head
            .resolve()?
            .target()
            .ok_or_else(|| format_err!("HEAD target does not resolve to oid"))?
            .to_string();
        Ok(Some(RevisionInfo {
            type_: "git".into(),
            revision: rev,
        }))
    }

    fn read_revision(&self) -> Result<RevisionInfo> {
        if let Some(info) = self.try_read_git_revision()? {
            return Ok(info);
        }
        bail!("Couldn't identify revision info");
    }

    pub fn commit(&mut self, passphrase: String) -> Result<()> {
        if self.staging()?.is_empty() {
            bail!("No reviews to commit. Use `add` first.");
        }
        let local = Local::auto_open()?;
        let id = local.read_unlocked_id(&passphrase)?;
        let project_config = self.load_project_config()?;
        let revision = self.read_revision()?;
        self.staging()?.enforce_current()?;
        let files = self.staging()?.to_review_files();

        let from = proof::Id::from(&id.id);

        let review = review::ReviewBuilder::default()
            .from(from)
            .revision(revision.revision)
            .revision_type(revision.type_)
            .project_id(project_config.project_id)
            .files(files)
            .build()
            .map_err(|e| format_err!("{}", e))?;

        let review =
            util::edit_proof_content_iteractively(&review.into(), proof::ProofType::Review)?;

        let proof = review.sign(&id)?;

        let rel_store_path = self.get_proof_rel_store_path(&proof);

        println!("{}", proof.clone());
        self.append_proof_at(proof.clone(), &rel_store_path)?;
        eprintln!(
            "Proof written to: {}",
            PathBuf::from(".crev").join(rel_store_path).display()
        );
        let local = Local::auto_open()?;
        local.append_proof(&proof)?;
        eprintln!("Proof added to your store");
        self.staging()?.wipe()?;
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
