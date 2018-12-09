use crate::{id, proof, Result};
use chrono::{self, prelude::*};
use crev_common;
use serde_yaml;
use std::{self, default::Default, fmt, path::PathBuf};

use crev_common::serde::{as_base64, as_rfc3339_fixed, from_base64, from_rfc3339_fixed};

const BEGIN_BLOCK: &str = "-----BEGIN CODE REVIEW-----";
const BEGIN_SIGNATURE: &str = "-----BEGIN CODE REVIEW SIGNATURE-----";
const END_BLOCK: &str = "-----END CODE REVIEW-----";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct File {
    pub path: PathBuf,
    #[serde(serialize_with = "as_base64", deserialize_with = "from_base64")]
    pub digest: Vec<u8>,
    #[serde(rename = "digest-type")]
    #[serde(
        skip_serializing_if = "proof::equals_default_digest_type",
        default = "proof::default_digest_type"
    )]
    pub digest_type: String,
}

/// Body of a Code Review Proof
#[derive(Clone, Builder, Debug, Serialize, Deserialize)]
// TODO: validate setters(no newlines, etc)
// TODO: https://github.com/colin-kiegel/rust-derive-builder/issues/136
pub struct Code {
    #[builder(default = "crate::current_version()")]
    version: i64,
    #[builder(default = "crev_common::now()")]
    #[serde(
        serialize_with = "as_rfc3339_fixed",
        deserialize_with = "from_rfc3339_fixed"
    )]
    date: chrono::DateTime<FixedOffset>,
    pub from: crate::PubId,
    #[serde(rename = "project")]
    pub project: proof::ProjectInfo,
    #[serde(flatten)]
    #[builder(default = "Default::default()")]
    review: super::Score,
    #[serde(skip_serializing_if = "String::is_empty", default = "Default::default")]
    #[builder(default = "Default::default()")]
    comment: String,
    #[serde(
        skip_serializing_if = "std::vec::Vec::is_empty",
        default = "std::vec::Vec::new"
    )]
    #[builder(default = "Default::default()")]
    pub files: Vec<File>,
}

/// Like `Code` but serializes for interactive editing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodeDraft {
    #[serde(skip_serializing, default = "crate::current_version")]
    version: i64,
    #[serde(
        serialize_with = "as_rfc3339_fixed",
        deserialize_with = "from_rfc3339_fixed"
    )]
    date: chrono::DateTime<FixedOffset>,
    pub from: crate::PubId,
    #[serde(rename = "project")]
    pub project: proof::ProjectInfo,
    review: super::Score,
    #[serde(default = "Default::default")]
    comment: String,
    #[serde(
        skip_serializing_if = "std::vec::Vec::is_empty",
        default = "std::vec::Vec::new"
    )]
    pub files: Vec<File>,
}

impl From<Code> for CodeDraft {
    fn from(code: Code) -> Self {
        CodeDraft {
            version: code.version,
            date: code.date,
            from: code.from,
            project: code.project,
            review: code.review,
            comment: code.comment,
            files: code.files,
        }
    }
}

impl From<CodeDraft> for Code {
    fn from(code: CodeDraft) -> Self {
        Code {
            version: code.version,
            date: code.date,
            from: code.from,
            project: code.project,
            review: code.review,
            comment: code.comment,
            files: code.files,
        }
    }
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
    fn author(&self) -> &crate::PubId {
        &self.from
    }
}

impl super::Common for Code {
    fn score(&self) -> &super::Score {
        &self.review
    }
}

impl Code {
    pub fn parse(s: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(&s)?)
    }

    pub fn sign_by(self, id: &id::OwnId) -> Result<proof::Proof> {
        proof::Content::from(self).sign_by(id)
    }
}

impl CodeDraft {
    pub fn parse(s: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(&s)?)
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crev_common::serde::write_as_headerless_yaml(self, f)
    }
}

impl fmt::Display for CodeDraft {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crev_common::serde::write_as_headerless_yaml(self, f)
    }
}
