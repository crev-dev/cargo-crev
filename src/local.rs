use id::{LockedId, OwnId};
use std::{collections::HashSet, path::PathBuf};
use util;
use Result;

use app_dirs::{app_root, get_app_root, AppDataType, AppInfo};
use proof::TrustProof;
use serde_yaml;
use util::APP_INFO;

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

    fn trust_proof_dir_path(&self) -> PathBuf {
        self.user_dir_path().join("trust")
    }

    fn review_proof_dir_path(&self) -> PathBuf {
        self.user_dir_path().join("review")
    }

    pub fn load_all_trust_proof_from(&self, id: String) -> Result<Vec<TrustProof>> {
        let content = util::read_file_to_string(&self.trust_proof_dir_path().join(id))?;

        unimplemented!();
    }
}
