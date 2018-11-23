use chrono::{self, prelude::*};
use crate::{id, proof, Result};
use crev_common::{
    self,
    serde::{as_hex, as_rfc3339_fixed, from_hex, from_rfc3339_fixed},
};
use serde_yaml;
use std::{default::Default, fmt};

const BEGIN_BLOCK: &str = "-----BEGIN PROJECT REVIEW-----";
const BEGIN_SIGNATURE: &str = "-----BEGIN PROJECT REVIEW SIGNATURE-----";
const END_BLOCK: &str = "-----END PROJECT REVIEW-----";

#[derive(Clone, Builder, Debug, Serialize, Deserialize)]
// TODO: validate setters(no newlines, etc)
// TODO: https://github.com/colin-kiegel/rust-derive-builder/issues/136
/// Unsigned proof of code review
pub struct Project {
    #[builder(default = "crev_common::now()")]
    #[serde(
        serialize_with = "as_rfc3339_fixed",
        deserialize_with = "from_rfc3339_fixed"
    )]
    date: chrono::DateTime<FixedOffset>,
    pub from: crate::PubId,
    #[serde(rename = "project")]
    #[builder(default = "Default::default()")]
    #[serde(skip_serializing_if = "proof::equals_none")]
    pub project: Option<proof::Project>,
    #[serde(flatten)]
    #[builder(default = "Default::default()")]
    pub revision: Option<proof::Revision>,
    #[serde(skip_serializing_if = "String::is_empty", default = "Default::default")]
    #[builder(default = "Default::default()")]
    comment: String,
    #[serde(flatten)]
    #[builder(default = "Default::default()")]
    score: super::Score,
    #[serde(serialize_with = "as_hex", deserialize_with = "from_hex")]
    pub digest: Vec<u8>,
    #[serde(
        skip_serializing_if = "proof::equals_default_digest_type",
        default = "proof::default_digest_type"
    )]
    #[builder(default = "proof::default_digest_type()")]
    pub digest_type: String,
}

impl Project {
    pub(crate) const BEGIN_BLOCK: &'static str = BEGIN_BLOCK;
    pub(crate) const BEGIN_SIGNATURE: &'static str = BEGIN_SIGNATURE;
    pub(crate) const END_BLOCK: &'static str = END_BLOCK;
}

impl proof::ContentCommon for Project {
    fn date(&self) -> &chrono::DateTime<FixedOffset> {
        &self.date
    }

    fn author(&self) -> &crate::PubId {
        &self.from
    }
}

impl super::Common for Project {
    fn project_id(&self) -> Option<&str> {
        self.project.as_ref().map(|p| p.id.as_str())
    }

    fn score(&self) -> &super::Score {
        &self.score
    }
}

impl Project {
    pub fn parse(s: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(&s)?)
    }

    pub fn sign_by(self, id: &id::OwnId) -> Result<proof::Proof> {
        proof::Content::from(self).sign_by(id)
    }
}

impl fmt::Display for Project {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let yaml_document = serde_yaml::to_string(self).map_err(|_| fmt::Error)?;
        let mut lines = yaml_document.lines();
        let dropped_header = lines.next();
        assert_eq!(dropped_header, Some("---"));

        for line in lines {
            f.write_str(&line)?;
            f.write_str("\n")?;
        }
        Ok(())
    }
}
