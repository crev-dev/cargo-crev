use derive_builder::Builder;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Builder, Serialize, Deserialize, PartialEq, Eq)]
pub struct Url {
    pub url: String,
    #[serde(
        rename = "url-type",
        skip_serializing_if = "equals_default_url_type",
        default = "default_url_type"
    )]
    pub url_type: String,
}

impl Url {
    pub fn new_git(url: String) -> Self {
        Self {
            url,
            url_type: default_url_type(),
        }
    }

    pub fn digest(&self) -> crate::Digest {
        let digest = crev_common::blake2b256sum(self.url.to_ascii_lowercase().as_bytes());
        crate::Digest::from_vec(digest)
    }
}

pub(crate) fn equals_default_url_type(s: &str) -> bool {
    s == default_url_type()
}

pub(crate) fn default_url_type() -> String {
    "git".into()
}
