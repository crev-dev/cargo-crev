use chrono;
use id;
use proof;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use util;
use Result;

pub mod staging;

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

    fn get_proofs_file(&self) -> PathBuf {
        let year_month = chrono::Utc::now().format("%Y-%m").to_string();

        self.dot_crev_path()
            .join("proofs")
            .join(year_month)
            .with_extension("crev")
    }

    fn write_out_proof_to(&mut self, proof: proof::ReviewProof, file_path: &Path) -> Result<()> {
        fs::create_dir_all(file_path.parent().expect("Not a root dir"))?;
        let mut file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .write(true)
            .open(file_path)?;

        file.write_all(proof.to_string().as_bytes())?;
        file.flush()?;

        Ok(())
    }

    pub fn commit(&mut self) -> Result<()> {
        if self.staging()?.is_empty() {
            bail!("No reviews to commit. Use `add` first.");
        }
        let passphrase = util::read_passphrase()?;
        let id = id::OwnId::auto_open(&passphrase)?;
        let files = self.staging()?.to_review_files();

        let review = proof::ReviewBuilder::default()
            .from(id.name().into())
            .from_id(id.pub_key_as_base64())
            .from_id_type(id.type_as_string())
            .revision(Some("TODO".into()))
            .revision_type("git".into())
            .project_urls(vec![])
            .comment(Some("".into()))
            .thoroughness(proof::Level::Low)
            .understanding(proof::Level::Low)
            .trust(proof::Level::Low)
            .files(files)
            .build()
            .map_err(|e| format_err!("{}", e))?;

        let redacted = util::edit_review_iteractively(review)?;

        let proof = redacted.sign(&id)?;

        let file_path = self.get_proofs_file();
        self.write_out_proof_to(proof, &file_path)?;
        println!("Proof written to: {}", file_path.display());
        Ok(())
    }
}
