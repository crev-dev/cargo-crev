use crate::ProofStore;
use crate::{
    id::{self, LockedId},
    prelude::*,
    proofdb::TrustSet,
    util::{self, APP_INFO},
};
use app_dirs::{app_root, AppDataType};
use crev_common;
use crev_data::{id::OwnId, proof, proof::trust::TrustLevel, Id, PubId, Url};
use default::default;
use failure::ResultExt;
use git2;
use insideout::InsideOut;
use resiter_dpc_tmp::*;
use serde_yaml;
use std::cell::RefCell;
use std::{
    collections::HashSet,
    ffi::OsString,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const CURRENT_USER_CONFIG_SERIALIZATION_VERSION: i64 = -1;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserConfig {
    pub version: i64,
    #[serde(rename = "current-id")]
    pub current_id: Option<Id>,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            version: CURRENT_USER_CONFIG_SERIALIZATION_VERSION,
            current_id: None,
        }
    }
}

impl UserConfig {
    pub fn get_current_userid(&self) -> Result<&Id> {
        self.get_current_userid_opt()
            .ok_or_else(|| format_err!("Current Id not set"))
    }
    pub fn get_current_userid_opt(&self) -> Option<&Id> {
        self.current_id.as_ref()
    }
}

/// Local config stored in `~/.config/crev`
///
/// This managed IDs, local proof repository, etc.
pub struct Local {
    root_path: PathBuf,
    cache_path: PathBuf,
    cur_url: RefCell<Option<Url>>,
}

impl Local {
    #[allow(clippy::new_ret_no_self)]
    fn new() -> Result<Self> {
        let root_path = app_root(AppDataType::UserConfig, &APP_INFO)?;
        let cache_path = app_root(AppDataType::UserCache, &APP_INFO)?;
        Ok(Self {
            root_path,
            cache_path,
            cur_url: RefCell::new(None),
        })
    }

    pub fn get_root_cache_dir(&self) -> &Path {
        &self.cache_path
    }

    pub fn auto_open() -> Result<Self> {
        let repo = Self::new()?;
        fs::create_dir_all(&repo.cache_remotes_path())?;
        if !repo.root_path.exists() || !repo.user_config_path().exists() {
            bail!("User config not-initialized. Use `crev new id` to generate CrevID.");
        }

        Ok(repo)
    }

    pub fn auto_create() -> Result<Self> {
        let repo = Self::new()?;
        fs::create_dir_all(&repo.root_path)?;

        let config_path = repo.user_config_path();
        if config_path.exists() {
            bail!("User config already exists");
        }
        let config: UserConfig = default();
        repo.store_user_config(&config)?;
        Ok(repo)
    }

    pub fn auto_create_or_open() -> Result<Self> {
        let repo = Self::new()?;
        let config_path = repo.user_config_path();
        if config_path.exists() {
            Self::auto_open()
        } else {
            Self::auto_create()
        }
    }

    pub fn read_current_id(&self) -> Result<crev_data::Id> {
        Ok(self.load_user_config()?.get_current_userid()?.to_owned())
    }

    pub fn save_current_id(&self, id: &Id) -> Result<()> {
        let path = self.id_path(id);
        if !path.exists() {
            bail!("Id file not found.");
        }

        *self.cur_url.borrow_mut() = None;

        let mut config = self.load_user_config()?;
        config.current_id = Some(id.clone());
        self.store_user_config(&config)?;

        Ok(())
    }

    pub fn user_dir_path(&self) -> PathBuf {
        self.root_path.clone()
    }

    pub fn user_ids_path(&self) -> PathBuf {
        self.user_dir_path().join("ids")
    }

    pub fn user_proofs_path(&self) -> PathBuf {
        self.root_path.join("proofs")
    }

    fn id_path(&self, id: &Id) -> PathBuf {
        match id {
            Id::Crev { id } => self
                .user_ids_path()
                .join(format!("{}.yaml", crev_common::base64_encode(id))),
        }
    }

    pub fn list_ids(&self) -> Result<Vec<PubId>> {
        let ids_path = self.user_ids_path();
        let mut ids = vec![];
        for dir_entry in std::fs::read_dir(&ids_path)? {
            let locked_id = LockedId::read_from_yaml_file(&dir_entry?.path())?;
            ids.push(locked_id.to_pubid())
        }

        Ok(ids)
    }

