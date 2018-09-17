use chrono::{self, prelude::*};
use crate::{
    id,
    level::Level,
    proof::{self, Proof},
    Result,
};
use crev_common::{
    self,
    serde::{as_rfc3339_fixed, from_rfc3339_fixed},
};
use serde_yaml;
use std::fmt;

const BEGIN_BLOCK: &str = "-----BEGIN CODE REVIEW TRUST-----";
const BEGIN_SIGNATURE: &str = "-----BEGIN CODE REVIEW TRUST SIGNATURE-----";
const END_BLOCK: &str = "-----END CODE REVIEW TRUST-----";

#[derive(Clone, Debug, Builder, Serialize, Deserialize)]
pub struct Trust {
    #[builder(default = "crev_common::now()")]
    #[serde(
        serialize_with = "as_rfc3339_fixed",
        deserialize_with = "from_rfc3339_fixed"
    )]
    pub date: chrono::DateTime<FixedOffset>,
    pub from: proof::Id,
    pub trusted: Vec<proof::Id>,
    #[serde(
        skip_serializing_if = "String::is_empty",
        default = "Default::default"
    )]
    #[builder(default = "Default::default()")]
    comment: String,
    #[builder(default = "Default::default()")]
    pub trust: Level,
}

impl fmt::Display for Trust {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

    pub fn date_utc(&self) -> chrono::DateTime<Utc> {
        self.date().with_timezone(&Utc)
    }

    pub fn project_id(&self) -> Option<&str> {
        None
    }
    pub fn from_pubid(&self) -> String {
        self.from.id.clone()
    }
    pub fn from_url(&self) -> Option<String> {
        self.from.url.as_ref().map(|v| v.url.clone())
    }
    pub fn parse(s: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(&s)?)
    }

    pub fn sign(self, id: &id::OwnId) -> Result<Proof> {
        super::Content::from(self).sign(id)
    }
}
