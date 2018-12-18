use crate::ProofStore;
use crate::{
    id::{self, LockedId},
    trustdb,
    util::{self, APP_INFO},
    Result,
};
use app_dirs::{app_root, AppDataType};
use crev_common;
use crev_data::{id::OwnId, proof, proof::trust::TrustLevel, Id, PubId, Url};
use default::default;
use failure::ResultExt;
use git2;
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
        self.current_id
            .as_ref()
            .ok_or_else(|| format_err!("Current Id not set"))
    }
}

fn https_to_git_url(http_url: &str) -> Option<String> {
    let mut split: Vec<_> = http_url.split('/').collect();

    while let Some(&"") = split.last() {
        split.pop();
    }
    if split.len() != 5 {
        return None;
    }
    if split[0] != "https:" && split[0] != "http:" {
        return None;
    }
    let host = split[2];
    let user = split[3];
    let repo = split[4];
    let suffix = if repo.ends_with(".git") { "" } else { ".git" };

    Some(format!("git@{}:{}/{}{}", host, user, repo, suffix))
}

#[test]
fn https_to_git_url_test() {
    assert_eq!(
        https_to_git_url("https://github.com/dpc/trust"),
        Some("git@github.com:dpc/trust.git".into())
    );
    assert_eq!(
        https_to_git_url("https://gitlab.com/hackeraudit/web.git"),
        Some("git@gitlab.com:hackeraudit/web.git".into())
    );
    assert_eq!(
        https_to_git_url("https://gitlab.com/hackeraudit/web.git/"),
        Some("git@gitlab.com:hackeraudit/web.git".into())
    );
    assert_eq!(
        https_to_git_url("https://gitlab.com/hackeraudit/web.git/////////"),
        Some("git@gitlab.com:hackeraudit/web.git".into())
    );
}