    fn user_config_path(&self) -> PathBuf {
        self.user_dir_path().join("config.yaml")
    }

    pub fn cache_remotes_path(&self) -> PathBuf {
        self.cache_path.join("remotes")
    }

    pub fn load_user_config(&self) -> Result<UserConfig> {
        let path = self.user_config_path();

        let config_str = crev_common::read_file_to_string(&path)?;

        Ok(serde_yaml::from_str(&config_str)?)
    }

    pub fn store_user_config(&self, config: &UserConfig) -> Result<()> {
        let path = self.user_config_path();

        let config_str = serde_yaml::to_string(&config)?;

        Ok(util::store_str_to_file(&path, &config_str)?)
    }

    pub fn get_current_userid(&self) -> Result<Option<Id>> {
        let config = self.load_user_config()?;
        Ok(config.current_id)
    }

    pub fn read_locked_id(&self, id: &Id) -> Result<LockedId> {
        let path = self.id_path(&id);
        LockedId::read_from_yaml_file(&path)
    }

    pub fn read_current_locked_id_opt(&self) -> Result<Option<LockedId>> {
        self.get_current_userid()?
            .map(|current_id| self.read_locked_id(&current_id))
            .inside_out()
    }

    pub fn read_current_locked_id(&self) -> Result<LockedId> {
        self.read_current_locked_id_opt()?
            .ok_or_else(|| format_err!("Current Id not set"))
    }

    pub fn read_current_unlocked_id_opt(&self, passphrase: &str) -> Result<Option<OwnId>> {
        self.get_current_userid()?
            .map(|current_id| self.read_unlocked_id(&current_id, passphrase))
            .inside_out()
    }
    pub fn read_current_unlocked_id(&self, passphrase: &str) -> Result<OwnId> {
        self.read_current_unlocked_id_opt(passphrase)?
            .ok_or_else(|| format_err!("Current Id not set"))
    }

    pub fn read_unlocked_id(&self, id: &Id, passphrase: &str) -> Result<OwnId> {
        let locked = self.read_locked_id(id)?;
        locked.to_unlocked(passphrase)
    }

    pub fn save_locked_id(&self, id: &id::LockedId) -> Result<()> {
        let path = self.id_path(&id.to_pubid().id);
        fs::create_dir_all(&path.parent().expect("Not /"))?;
        id.save_to(&path)
    }

    /// Git clone or init new remote Github crev-proof repo
    pub fn clone_proof_dir_from_git(
        &self,
        git_https_url: &str,
        use_https_push: bool,
    ) -> Result<()> {
        let push_url = if use_https_push {
            git_https_url.to_string()
        } else {
            match util::git::https_to_git_url(git_https_url) {
                Some(git_url) => git_url,
                None => {
                    eprintln!("Could not deduce `ssh` push url. Call:");
                    eprintln!("cargo crev git remote set-url --push origin <url>");
                    eprintln!("manually, after id is generated.");
                    eprintln!("");
                    git_https_url.to_string()
                }
            }
        };

        let proof_dir =
            self.get_proofs_dir_path_for_url(&Url::new_git(git_https_url.to_owned()))?;

        if proof_dir.exists() {
            eprintln!(
                "Proof directory `{}` already exists. Will not clone.",
                proof_dir.display()
            );
            return Ok(());
        }

        self.ensure_proofs_root_exists()?;

        match git2::Repository::clone(git_https_url, &proof_dir) {
            Ok(repo) => {
                eprintln!("{} cloned to {}", git_https_url, proof_dir.display());
                repo.remote_set_url("origin", &push_url)?;
            }
            Err(e) => {
                bail!("Couldn't clone {}: {}", git_https_url, e);
            }
        }

        Ok(())
    }

    pub fn init_readme_using_this_repo_file(&self) -> Result<()> {
        let proof_dir = self.get_proofs_dir_path()?;
        let mut file = std::fs::File::create(proof_dir.join("README_USING_THIS_REPO.md"))?;
        file.write_all(include_bytes!("../rc/doc/README_USING_THIS_REPO.md"))?;
        file.flush()?;
        self.proof_dir_git_add_path(&PathBuf::from("README_USING_THIS_REPO.md"))?;
        Ok(())
    }

