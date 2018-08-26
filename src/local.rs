use id::{LockedId, OwnId};
use std::path::PathBuf;
use Result;

use app_dirs::{app_root, get_app_root, AppDataType, AppInfo};
use util::APP_INFO;

/// Local config stored in `~/.config/crev`
pub struct Local {
    // root dir, where `.crev` subdiretory resides
    root_dir: PathBuf,
}

impl Local {
    pub fn path() -> Result<PathBuf> {
        Ok(app_root(AppDataType::UserConfig, &APP_INFO)?.join("id.yaml"))
    }

    pub fn id_path() -> Result<PathBuf> {
        Ok(Self::path()?.join("id.yaml"))
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
