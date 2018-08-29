use chrono;
use id;
use level;
use local::Local;
use proof::{self, Content};
use review;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use trust;
use util;
use Result;

pub mod staging;

const CREV_DOT_NAME: &str = ".crev";

#[derive(Fail, Debug)]
#[fail(display = "`.crew` project dir not found. Use `crev init`?")]
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
        Ok(Self {
            root_dir,
            staging: None,
        })
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

    fn append_proof_at<T: proof::Content>(
        &mut self,
        proof: proof::Proof<T>,
        rel_store_path: &Path,
    ) -> Result<()> {
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

    pub fn get_proof_rel_store_path(&self, content: &impl proof::Content) -> PathBuf {
        PathBuf::from("proofs").join(content.rel_store_path())
    }

    pub fn commit(&mut self) -> Result<()> {
        if self.staging()?.is_empty() {
            bail!("No reviews to commit. Use `add` first.");
        }
        let passphrase = util::read_passphrase()?;
        let local = Local::auto_open()?;
        let id = local.read_unlocked_id(&passphrase)?;
        let pub_id = id.to_pubid();
        let files = self.staging()?.to_review_files();

        let review = review::ReviewBuilder::default()
            .from(id.pub_key_as_base64())
            .from_url(id.url().into())
            .from_type(id.type_as_string())
            .revision(Some("TODO".into()))
            .revision_type("git".into())
            .project_url("TODO".into())
            .comment(Some("".into()))
            .thoroughness(level::Level::Low)
            .understanding(level::Level::Low)
            .trust(level::Level::Low)
            .files(files)
            .build()
            .map_err(|e| format_err!("{}", e))?;

        let review = util::edit_proof_content_iteractively(&review)?;

        let proof = review.sign(&id)?;

        let rel_store_path = self.get_proof_rel_store_path(&review);

        println!("{}", proof.clone());
        self.append_proof_at(proof.clone(), &rel_store_path)?;
        eprintln!("Proof written to: {}", rel_store_path.display());
        let local = Local::auto_open()?;
        local.append_proof(&proof, &review);
        eprintln!("Proof added to your store");
        self.staging()?.wipe()?;
        Ok(())
    }

    pub fn status(&mut self) -> Result<()> {
        let staging = self.staging()?;
        for (k, v) in staging.entries.iter() {
            println!("{}", k.display());
        }

        Ok(())
    }

    pub fn add(&mut self, file_paths: Vec<PathBuf>) -> Result<()> {
        let mut staging = self.staging()?;
        for path in file_paths {
            staging.insert(&path);
        }
        staging.save()?;

        Ok(())
    }

    pub fn remove(&mut self, file_paths: Vec<PathBuf>) -> Result<()> {
        let mut staging = self.staging()?;
        for path in file_paths {
            staging.remove(&path);
        }
        staging.save()?;

        Ok(())
    }
}