    // Get path relative to `get_proofs_dir_path` to store the `proof`
    fn get_proof_rel_store_path(&self, proof: &proof::Proof) -> PathBuf {
        crate::proof::rel_store_path(&proof.content)
    }

    fn get_cur_url(&self) -> Result<Option<Url>> {
        let url = self.cur_url.borrow().clone();
        Ok(if let Some(url) = url {
            Some(url)
        } else if let Some(locked_id) = self.read_current_locked_id_opt()? {
            *self.cur_url.borrow_mut() = Some(locked_id.url.clone());
            Some(locked_id.url)
        } else {
            None
        })
    }

    fn ensure_proofs_root_exists(&self) -> Result<()> {
        fs::create_dir_all(&self.user_proofs_path())?;
        Ok(())
    }

    pub fn get_proofs_dir_path_for_url(&self, url: &Url) -> Result<PathBuf> {
        Ok(self.user_proofs_path().join(url.digest().to_string()))
    }

    // Path where the `proofs` are stored under `git` repository
    pub fn get_proofs_dir_path_opt(&self) -> Result<Option<PathBuf>> {
        Ok(self
            .get_cur_url()?
            .map(|url| self.root_path.join("proofs").join(url.digest().to_string())))
    }

    pub fn get_proofs_dir_path(&self) -> Result<PathBuf> {
        self.get_proofs_dir_path_opt()?
            .ok_or_else(|| format_err!("Current Id not set"))
    }

    pub fn build_trust_proof(
        &self,
        id_strings: Vec<String>,
        passphrase: &str,
        trust_or_distrust: crate::TrustOrDistrust,
    ) -> Result<()> {
        if id_strings.is_empty() {
            bail!("No ids given.");
        }

        let mut db = crate::ProofDB::new();
        db.import_from_iter(self.proofs_iter()?);
        db.import_from_iter(proofs_iter_for_path(self.cache_remotes_path()));
        let mut pub_ids = vec![];

        for id_string in id_strings {
            let id = Id::crevid_from_str(&id_string)?;

            if let Some(url) = db.lookup_url(&id) {
                pub_ids.push(PubId::new(id, url.to_owned()));
            } else {
                bail!(
                    "URL not found for Id {}; Fetch proofs with `fetch url <url>` first",
                    id_string
                )
            }
        }

        let own_id = self.read_current_unlocked_id(&passphrase)?;

        let trust = own_id.create_trust_proof(
            pub_ids,
            if trust_or_distrust.is_trust() {
                TrustLevel::Medium
            } else {
                TrustLevel::Distrust
            },
        )?;

        let trust = util::edit_proof_content_iteractively(&trust.into())?;

        let proof = trust.sign_by(&own_id)?;

        self.insert(&proof)?;
        Ok(())
    }

    pub fn fetch_url(&self, url: &str) -> Result<()> {
        let _success = util::err_eprint_and_ignore(self.fetch_remote_git(url).compat());
        Ok(())
    }

    pub fn fetch_trusted(&self, trust_params: crate::TrustDistanceParams) -> Result<()> {
        let mut already_fetched = HashSet::new();
        let mut db = crate::ProofDB::new();
        db.import_from_iter(self.proofs_iter()?);
        db.import_from_iter(proofs_iter_for_path(self.cache_remotes_path()));
        let user_config = self.load_user_config()?;
        let user_id = user_config.get_current_userid()?;

        let mut something_was_fetched = true;
        while something_was_fetched {
            something_was_fetched = false;
            let trust_set =
                db.calculate_trust_set(user_config.get_current_userid()?, &trust_params);

            for id in trust_set.trusted_ids() {
                if already_fetched.contains(id) {
                    continue;
                } else {
                    already_fetched.insert(id.to_owned());
                }
                if user_id == id {
                    continue;
                } else if let Some(url) = db.lookup_url(id) {
                    let success =
                        util::err_eprint_and_ignore(self.fetch_remote_git(&url.url).compat());
                    if success {
                        something_was_fetched = true;
                        db.import_from_iter(proofs_iter_for_path(
                            self.get_remote_git_cache_path(&url.url),
                        ));
                    }
                } else {
                    eprintln!("No URL for {}", id);
                }
            }
        }
        Ok(())
    }

