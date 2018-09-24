use chrono::{self, prelude::*};
use crate::{
    id,
    proof::{self, Proof},
    Result,
};
use crev_common;
use serde_yaml;
use std::{self, default::Default, fmt, path::PathBuf};

use crev_common::serde::{as_hex, as_rfc3339_fixed, from_hex, from_rfc3339_fixed};

const BEGIN_BLOCK: &str = "-----BEGIN CODE REVIEW-----";
const BEGIN_SIGNATURE: &str = "-----BEGIN CODE REVIEW SIGNATURE-----";
const END_BLOCK: &str = "-----END CODE REVIEW-----";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct File {
    pub path: PathBuf,
    #[serde(serialize_with = "as_hex", deserialize_with = "from_hex")]
    pub digest: Vec<u8>,
    #[serde(rename = "digest-type")]
    #[serde(
        skip_serializing_if = "proof::equals_default_digest_type",
        default = "proof::default_digest_type"
    )]
    pub digest_type: String,
}

#[derive(Clone, Builder, Debug, Serialize, Deserialize)]
// TODO: validate setters(no newlines, etc)
// TODO: https://github.com/colin-kiegel/rust-derive-builder/issues/136
/// Unsigned proof of code review
pub struct Code {
    #[builder(default = "crev_common::now()")]
    #[serde(
        serialize_with = "as_rfc3339_fixed",
        deserialize_with = "from_rfc3339_fixed"
    )]
    date: chrono::DateTime<FixedOffset>,
    pub from: proof::Id,
    #[serde(rename = "project-id")]
    pub project: proof::Project,
    revision: String,
    #[serde(
        rename = "revision-type",
        skip_serializing_if = "proof::equals_default_revision_type",
        default = "proof::default_revision_type"
    )]
    #[builder(default = "\"git\".into()")]
    revision_type: String,

    #[serde(
        skip_serializing_if = "String::is_empty",
        default = "Default::default"
    )]
    #[builder(default = "Default::default()")]
    comment: String,
    #[builder(default = "None")]
    #[serde(
        skip_serializing_if = "Option::is_none",
        default = "Default::default"
    )]
    digest: Option<String>,
    #[serde(
        skip_serializing_if = "proof::equals_default_digest_type",
        default = "proof::default_digest_type"
    )]
    #[builder(default = "proof::default_digest_type()")]
    digest_type: String,
    #[serde(flatten)]
    #[builder(default = "Default::default()")]
    score: super::Score,
    #[serde(
        skip_serializing_if = "std::vec::Vec::is_empty",
        default = "std::vec::Vec::new"
    )]
    #[builder(default = "Default::default()")]
    pub files: Vec<File>,
}

impl Code {
    pub(crate) const BEGIN_BLOCK: &'static str = BEGIN_BLOCK;
    pub(crate) const BEGIN_SIGNATURE: &'static str = BEGIN_SIGNATURE;
    pub(crate) const END_BLOCK: &'static str = END_BLOCK;
}
impl proof::ContentCommon for Code {
    fn date(&self) -> &chrono::DateTime<FixedOffset> {
        &self.date
    }
    fn from(&self) -> &proof::Id {
        &self.from
    }
}

impl super::Common for Code {
    fn project_id(&self) -> &str {
        &self.project.id
    }

    fn score(&self) -> &super::Score {
        &self.score
    }
}

impl Code {
    pub fn parse(s: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(&s)?)
    }

    pub fn sign(self, id: &id::OwnId) -> Result<Proof> {
        proof::Content::from(self).sign(id)
    }
}

impl fmt::Display for Code {
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
