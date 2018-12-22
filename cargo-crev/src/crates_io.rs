use crate::prelude::*;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct Client {
    client: crates_io_api::SyncClient,
    cache_dir: PathBuf,
}

fn get_downloads_stats(resp: &crates_io_api::CrateResponse, version: &str) -> (u64, u64) {
    (
        resp.versions
            .iter()
            .find(|v| v.num == version)
            .map(|v| v.downloads)
            .unwrap_or(0),
        resp.crate_data.downloads,
    )
}

impl Client {
    pub fn new(local: &crev_lib::Local) -> Result<Self> {
        let cache_dir = local
            .get_root_cache_dir()
            .join("crates_io")
            .join("get_crate");
        fs::create_dir_all(&cache_dir)?;
        Ok(Self {
            client: crates_io_api::SyncClient::new(),
            cache_dir: cache_dir,
        })
    }

    fn get_crate_cached_path(&self, name: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.json", name))
    }

    fn is_fresh(&self, path: &Path) -> Result<bool> {
        let metadata = fs::metadata(path)?;
        let created = metadata.created().or_else(|_e| metadata.modified())?;
        let now = std::time::SystemTime::now();
        Ok(((now - Duration::from_secs(60 * 60 * 24)) < created) && (created < now))
    }

    fn load_cached_get_crate(&self, path: &Path) -> Result<crates_io_api::CrateResponse> {
        let mut file = std::fs::File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        Ok(serde_json::from_str::<crates_io_api::CrateResponse>(
            &content,
        )?)
    }

    fn get_crate_cached(&self, name: &str) -> Result<Option<(crates_io_api::CrateResponse, bool)>> {
        let path = self.get_crate_cached_path(name);
        if path.exists() {
            Ok(Some((
                self.load_cached_get_crate(&path)?,
                self.is_fresh(&path)?,
            )))
        } else {
            Ok(None)
        }
    }

    fn store_get_crate_response_in_cache(
        &self,
        crate_: &str,
        resp: &crates_io_api::CrateResponse,
    ) -> Result<()> {
        let path = self.get_crate_cached_path(crate_);
        crev_common::store_to_file_with(&path, |file| serde_json::to_writer(file, &resp))??;
        Ok(())
    }
    fn get_crate_from_crates_io(&self, crate_: &str) -> Result<crates_io_api::CrateResponse> {
        let resp = self.client.get_crate(crate_)?;
        self.store_get_crate_response_in_cache(crate_, &resp)?;
        Ok(resp)
    }

    pub fn get_downloads_count(&self, crate_: &str, version: &str) -> Result<(u64, u64)> {
        let cached = self.get_crate_cached(crate_)?;

        match cached {
            Some((resp, true)) => Ok(get_downloads_stats(&resp, version)),
            Some((resp, false)) => match self.get_crate_from_crates_io(crate_) {
                Ok(new_resp) => Ok(get_downloads_stats(&new_resp, version)),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    Ok(get_downloads_stats(&resp, version))
                }
            },
            None => Ok(get_downloads_stats(
                &self.get_crate_from_crates_io(crate_)?,
                version,
            )),
        }
    }
}