    fn fetch_all_ids_recursively(&self, mut already_fetched_urls: HashSet<String>) -> Result<()> {
        let mut already_fetched = HashSet::new();
        let mut db = crate::ProofDB::new();
        db.import_from_iter(self.proofs_iter()?);
        db.import_from_iter(proofs_iter_for_path(self.cache_remotes_path()));
        let user_config = self.load_user_config()?;
        let user_id = user_config.get_current_userid()?;

        let mut something_was_fetched = true;
        while something_was_fetched {
            something_was_fetched = false;

            for id in &db.all_known_ids() {
                if already_fetched.contains(id) {
                    continue;
                } else {
                    already_fetched.insert(id.to_owned());
                }
                if user_id == id {
                    continue;
                } else if let Some(url) = db.lookup_url(id) {
                    let url = url.url.to_string();

                    if already_fetched_urls.contains(&url) {
                        continue;
                    } else {
                        already_fetched_urls.insert(url.clone());
                    }

                    let success = util::err_eprint_and_ignore(self.fetch_remote_git(&url).compat());
                    if success {
                        something_was_fetched = true;
                        db.import_from_iter(proofs_iter_for_path(
                            self.get_remote_git_cache_path(&url),
                        ));
                    }
                } else {
                    eprintln!("No URL for {}", id);
                }
            }
        }
        Ok(())
    }

    pub fn get_remote_git_cache_path(&self, url: &str) -> PathBuf {
        let digest = crev_common::blake2b256sum(url.as_bytes());
        let digest = crev_data::Digest::from_vec(digest);
        self.cache_remotes_path().join(digest.to_string())
    }

    pub fn fetch_remote_git(&self, url: &str) -> Result<()> {
        let dir = self.get_remote_git_cache_path(url);

        if dir.exists() {
            eprintln!("Fetching {} to {}", url, dir.display());
            let repo = git2::Repository::open(dir)?;
            util::git::fetch_and_checkout_git_repo(&repo)?
        } else {
            eprintln!("Cloning {} to {}", url, dir.display());
            git2::Repository::clone(url, dir)?;
        }

        Ok(())
    }

    pub fn fetch_all(&self) -> Result<()> {
        let mut fetched_urls = HashSet::new();
        for entry in fs::read_dir(self.cache_remotes_path())? {
            let path = entry?.path();
            if !path.is_dir() {
                continue;
            }

            let repo = git2::Repository::open(&path);
            if repo.is_err() {
                continue;
            }

            let url = {
                || -> Result<String> {
                    let repo = repo.unwrap();
                    let remote = repo.find_remote("origin")?;
                    let url = remote
                        .url()
                        .ok_or_else(|| format_err!("origin has no url"))?;
                    Ok(url.to_string())
                }
            }();

            match url {
                Ok(url) => {
                    fetched_urls.insert(url.clone());
                    let _success =
                        util::err_eprint_and_ignore(self.fetch_remote_git(&url).compat());
                }
                Err(e) => {
                    eprintln!("ERR: {} {}", path.display(), e);
                }
            }
        }

        self.fetch_all_ids_recursively(fetched_urls)?;

        Ok(())
    }

    pub fn run_git(&self, args: Vec<OsString>) -> Result<std::process::ExitStatus> {
        let orig_dir = std::env::current_dir()?;
        std::env::set_current_dir(self.get_proofs_dir_path()?)?;

        use std::process::Command;

        let status = Command::new("git")
            .args(args)
            .status()
            .expect("failed to execute git");

        std::env::set_current_dir(orig_dir)?;

        Ok(status)
    }

    pub fn edit_readme(&self) -> Result<()> {
        util::edit_file(&self.get_proofs_dir_path()?.join("README.md"))?;
        self.proof_dir_git_add_path(&PathBuf::from("README.md"))?;
        Ok(())
    }

    pub fn load_db(
        &self,
        params: &crate::TrustDistanceParams,
    ) -> Result<(crate::ProofDB, TrustSet)> {
        let user_config = self.load_user_config()?;
        let mut db = crate::ProofDB::new();
        db.import_from_iter(self.proofs_iter()?);
        db.import_from_iter(proofs_iter_for_path(self.cache_remotes_path()));

        let trust_set = if let Some(id) = user_config.get_current_userid_opt() {
            db.calculate_trust_set(id, &params)
        } else {
            TrustSet::default()
        };
        Ok((db, trust_set))
    }

