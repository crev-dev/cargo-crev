use id::{LockedId, OwnId};
use std::{collections::HashSet, path::PathBuf};
use util;
use Result;

use app_dirs::{app_root, get_app_root, AppDataType, AppInfo};
use serde_yaml;
use util::APP_INFO;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UserConfig {
    #[serde(rename = "id-urls")]
    pub id_urls: HashSet<String>,
}

/// Local config stored in `~/.config/crev`
pub struct Local;

impl Local {
    pub fn user_dir_path() -> Result<PathBuf> {
        Ok(app_root(AppDataType::UserConfig, &APP_INFO)?)
    }

    fn id_path() -> Result<PathBuf> {
        Ok(Self::user_dir_path()?.join("id.yaml"))
    }

    fn user_config_path() -> Result<PathBuf> {
        Ok(Self::user_dir_path()?.join("config.yaml"))
    }

    pub fn load_user_config() -> Result<UserConfig> {
        let path = Self::user_config_path()?;
        if !path.exists() {
            return Ok(Default::default());
        }

        let config_str = util::read_file_to_string(&path)?;

        Ok(serde_yaml::from_str(&config_str)?)
    }

    pub fn store_user_config(config: &UserConfig) -> Result<()> {
        let path = Self::user_config_path()?;

        let config_str = serde_yaml::to_string(&config)?;

        util::store_str_to_file(&path, &config_str)
    }

    pub fn add_id_urls(urls: Vec<String>) -> Result<()> {
        let mut config = Local::load_user_config()?;

        for url in urls {
            config.id_urls.insert(url);
        }

        Local::store_user_config(&config)
    }

    pub fn read_locked_id() -> Result<LockedId> {
        let path = Self::id_path()?;
        LockedId::read_from_yaml_file(&path)
    }

    pub fn read_unlocked_id(passphrase: &str) -> Result<OwnId> {
        let locked = Self::read_locked_id()?;

        locked.to_unlocked(passphrase)
    }

    pub fn save_locked_id(id: &LockedId) -> Result<()> {
        id.save_to(&Self::id_path()?)
    }
}
