use crate::ProofStore;
use crate::{
    id::{self, LockedId},
    trustdb,
    util::{self, APP_INFO},
    Result,
};
use app_dirs::{app_root, AppDataType};
use base64;
use crev_common;
use crev_data::{id::OwnId, level, proof, Id, PubId};
use default::default;
use failure::ResultExt;
use git2;
use resiter::*;
use serde_yaml;
use std::{collections::HashSet, ffi::OsString, fs, io::Write, path::PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserConfig {
    pub version: i64,
    #[serde(rename = "current-id")]
    pub current_id: Option<Id>,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            version: crev_data::current_version(),
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
    let split: Vec<_> = http_url.split('/').collect();

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
}

/// Local config stored in `~/.config/crev`
pub struct Local {
    root_path: PathBuf,
    cache_path: PathBuf,
}

impl Local {
    fn new() -> Result<Self> {
        let root_path = app_root(AppDataType::UserConfig, &APP_INFO)?;
        let cache_path = app_root(AppDataType::UserCache, &APP_INFO)?;
        Ok(Self {
            root_path,
            cache_path,
        })
    }
    pub fn auto_open() -> Result<Self> {
        let repo = Self::new()?;
        fs::create_dir_all(&repo.cache_remotes_path())?;
        if !repo.root_path.exists() {
            bail!("User config not-initialized. Use `crev id gen` to generate CrevID.");
        }

        if !repo.user_config_path().exists() {
            bail!("User config not-initialized. Use `crev id gen` to generate CrevID.");
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

    pub fn read_current_id(&self) -> Result<crev_data::Id> {
        Ok(self.load_user_config()?.get_current_userid()?.to_owned())
    }

    pub fn save_current_id(&self, id: &OwnId) -> Result<()> {
        let mut config = self.load_user_config()?;
        config.current_id = Some(id.id.id.clone());
        self.store_user_config(&config)?;

        Ok(())
    }

    pub fn user_dir_path(&self) -> PathBuf {
        self.root_path.clone()
    }

    fn id_path(&self, id: &Id) -> PathBuf {
        match id {
            Id::Crev { id } => self.user_dir_path().join("ids").join(format!(
                "{}.yaml",
                base64::encode_config(id, base64::URL_SAFE)
            )),
        }
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

    pub fn read_locked_id(&self) -> Result<LockedId> {
        let config = self.load_user_config()?;
        let current_id = &config
            .current_id
            .ok_or_else(|| format_err!("Current id not set"))?;
        let path = self.id_path(current_id);
        LockedId::read_from_yaml_file(&path)
    }

    pub fn read_unlocked_id(&self, passphrase: &str) -> Result<OwnId> {
        let locked = self.read_locked_id()?;

        locked.to_unlocked(passphrase)
    }

    pub fn save_locked_id(&self, id: &id::LockedId) -> Result<()> {
        let path = self.id_path(&id.to_pubid().id);
        fs::create_dir_all(&path.parent().expect("Not /"))?;
        id.save_to(&path)
    }

    pub fn git_init_proof_dir(&self, git_url: &str) -> Result<()> {
        let git_url = https_to_git_url(git_url).unwrap_or(git_url.to_owned());
        let proof_dir = self.get_proofs_dir_path();
        fs::create_dir_all(&proof_dir)?;

        let repo = git2::Repository::init(&proof_dir)?;

        let _remote = repo.remote("origin", &git_url)?;

        Ok(())
    }
    /*
    fn trust_proof_dir_path(&self) -> PathBuf {
        self.user_dir_path().join("trust")
    }

    fn trust_proof_dir_path_for_id(&self, pub_id: &id::PubId) -> PathBuf {
        let id_str = pub_id.id_as_base64();
        self.trust_proof_dir_path().join(id_str)
    }

    fn review_proof_dir_path(&self) -> PathBuf {
        self.user_dir_path().join("review")
    }

    fn review_proof_dir_path_for_id(&self, pub_id: &id::PubId) -> PathBuf {
        let id_str = pub_id.id_as_base64();
        self.review_proof_dir_path().join(id_str)
    }

    pub fn load_all_trust_proof_from(&self, pub_id: &id::PubId) -> Result<Vec<TrustProof>> {
        let path = self.trust_proof_dir_path_for_id(pub_id);
        if !path.exists() {
            return Ok(vec![]);
        }
        let content = util::read_file_to_string(&path)?;

        TrustProof::parse(&content)
    }

    pub fn store_all_trust_proof_from(
        &self,
        pub_id: &id::PubId,
        proofs: &[TrustProof],
    ) -> Result<()> {
        util::store_to_file_with(&self.trust_proof_dir_path_for_id(pub_id), |w| {
            for proof in proofs {
                w.write_all(proof.to_string().as_bytes())?;
            }
            Ok(())
        })
    }


    pub fn load_all_review_proof_from(&self, pub_id: &id::PubId) -> Result<Vec<ReviewProof>> {
        let path = &self.review_proof_dir_path_for_id(pub_id);
        if !path.exists() {
            return Ok(vec![]);
        }
        let content = util::read_file_to_string(&path)?;

        ReviewProof::parse(&content)
    }

    pub fn store_all_review_proof_from(
        &self,
        pub_id: &id::PubId,
        proofs: &[ReviewProof],
    ) -> Result<()> {
        util::store_to_file_with(&self.review_proof_dir_path_for_id(pub_id), |w| {
            for proof in proofs {
                w.write_all(proof.to_string().as_bytes())?;
            }
            Ok(())
        })
    }

    pub fn add_trust_proof_from(&self, pub_id: &id::PubId, proof: TrustProof) -> Result<()> {
        let mut proofs = self.load_all_trust_proof_from(pub_id)?;
        proofs.push(proof);
        self.store_all_trust_proof_from(pub_id, &proofs)?;

        Ok(())
    }

    pub fn add_review_proof_from(&self, pub_id: &id::PubId, proof: ReviewProof) -> Result<()> {
        let mut proofs = self.load_all_review_proof_from(pub_id)?;
        proofs.push(proof);
        self.store_all_review_proof_from(pub_id, &proofs)?;

        Ok(())
    }

    */

    fn get_proof_rel_store_path(&self, proof: &proof::Proof) -> PathBuf {
        PathBuf::from("proofs").join(crate::proof::rel_store_path(&proof.content))
    }

    pub fn get_proofs_dir_path(&self) -> PathBuf {
        self.root_path.join("proofs")
    }

    pub fn build_trust_proof(
        &self,
        pub_ids: Vec<String>,
        passphrase: String,
        trust_or_distrust: crate::TrustOrDistrust,
    ) -> Result<()> {
        if pub_ids.is_empty() {
            bail!("No ids given.");
        }

        let mut trustdb = trustdb::TrustDB::new();
        trustdb.import_from_iter(self.proofs_iter());

        let own_id = self.read_unlocked_id(&passphrase)?;

        let from = own_id.as_pubid();

        let pub_ids: Result<Vec<_>> = pub_ids
            .into_iter()
            .map(|s| {
                let mut id = PubId::new_crevid_from_base64(&s)?;

                if let Some(url) = trustdb.lookup_url(&id.id) {
                    id.set_git_url(url.to_owned())
                }
                Ok(id)
            })
            .collect();
        let pub_ids = pub_ids?;

        let trust = proof::TrustBuilder::default()
            .from(from.to_owned())
            .comment("".into())
            .trust(if trust_or_distrust.is_trust() {
                level::Level::Medium
            } else {
                level::Level::None
            })
            .distrust(if trust_or_distrust.is_trust() {
                level::Level::None
            } else {
                level::Level::Medium
            })
            .ids(pub_ids)
            .build()
            .map_err(|e| format_err!("{}", e))?;

        let trust = util::edit_proof_content_iteractively(&trust.into(), proof::ProofType::Trust)?;

        let proof = trust.sign_by(&own_id)?;

        self.insert(&proof)?;
        println!("{}", proof);
        eprintln!("Proof added to your store");
        Ok(())
    }

    pub fn fetch_updates(&self) -> Result<()> {
        let mut already_fetched = HashSet::new();
        let mut db = trustdb::TrustDB::new();
        db.import_from_iter(self.proofs_iter());
        db.import_from_iter(proofs_iter_for_path(self.cache_remotes_path()));
        let params = Default::default();
        let user_config = self.load_user_config()?;
        let user_id = user_config.get_current_userid()?;

        let mut something_was_fetched = true;
        while something_was_fetched {
            something_was_fetched = false;
            let trust_set =
                db.calculate_trust_set(user_config.get_current_userid()?.clone(), &params);

            for id in &trust_set {
                if already_fetched.contains(id) {
                    continue;
                } else {
                    already_fetched.insert(id.to_owned());
                }
                if user_id == id {
                    continue;
                } else if let Some(url) = db.lookup_url(id) {
                    eprintln!("Fetching {}", url);
                    let success =
                        util::err_eprint_and_ignore(self.fetch_remote_git(id, url).compat());
                    if success {
                        something_was_fetched = true;
                        db.import_from_iter(proofs_iter_for_path(
                            self.get_remote_git_path(id, url),
                        ));
                    }
                } else {
                    eprintln!("No URL for {}", id);
                }
            }
        }
        Ok(())
    }

    pub fn get_remote_git_path(&self, id: &Id, url: &str) -> PathBuf {
        let digest = crev_common::blake2sum(url.as_bytes());
        let digest = base64::encode_config(&digest, base64::URL_SAFE);
        self.cache_remotes_path().join(id.to_string()).join(digest)
    }

    pub fn fetch_remote_git(&self, id: &Id, url: &str) -> Result<()> {
        let dir = self.get_remote_git_path(id, url);

        if dir.exists() {
            let repo = git2::Repository::open(dir)?;
            repo.find_remote("origin")?.fetch(&["master"], None, None)?;
            repo.set_head("FETCH_HEAD")?;
            let mut opts = git2::build::CheckoutBuilder::new();
            opts.force();
            repo.checkout_head(Some(&mut opts))?;
        } else {
            git2::Repository::clone(url, dir)?;
        }

        Ok(())
    }

    pub fn run_git(&self, args: Vec<OsString>) -> Result<std::process::ExitStatus> {
        let orig_dir = std::env::current_dir()?;
        std::env::set_current_dir(self.get_proofs_dir_path())?;

        use std::process::Command;

        let status = Command::new("git")
            .args(args)
            .status()
            .expect("failed to execute git");

        std::env::set_current_dir(orig_dir)?;

        Ok(status)
    }

    pub fn load_db(
        &self,
        params: &trustdb::TrustDistanceParams,
    ) -> Result<(trustdb::TrustDB, HashSet<Id>)> {
        let user_config = self.load_user_config()?;
        let mut db = trustdb::TrustDB::new();
        db.import_from_iter(self.proofs_iter());
        db.import_from_iter(proofs_iter_for_path(self.cache_remotes_path()));
        let trusted_set =
            db.calculate_trust_set(user_config.get_current_userid()?.clone(), &params);

        Ok((db, trusted_set))
    }
}

impl ProofStore for Local {
    fn insert(&self, proof: &proof::Proof) -> Result<()> {
        let rel_store_path = self.get_proof_rel_store_path(proof);
        let path = self.user_dir_path().join(rel_store_path);

        fs::create_dir_all(path.parent().expect("Not a root dir"))?;
        let mut file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .write(true)
            .open(path)?;

        file.write_all(proof.to_string().as_bytes())?;
        file.write_all(b"\n")?;
        file.flush()?;

        Ok(())
    }

    fn proofs_iter(&self) -> Box<Iterator<Item = proof::Proof>> {
        proofs_iter_for_path(self.get_proofs_dir_path())
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
            eprintln!("{}", e);
        });

    Box::new(proofs_iter.oks())
}
