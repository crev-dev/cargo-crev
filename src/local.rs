use app_dirs::{app_root, get_app_root, AppDataType, AppInfo};
use id::{self, LockedId, OwnId};
use level;
use proof::{self, Content};
use review::ReviewProof;
use serde_yaml;
use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use trust::{self, TrustProof};
use util::{self, APP_INFO};
use Result;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UserConfig {
    #[serde(rename = "id-urls")]
    pub id_urls: HashSet<String>,
}

/// Local config stored in `~/.config/crev`
pub struct Local {
    root_path: PathBuf,
}

impl Local {
    pub fn auto_open() -> Result<Self> {
        Ok(Self {
            root_path: app_root(AppDataType::UserConfig, &APP_INFO)?,
        })
    }

    pub fn user_dir_path(&self) -> PathBuf {
        self.root_path.clone()
    }

    fn id_path(&self) -> PathBuf {
        self.user_dir_path().join("id.yaml")
    }

    fn user_config_path(&self) -> PathBuf {
        self.user_dir_path().join("config.yaml")
    }

    pub fn load_user_config(&self) -> Result<UserConfig> {
        let path = self.user_config_path();
        if !path.exists() {
            return Ok(Default::default());
        }

        let config_str = util::read_file_to_string(&path)?;

        Ok(serde_yaml::from_str(&config_str)?)
    }

    pub fn store_user_config(&self, config: &UserConfig) -> Result<()> {
        let path = self.user_config_path();

        let config_str = serde_yaml::to_string(&config)?;

        util::store_str_to_file(&path, &config_str)
    }

    pub fn add_id_urls(&self, urls: Vec<String>) -> Result<()> {
        let mut config = self.load_user_config()?;

        for url in urls {
            config.id_urls.insert(url);
        }

        self.store_user_config(&config)
    }

    pub fn read_locked_id(&self) -> Result<LockedId> {
        let path = self.id_path();
        LockedId::read_from_yaml_file(&path)
    }

    pub fn read_unlocked_id(&self, passphrase: &str) -> Result<OwnId> {
        let locked = self.read_locked_id()?;

        locked.to_unlocked(passphrase)
    }

    pub fn save_locked_id(&self, id: &LockedId) -> Result<()> {
        id.save_to(&self.id_path())
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

    fn get_proof_rel_store_path(&self, content: &impl proof::Content) -> PathBuf {
        PathBuf::from("proof").join(content.rel_store_path())
    }

    pub fn trust_ids(&self, pub_ids: Vec<String>) -> Result<()> {
        if pub_ids.is_empty() {
            bail!("No ids to trust. Use `add` first.");
        }
        let passphrase = util::read_passphrase()?;
        let id = self.read_unlocked_id(&passphrase)?;
        let pub_id = id.to_pubid();

        let trust = trust::TrustBuilder::default()
            .from(id.pub_key_as_base64())
            .from_name(id.name().into())
            .from_type(id.type_as_string())
            .from_urls(vec!["TODO".into()])
            .comment(Some("".into()))
            .trust(level::Level::Medium)
            .trusted_ids(pub_ids)
            .build()
            .map_err(|e| format_err!("{}", e))?;

        let trust = util::edit_proof_content_iteractively(&trust)?;

        let rel_store_path = self.get_proof_rel_store_path(&trust);

        let proof = trust.sign(&id)?;

        self.append_proof_at(&proof, &rel_store_path)?;
        println!("{}", proof);
        eprintln!("Proof added to your store");
        Ok(())
    }

    pub fn append_proof<T: proof::Content>(
        &self,
        proof: &proof::Proof<T>,
        content: &T,
    ) -> Result<()> {
        let rel_store_path = self.get_proof_rel_store_path(content);
        self.append_proof_at(&proof, &rel_store_path)
    }

    fn append_proof_at<T: proof::Content>(
        &self,
        proof: &proof::Proof<T>,
        rel_store_path: &Path,
    ) -> Result<()> {
        let path = self.user_dir_path().join(rel_store_path);

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
}
