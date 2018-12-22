pub struct Client {
    client: crates_io_api::SyncClient,
}

impl Client {
    pub fn new() -> Self {
        Self {
            client: crates_io_api::SyncClient::new(),
        }
    }

    pub fn get_downloads_count(&self, crate_name: &str, version: &str) -> (u64, u64) {
        self.client
            .get_crate(crate_name)
            .map(|crate_info| {
                (
                    crate_info
                        .versions
                        .iter()
                        .find(|v| v.num == version)
                        .map(|v| v.downloads)
                        .unwrap_or(0),
                    crate_info.crate_data.downloads,
                )
            })
            .unwrap_or((0, 0))
    }
}
