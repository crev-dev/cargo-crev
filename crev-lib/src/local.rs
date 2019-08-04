use crate::activity::ReviewActivity;
use crate::{
    id::{self, LockedId, PassphraseFn},
    prelude::*,
    util, ProofDB, ProofStore,
};
use crev_common::{
    self,
    sanitize_name,
    serde::{as_base64, from_base64},
};
use crev_data::{
    id::OwnId,
    proof::{self, trust::TrustLevel},
    Id, PubId, Url,
};
use default::default;
use directories::ProjectDirs;
use failure::{bail, format_err, ResultExt};
use git2;
use insideout::InsideOut;
use resiter::*;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::{
    cell::RefCell,
    collections::HashSet,
    ffi::OsString,
    fs,
    io::{BufRead, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

const CURRENT_USER_CONFIG_SERIALIZATION_VERSION: i64 = -1;

fn generete_salt() -> Vec<u8> {
    crev_common::rand::random_vec(32)
}

/// Backfill the host salt
///
/// For people that have configs generated when
/// `host_salt` was not a thing - generate some
/// form of stable id
///
/// TODO: at some point this should no longer be neccessary
fn backfill_salt() -> Vec<u8> {
    crev_common::blake2b256sum(b"BACKFILLED_SUM")
}

fn is_none_or_empty(s: &Option<String>) -> bool {
    if let Some(s) = s {
        s.is_empty()
    } else {
        true
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserConfig {
    pub version: i64,
    #[serde(rename = "current-id")]
    pub current_id: Option<Id>,
    #[serde(
        rename = "host-salt",
        serialize_with = "as_base64",
        deserialize_with = "from_base64",
        default = "backfill_salt"
    )]
    host_salt: Vec<u8>,

    #[serde(
        rename = "open-cmd",
        skip_serializing_if = "is_none_or_empty",
        default = "Option::default"
    )]
    pub open_cmd: Option<String>,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            version: CURRENT_USER_CONFIG_SERIALIZATION_VERSION,
            current_id: None,
            host_salt: generete_salt(),
            open_cmd: None,
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

    pub fn edit_iteractively(&self) -> Result<Self> {
        let mut text = serde_yaml::to_string(self)?;
        loop {
            text = util::edit_text_iteractively(&text)?;
            match serde_yaml::from_str(&text) {
                Err(e) => {
                    eprintln!("There was an error parsing content: {}", e);
                    crev_common::try_again_or_cancel()?;
                }
                Ok(s) => return Ok(s),
            }
        }
    }
}

/// Local config stored in `~/.config/crev`
///
/// This managed IDs, local proof repository, etc.
pub struct Local {
    root_path: PathBuf,
    cache_path: PathBuf,
    cur_url: RefCell<Option<Url>>,
    user_config: RefCell<Option<UserConfig>>,
}

impl Local {
    #[allow(clippy::new_ret_no_self)]
    fn new() -> Result<Self> {
        let proj_dir = ProjectDirs::from("", "", "crev")
            .expect("no valid home directory path could be retrieved from the operating system");
        let root_path = proj_dir.config_dir().into();
        let cache_path = proj_dir.cache_dir().into();
        Ok(Self {
            root_path,
            cache_path,
            cur_url: RefCell::new(None),
            user_config: RefCell::new(None),
        })
    }

    pub fn get_root_cache_dir(&self) -> &Path {
        &self.cache_path
    }

    pub fn auto_open() -> Result<Self> {
        let repo = Self::new()?;
        fs::create_dir_all(&repo.cache_remotes_path())?;
        if !repo.root_path.exists() || !repo.user_config_path().exists() {
            bail!("User config not-initialized. Use `crev id new` to generate CrevID.");
        }

        *repo.user_config.borrow_mut() = Some(repo.load_user_config()?);
        Ok(repo)
    }

