use chrono::{self, prelude::*};
use crev_common::{
    self,
    serde::{as_rfc3339_fixed, from_rfc3339_fixed},
};
use id;
use level::Level;
use proof::{self, Proof};
use serde_yaml;
use std::fmt;
use Result;

const BEGIN_BLOCK: &str = "-----BEGIN CODE REVIEW TRUST-----";
const BEGIN_SIGNATURE: &str = "-----BEGIN CODE REVIEW TRUST SIGNATURE-----";
const END_BLOCK: &str = "-----END CODE REVIEW TRUST-----";

#[derive(Clone, Debug, Builder, Serialize, Deserialize)]
pub struct Trust {
    #[builder(default = "crev_common::now()")]
    #[serde(serialize_with = "as_rfc3339_fixed", deserialize_with = "from_rfc3339_fixed")]
    pub date: chrono::DateTime<FixedOffset>,
    pub from: String,
    #[serde(rename = "from-url")]
    pub from_url: String,
    #[serde(
        rename = "from-type",
        skip_serializing_if = "proof::equals_crev",
        default = "proof::default_crev_value"
    )]
    #[builder(default = "\"crev\".into()")]
    pub from_type: String,
    #[serde(rename = "trusted-ids")]
    pub trusted_ids: Vec<String>,
    #[serde(rename = "comment")]
    pub comment: Option<String>,
    pub trust: Level,
}

impl fmt::Display for Trust {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        crev_common::serde::write_as_headerless_yaml(self, f)
    }
}

impl Trust {
    pub(crate) const BEGIN_BLOCK: &'static str = BEGIN_BLOCK;
    pub(crate) const BEGIN_SIGNATURE: &'static str = BEGIN_SIGNATURE;
    pub(crate) const END_BLOCK: &'static str = END_BLOCK;

    pub fn date(&self) -> chrono::DateTime<FixedOffset> {
        self.date
    }
    pub fn project_id(&self) -> Option<&str> {
        None
    }
    pub fn from_pubid(&self) -> String {
        self.from.clone()
    }
    pub fn from_url(&self) -> String {
        self.from_url.clone()
    }
    pub fn parse(s: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(&s)?)
    }

    pub fn sign(self, id: &id::OwnId) -> Result<Proof> {
        super::Content::from(self).sign(id)
    }
}