    pub fn proof_dir_git_add_path(&self, rel_path: &Path) -> Result<()> {
        let proof_dir = self.get_proofs_dir_path()?;
        let repo = git2::Repository::init(&proof_dir)?;
        let mut index = repo.index()?;

        index.add_path(rel_path)?;
        index.write()?;
        Ok(())
    }

    pub fn show_current_id(&self) -> Result<()> {
        if let Some(id) = self.read_current_locked_id_opt()? {
            let id = id.to_pubid();
            println!("{} {}", id.id, id.url.url);
        }
        Ok(())
    }

    pub fn generate_id(
        &self,
        url: Option<String>,
        github_username: Option<String>,
        use_https_push: bool,
    ) -> Result<()> {
        let url = match (url, github_username) {
            (Some(url), None) => url,
            (None, Some(username)) => format!("https://github.com/{}/crev-proofs", username),
            (Some(_), Some(_)) => bail!("Can't provide both username and url"),
            (None, None) => bail!("Must provide github username or url"),
        };

        if !url.starts_with("https://") {
            bail!("URL must start with 'https://");
        }

        self.clone_proof_dir_from_git(&url, use_https_push)?;

        let id = crev_data::id::OwnId::generate(crev_data::Url::new_git(url.clone()));
        eprintln!("CrevID will be protected by a passphrase.");
        eprintln!("There's no way to recover your CrevID if you forget your passphrase.");
        let passphrase = crev_common::read_new_passphrase()?;
        let locked = id::LockedId::from_own_id(&id, &passphrase)?;

        self.save_locked_id(&locked)?;
        self.save_current_id(id.as_ref())?;

        eprintln!("");
        eprintln!("Your CrevID was created and will be printed below in an encrypted form.");
        eprintln!("Make sure to back it up on another device, to prevent loosing it.");

        eprintln!("");
        println!("{}", locked);

        self.init_readme_using_this_repo_file()?;

        Ok(())
    }

    pub fn switch_id(&self, id_str: &str) -> Result<()> {
        let id: Id = Id::crevid_from_str(id_str)?;
        self.save_current_id(&id)?;

        Ok(())
    }

    pub fn list_own_ids(&self) -> Result<()> {
        for id in self.list_ids()? {
            println!("{} {}", id.id, id.url.url);
        }
        Ok(())
    }
}

impl ProofStore for Local {
    fn insert(&self, proof: &proof::Proof) -> Result<()> {
        let rel_store_path = self.get_proof_rel_store_path(proof);
        let path = self.get_proofs_dir_path()?.join(&rel_store_path);

        fs::create_dir_all(path.parent().expect("Not a root dir"))?;
        let mut file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .write(true)
            .open(path)?;

        file.write_all(proof.to_string().as_bytes())?;
        file.write_all(b"\n")?;
        file.flush()?;
        drop(file);

        self.proof_dir_git_add_path(&rel_store_path)?;

        Ok(())
    }

    fn proofs_iter(&self) -> Result<Box<Iterator<Item = proof::Proof>>> {
        Ok(Box::new(
            self.get_proofs_dir_path_opt()?
                .into_iter()
                .flat_map(proofs_iter_for_path),
        ))
    }
}

fn proofs_iter_for_path(path: PathBuf) -> impl Iterator<Item = proof::Proof> {
    use std::ffi::OsStr;
    let file_iter = walkdir::WalkDir::new(path)
        .into_iter()
        .map_err(|e| format_err!("Error iterating local ProofStore: {:?}", e))
        .filter_map_ok(|entry| {
            let path = entry.path();
            if !path.is_file() {
                return None;
            }

            let osext_match: &OsStr = "crev".as_ref();
            match path.extension() {
                Some(osext) if osext == osext_match => Some(path.to_owned()),
                _ => None,
            }
        });

    let proofs_iter = file_iter
        .and_then_ok(|path| Ok(proof::Proof::parse_from(&path)?))
        .flatten_ok()
        .and_then_ok(|proof| {
            proof.verify()?;
            Ok(proof)
        })
        .on_err(|e| {
            eprintln!("Failed processing a proof: {}", e);
        });

    proofs_iter.oks()
}