    pub fn auto_create() -> Result<Self> {
        let repo = Self::new()?;
        fs::create_dir_all(&repo.root_path)?;
        fs::create_dir_all(&repo.cache_remotes_path())?;

        let config_path = repo.user_config_path();
        if config_path.exists() {
            bail!("User config already exists");
        }
        let config: UserConfig = default();
        repo.store_user_config(&config)?;
        *repo.user_config.borrow_mut() = Some(config);
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

    pub fn read_current_id_opt(&self) -> Result<Option<crev_data::Id>> {
        Ok(self.load_user_config()?.get_current_userid_opt().cloned())
    }

    /// Calculate `for_id` that is used in a lot of operations
    ///
    /// * if `id_str` is given - convert to Id
    /// * otherwise return current id
    pub fn get_for_id_from_str_opt(&self, id_str: Option<&str>) -> Result<Option<Id>> {
        id_str
            .map(crev_data::id::Id::crevid_from_str)
            .or_else(|| self.read_current_id_opt().inside_out())
            .inside_out()
    }

    pub fn get_for_id_from_str(&self, id_str: Option<&str>) -> Result<Id> {
        self.get_for_id_from_str_opt(id_str)?
            .ok_or_else(|| format_err!("Id not specified and current id not set"))
    }

    pub fn save_current_id(&self, id: &Id) -> Result<()> {
        let path = self.id_path(id);
        if !path.exists() {
            bail!("Id file not found.");
        }

        *self.cur_url.borrow_mut() = None;

        let mut config = self.load_user_config()?;
        config.current_id = Some(id.clone());
        // Change the old, backfilled `host_salt` the first time
        // the id is being switched
        if config.host_salt == backfill_salt() {
            config.host_salt = generete_salt();
        }
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

    fn cache_activity_path(&self) -> PathBuf {
        self.cache_path.join("activity")
    }

    fn cache_review_activity_path(
        &self,
        source: &str,
        name: &str,
        version: &semver::Version,
    ) -> PathBuf {
        self.cache_activity_path()
            .join("review")
            .join(sanitize_name(source))
            .join(sanitize_name(name))
            .join(sanitize_name(&version.to_string()))
            .with_extension("yaml")
    }

    pub fn record_review_activity(
        &self,
        source: &str,
        name: &str,
        version: &semver::Version,
        activity: &ReviewActivity,
    ) -> Result<()> {
        let path = self.cache_review_activity_path(source, name, version);

        crev_common::save_to_yaml_file(&path, activity)?;

        Ok(())
    }

    pub fn read_review_activity(
        &self,
        source: &str,
        name: &str,
        version: &semver::Version,
    ) -> Result<Option<ReviewActivity>> {
        let path = self.cache_review_activity_path(source, name, version);

        if path.exists() {
            Ok(Some(crev_common::read_from_yaml_file(&path)?))
        } else {
            Ok(None)
        }
    }

    pub fn load_user_config(&self) -> Result<UserConfig> {
        let path = self.user_config_path();

        let config_str = crev_common::read_file_to_string(&path)?;

        Ok(serde_yaml::from_str(&config_str)?)
    }

    pub fn store_user_config(&self, config: &UserConfig) -> Result<()> {
        let path = self.user_config_path();

        let config_str = serde_yaml::to_string(&config)?;

        util::store_str_to_file(&path, &config_str)?;

        *self.user_config.borrow_mut() = Some(config.clone());
        Ok(())
    }

    pub fn get_current_userid(&self) -> Result<Id> {
        self.get_current_userid_opt()?
            .ok_or_else(|| format_err!("Current Id not set"))
    }

    pub fn get_current_userid_opt(&self) -> Result<Option<Id>> {
        let config = self.load_user_config()?;
        Ok(config.current_id)
    }

    pub fn read_locked_id(&self, id: &Id) -> Result<LockedId> {
        let path = self.id_path(&id);
        LockedId::read_from_yaml_file(&path)
    }

    pub fn read_current_locked_id_opt(&self) -> Result<Option<LockedId>> {
        self.get_current_userid_opt()?
            .map(|current_id| self.read_locked_id(&current_id))
            .inside_out()
    }

    pub fn read_current_locked_id(&self) -> Result<LockedId> {
        self.read_current_locked_id_opt()?
            .ok_or_else(|| format_err!("Current Id not set"))
    }

    pub fn read_current_unlocked_id_opt(
        &self,
        passphrase_callback: PassphraseFn<'_>,
    ) -> Result<Option<OwnId>> {
        self.get_current_userid_opt()?
            .map(|current_id| self.read_unlocked_id(&current_id, passphrase_callback))
            .inside_out()
    }

    pub fn read_current_unlocked_id(&self, passphrase_callback: PassphraseFn<'_>) -> Result<OwnId> {
        self.read_current_unlocked_id_opt(passphrase_callback)?
            .ok_or_else(|| format_err!("Current Id not set"))
    }

    pub fn read_unlocked_id(&self, id: &Id, passphrase_callback: PassphraseFn<'_>) -> Result<OwnId> {
        let locked = self.read_locked_id(id)?;
        let mut i = 0;
        loop {
            let passphrase = passphrase_callback()?;
            match locked.to_unlocked(&passphrase) {
                Ok(o) => return Ok(o),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    if i == 5 {
                        return Err(e);
                    }
                }
            }
            i += 1;
        }
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
        const README_MARKER_V0: &str = "CREV_README_MARKER_V0";

        let proof_dir = self.get_proofs_dir_path()?;
        let path = proof_dir.join("README.md");
        if path.exists() {
            if let Some(line) = std::io::BufReader::new(std::fs::File::open(&path)?)
                .lines()
                .find(|line| {
                    if let Ok(ref line) = line {
                        line.trim() != ""
                    } else {
                        true
                    }
                })
            {
                if line?.contains(README_MARKER_V0) {
                    return Ok(());
                }
            }
        }

        std::fs::write(
            proof_dir.join("README.md"),
            &include_bytes!("../rc/doc/README.md")[..],
        )?;
        self.proof_dir_git_add_path(Path::new("README.md"))?;
        Ok(())
    }

    // Get path relative to `get_proofs_dir_path` to store the `proof`
    fn get_proof_rel_store_path(&self, proof: &proof::Proof, host_salt: &[u8]) -> PathBuf {
        crate::proof::rel_store_path(&proof.content, host_salt)
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
        from_id: &PubId,
        id_strings: Vec<String>,
        trust_or_distrust: crate::TrustOrDistrust,
    ) -> Result<proof::Content> {
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

        let trust = from_id.create_trust_proof(
            &pub_ids,
            if trust_or_distrust.is_trust() {
                TrustLevel::Medium
            } else {
                TrustLevel::Distrust
            },
        )?;

        // TODO: Look up previous trust proof?
        Ok(util::edit_proof_content_iteractively(
            &trust.into(),
            None,
            None,
        )?)
    }

    pub fn fetch_url(&self, url: &str) -> Result<()> {
        let mut db = self.load_db()?;
        if let Some(dir) = self.fetch_proof_repo_import_and_print_counts(url, &mut db) {
            let mut db = ProofDB::new();
            db.import_from_iter(proofs_iter_for_path(dir));
            eprintln!("Found proofs from:");
            for (id, count) in db.all_author_ids() {
                println!("{:>8} {}", count, id);
            }
        }
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
                } else if let Some(url) = db.lookup_url(id).cloned() {
                    self.fetch_proof_repo_import_and_print_counts(&url.url, &mut db);
                } else {
                    eprintln!("No URL for {}", id);
                }
            }
        }
        Ok(())
    }

    fn fetch_all_ids_recursively(
        &self,
        mut already_fetched_urls: HashSet<String>,
        db: &mut ProofDB,
    ) -> Result<()> {
        let mut already_fetched = HashSet::new();
        let user_config = self.load_user_config()?;
        let user_id = user_config.get_current_userid_opt();

        let mut something_was_fetched = true;
        while something_was_fetched {
            something_was_fetched = false;

            for id in &db.all_known_ids() {
                if already_fetched.contains(id) {
                    continue;
                } else {
                    already_fetched.insert(id.to_owned());
                }
                if user_id == Some(id) {
                    continue;
                } else if let Some(url) = db.lookup_url(id).cloned() {
                    let url = url.url;

                    if already_fetched_urls.contains(&url) {
                        continue;
                    } else {
                        already_fetched_urls.insert(url.clone());
                    }
                    self.fetch_proof_repo_import_and_print_counts(&url, db);
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

    /// Fetch a git proof repository
    ///
    /// Returns url where it was cloned/fetched
    pub fn fetch_remote_git(&self, url: &str) -> Result<PathBuf> {
        let dir = self.get_remote_git_cache_path(url);

        if dir.exists() {
            let repo = git2::Repository::open(&dir)?;
            util::git::fetch_and_checkout_git_repo(&repo)?
        } else {
            git2::Repository::clone(url, &dir)?;
        }

        Ok(dir)
    }

    pub fn fetch_proof_repo_import_and_print_counts(
        &self,
        url: &str,
        db: &mut ProofDB,
    ) -> Option<PathBuf> {
        let prev_pkg_review_count = db.unique_package_review_proof_count();
        let prev_trust_count = db.unique_trust_proof_count();

        eprint!("Fetching {}... ", url);
        match self.fetch_remote_git(url) {
            Ok(dir) => {
                db.import_from_iter(proofs_iter_for_path(dir.clone()));

                eprint!("OK");

                let new_pkg_review_count =
                    db.unique_package_review_proof_count() - prev_pkg_review_count;
                let new_trust_count = db.unique_trust_proof_count() - prev_trust_count;

                if new_trust_count > 0 {
                    eprint!("; {} new trust proofs", new_pkg_review_count);
                }
                if new_pkg_review_count > 0 {
                    eprint!("; {} new package reviews", new_pkg_review_count);
                }
                eprintln!("");
                Some(dir)
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                None
            }
        }
    }

    pub fn fetch_all(&self) -> Result<()> {
        let mut fetched_urls = HashSet::new();
        let mut db = self.load_db()?;

        // Temporarily hardcode `dpc`'s proof-repo url
        let dpc_url = "https://github.com/dpc/crev-proofs";
        self.fetch_proof_repo_import_and_print_counts(dpc_url, &mut db);
        fetched_urls.insert(dpc_url.to_owned());

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
                    if !fetched_urls.contains(&url) {
                        fetched_urls.insert(url.clone());
                        self.fetch_proof_repo_import_and_print_counts(&url, &mut db);
                    }
                }
                Err(e) => {
                    eprintln!("ERR: {} {}", path.display(), e);
                }
            }
        }

        self.fetch_all_ids_recursively(fetched_urls, &mut db)?;

        Ok(())
    }

    pub fn run_git(&self, args: Vec<OsString>) -> Result<std::process::ExitStatus> {
        let orig_dir = std::env::current_dir()?;
        let proof_dir_path = self.get_proofs_dir_path()?;
        if !proof_dir_path.exists() {
            let id = self.read_current_locked_id()?;
            self.clone_proof_dir_from_git(&id.url.url, false)?;
        }

        std::env::set_current_dir(proof_dir_path)
            .with_context(|_| "Trying to change dir to the current local proof repo")?;

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

    pub fn edit_user_config(&self) -> Result<()> {
        let config = self.load_user_config()?;
        let config = config.edit_iteractively()?;
        self.store_user_config(&config)?;
        Ok(())
    }

    pub fn store_config_open_cmd(&self, cmd: String) -> Result<()> {
        let mut config = self.load_user_config()?;
        config.open_cmd = Some(cmd);
        self.store_user_config(&config)?;
        Ok(())
    }

    /// Create a new proofdb, and populate it with local repo
    /// and cache content.
    pub fn load_db(&self) -> Result<crate::ProofDB> {
        let mut db = crate::ProofDB::new();
        db.import_from_iter(self.proofs_iter()?);
        db.import_from_iter(proofs_iter_for_path(self.cache_remotes_path()));

        Ok(db)
    }

    pub fn proof_dir_git_add_path(&self, rel_path: &Path) -> Result<()> {
        let proof_dir = self.get_proofs_dir_path()?;
        let repo = git2::Repository::open(&proof_dir)?;
        let mut index = repo.index()?;

        index.add_path(rel_path)?;
        index.write()?;
        Ok(())
    }

    pub fn proof_dir_commit(&self, commit_msg: &str) -> Result<()> {
        let proof_dir = self.get_proofs_dir_path()?;
        let repo = git2::Repository::open(&proof_dir)?;
        let mut index = repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let head = repo.head()?.peel_to_commit()?;

        let signature = repo.signature()?;

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            commit_msg,
            &tree,
            &[&head],
        )?;

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

    pub fn show_own_ids(&self) -> Result<()> {
        let current = self.read_current_locked_id_opt()?.map(|id| id.to_pubid());
        for id in self.list_ids()? {
            let is_current = current.as_ref().map_or(false, |c| {c.id == id.id});
            println!("{} {}{}", id.id, id.url.url, if is_current {" (current)"} else {""});
        }
        Ok(())
    }

    pub fn export_locked_id(&self, id_str: Option<String>) -> Result<String> {
        let id = if let Some(id_str) = id_str {
            let id = Id::crevid_from_str(&id_str)?;
            self.read_locked_id(&id)?
        } else {
            self.read_current_locked_id()?
        };

        Ok(id.to_string())
    }

    pub fn import_locked_id(&self, locked_id_serialized: &str) -> Result<PubId> {
        let id = LockedId::from_str(locked_id_serialized)?;
        self.save_locked_id(&id)?;
        Ok(id.to_pubid())
    }
}

impl ProofStore for Local {
    fn insert(&self, proof: &proof::Proof) -> Result<()> {
        let rel_store_path = self.get_proof_rel_store_path(
            proof,
            &self
                .user_config
                .borrow()
                .as_ref()
                .expect("User config loaded")
                .host_salt,
        );
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

    fn proofs_iter(&self) -> Result<Box<dyn Iterator<Item = proof::Proof>>> {
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