/// Local config stored in `~/.config/crev`
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

    fn id_path(&self, id: &Id) -> PathBuf {
        match id {
            Id::Crev { id } => self
                .user_ids_path()
                .join(format!("{}.yaml", crev_common::base64_encode(id))),
        }
    }

    pub fn list_ids(&self) -> Result<Vec<Id>> {
        let ids_path = self.user_ids_path();
        let mut ids = vec![];
        for dir_entry in std::fs::read_dir(&ids_path)? {
            let locked_id = LockedId::read_from_yaml_file(&dir_entry?.path())?;
            ids.push(locked_id.to_pubid().id)
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

        let config_str = util::read_file_to_string(&path)?;

        Ok(serde_yaml::from_str(&config_str)?)
    }

    pub fn store_user_config(&self, config: &UserConfig) -> Result<()> {
        let path = self.user_config_path();

        let config_str = serde_yaml::to_string(&config)?;

        util::store_str_to_file(&path, &config_str)
    }

    pub fn get_current_userid(&self) -> Result<Id> {
        let config = self.load_user_config()?;
        Ok(config
            .current_id
            .ok_or_else(|| format_err!("Current id not set"))?)
    }

    pub fn read_locked_id(&self, id: &Id) -> Result<LockedId> {
        let path = self.id_path(&id);
        LockedId::read_from_yaml_file(&path)
    }

    pub fn read_current_locked_id(&self) -> Result<LockedId> {
        let current_id = self.get_current_userid()?;
        self.read_locked_id(&current_id)
    }

    pub fn read_current_unlocked_id(&self, passphrase: &str) -> Result<OwnId> {
        let current_id = self.get_current_userid()?;
        self.read_unlocked_id(&current_id, passphrase)
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

    pub fn git_init_proof_dir(&self, git_https_url: &str) -> Result<()> {
        let git_url = https_to_git_url(git_https_url);

        let proof_dir =
            self.get_proofs_dir_path_for_url(&Url::new_git(git_https_url.to_owned()))?;
        match git2::Repository::clone(git_https_url, &proof_dir) {
            Ok(repo) => {
                eprintln!("{} cloned to {}", git_https_url, proof_dir.display());
                if let Some(git_url) = git_url {
                    repo.remote_set_url("origin", &git_url)?;
                }
            }
            Err(e) => {
                eprintln!("Couldn't clone {}: {}", git_https_url, e);
                fs::create_dir_all(&proof_dir)?;

                let repo = git2::Repository::init(&proof_dir)?;
                eprintln!(
                    "Empty git repository initialized in {}",
                    proof_dir.display()
                );

                eprintln!("");
                if let Some(git_url) = git_url {
                    eprintln!("Setting push url to: {}", git_url);
                    repo.remote_set_url("origin", &git_url)?;
                } else {
                    eprintln!("Could not deduce ssh push url.");
                    eprintln!("Setting push url to: {}", git_https_url);
                    repo.remote_set_url("origin", &git_https_url)?;
                }

                eprintln!("Use:");
                eprintln!("cargo crev git remote set-url --push origin <url>");
                eprintln!("to change it at later time");
                eprintln!("");
            }
        }

        Ok(())
    }

    // Get path relative to `get_proofs_dir_path` to store the `proof`
    fn get_proof_rel_store_path(&self, proof: &proof::Proof) -> PathBuf {
        crate::proof::rel_store_path(&proof.content)
    }

    fn get_cur_url(&self) -> Result<Url> {
        let url = self.cur_url.borrow().clone();
        Ok(if let Some(url) = url {
            url
        } else {
            let locked_id = self.read_current_locked_id()?;
            *self.cur_url.borrow_mut() = Some(locked_id.url.clone());
            locked_id.url
        })
    }

    pub fn get_proofs_dir_path_for_url(&self, url: &Url) -> Result<PathBuf> {
        Ok(self.root_path.join("proofs").join(url.digest().to_string()))
    }

    // Path where the `proofs` are stored under `git` repository
    pub fn get_proofs_dir_path(&self) -> Result<PathBuf> {
        Ok(self
            .root_path
            .join("proofs")
            .join(self.get_cur_url()?.digest().to_string()))
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

        let mut trustdb = trustdb::TrustDB::new();
        trustdb.import_from_iter(self.proofs_iter()?);
        trustdb.import_from_iter(proofs_iter_for_path(self.cache_remotes_path()));
        let mut pub_ids = vec![];

        for id_string in id_strings {
            let id = Id::crevid_from_str(&id_string)?;

            if let Some(url) = trustdb.lookup_url(&id) {
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

        let trust = util::edit_proof_content_iteractively(&trust.into(), proof::ProofType::Trust)?;

        let proof = trust.sign_by(&own_id)?;

        self.insert(&proof)?;
        Ok(())
    }

    pub fn fetch_url(&self, url: &str) -> Result<()> {
        let _success = util::err_eprint_and_ignore(self.fetch_remote_git(url).compat());
        Ok(())
    }

    pub fn fetch_trusted(&self, trust_params: trustdb::TrustDistanceParams) -> Result<()> {
        let mut already_fetched = HashSet::new();
        let mut db = trustdb::TrustDB::new();
        db.import_from_iter(self.proofs_iter()?);
        db.import_from_iter(proofs_iter_for_path(self.cache_remotes_path()));
        let user_config = self.load_user_config()?;
        let user_id = user_config.get_current_userid()?;

        let mut something_was_fetched = true;
        while something_was_fetched {
            something_was_fetched = false;
            let trust_set =
                db.calculate_trust_set(user_config.get_current_userid()?, &trust_params);

            for id in &trust_set {
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
            repo.find_remote("origin")?.fetch(&["master"], None, None)?;
            repo.set_head("FETCH_HEAD")?;
            let mut opts = git2::build::CheckoutBuilder::new();
            opts.force();
            repo.checkout_head(Some(&mut opts))?;
        } else {
            eprintln!("Cloning {} to {}", url, dir.display());
            git2::Repository::clone(url, dir)?;
        }

        Ok(())
    }

    pub fn fetch_all(&self) -> Result<()> {
        eprintln!(
            "Fetching all crev-proofs repositories ({})",
            self.cache_remotes_path().display()
        );

        for entry in fs::read_dir(self.cache_remotes_path())? {
            let path = entry?.path();
            if !path.is_dir() {
                continue;
            }

            let repo = git2::Repository::open(&path);
            if repo.is_err() {
                continue;
            }

            let repo = repo?;
            repo.find_remote("origin")?.fetch(&["master"], None, None)?;
            repo.set_head("FETCH_HEAD")?;
            let mut opts = git2::build::CheckoutBuilder::new();
            opts.force();
            repo.checkout_head(Some(&mut opts))?;

            let remote = repo.find_remote("origin")?;
            eprintln!("fetch\t{}", remote.url().unwrap());
        }

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
        params: &trustdb::TrustDistanceParams,
    ) -> Result<(trustdb::TrustDB, HashSet<Id>)> {
        let user_config = self.load_user_config()?;
        let mut db = trustdb::TrustDB::new();
        db.import_from_iter(self.proofs_iter()?);
        db.import_from_iter(proofs_iter_for_path(self.cache_remotes_path()));
        let trusted_set = db.calculate_trust_set(user_config.get_current_userid()?, &params);

        Ok((db, trusted_set))
    }

    pub fn proof_dir_git_add_path(&self, rel_path: &Path) -> Result<()> {
        let proof_dir = self.get_proofs_dir_path()?;
        let repo = git2::Repository::init(&proof_dir)?;
        let mut index = repo.index()?;

        index.add_path(rel_path)?;
        index.write()?;
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
        Ok(proofs_iter_for_path(self.get_proofs_dir_path()?))
    }
}

fn proofs_iter_for_path(path: PathBuf) -> Box<Iterator<Item = proof::Proof>> {
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

    Box::new(proofs_iter.oks())
}
