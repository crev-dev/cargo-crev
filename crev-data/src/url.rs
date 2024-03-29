use derive_builder::Builder;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Builder, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
    pub fn new_git<Stringy: Into<String>>(url: Stringy) -> Self {
        Self {
            url: url.into(),
            url_type: default_url_type(),
        }
    }

    #[must_use]
    pub fn digest(&self) -> crate::Digest {
        let digest = crev_common::blake2b256sum(self.url.to_ascii_lowercase().as_bytes());
        digest.into()
    }
}

pub(crate) fn equals_default_url_type(s: &str) -> bool {
    s == default_url_type()
}

pub(crate) fn default_url_type() -> String {
    "git".into()
}
