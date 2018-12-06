use crate::{id, level::Level, proof, Result};
use chrono::{self, prelude::*};
use crev_common::{
    self,
    serde::{as_rfc3339_fixed, from_rfc3339_fixed},
};
use serde_yaml;
use std::fmt;

const BEGIN_BLOCK: &str = "-----BEGIN CREV TRUST -----";
const BEGIN_SIGNATURE: &str = "-----BEGIN CREV TRUST SIGNATURE-----";
const END_BLOCK: &str = "-----END CREV TRUST-----";

/// Body of a Trust Proof
#[derive(Clone, Debug, Builder, Serialize, Deserialize)]
pub struct Trust {
    #[builder(default = "crate::current_version()")]
    version: i64,
    #[builder(default = "crev_common::now()")]
    #[serde(
        serialize_with = "as_rfc3339_fixed",
        deserialize_with = "from_rfc3339_fixed"
    )]
    pub date: chrono::DateTime<FixedOffset>,
    pub from: crate::PubId,
    pub ids: Vec<crate::PubId>,
    #[builder(default = "Default::default()")]
    pub trust: Level,
    #[builder(default = "proof::default_distrust_level()")]
    #[serde(
        skip_serializing_if = "proof::equals_default_distrust_level",
        default = "proof::default_distrust_level"
    )]
    pub distrust: Level,
    #[serde(skip_serializing_if = "String::is_empty", default = "Default::default")]
    #[builder(default = "Default::default()")]
    comment: String,
}

/// Like `Trust` but serializes for interactive editing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrustDraft {
    #[serde(skip_serializing, default = "crate::current_version")]
    version: i64,
    #[serde(
        serialize_with = "as_rfc3339_fixed",
        deserialize_with = "from_rfc3339_fixed"
    )]
    pub date: chrono::DateTime<FixedOffset>,
    pub from: crate::PubId,
    pub ids: Vec<crate::PubId>,
    #[serde(
        skip_serializing_if = "proof::equals_none_level",
        default = "proof::none_level"
    )]
    pub trust: Level,
    #[serde(
        skip_serializing_if = "proof::equals_none_level",
        default = "proof::none_level"
    )]
    pub distrust: Level,
    #[serde(default = "Default::default")]
    comment: String,
}

impl From<Trust> for TrustDraft {
    fn from(trust: Trust) -> Self {
        TrustDraft {
            version: trust.version,
            date: trust.date,
            from: trust.from,
            ids: trust.ids,
            trust: trust.trust,
            distrust: trust.distrust,
            comment: trust.comment,
        }
    }
}

impl From<TrustDraft> for Trust {
    fn from(trust: TrustDraft) -> Self {
        Trust {
            version: trust.version,
            date: trust.date,
            from: trust.from,
            ids: trust.ids,
            trust: trust.trust,
            distrust: trust.distrust,
            comment: trust.comment,
        }
    }
}
impl fmt::Display for Trust {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crev_common::serde::write_as_headerless_yaml(self, f)
    }
}

impl fmt::Display for TrustDraft {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crev_common::serde::write_as_headerless_yaml(self, f)
    }
}

impl Trust {
    pub(crate) const BEGIN_BLOCK: &'static str = BEGIN_BLOCK;
    pub(crate) const BEGIN_SIGNATURE: &'static str = BEGIN_SIGNATURE;
    pub(crate) const END_BLOCK: &'static str = END_BLOCK;
}

impl proof::ContentCommon for Trust {
    fn date(&self) -> &chrono::DateTime<FixedOffset> {
        &self.date
    }

    fn author(&self) -> &crate::PubId {
        &self.from
    }
}

impl Trust {
    pub fn parse(s: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(&s)?)
    }

    pub fn sign_by(self, id: &id::OwnId) -> Result<proof::Proof> {
        super::Content::from(self).sign_by(id)
    }
}

impl TrustDraft {
    pub fn parse(s: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(&s)?)
    }
}
