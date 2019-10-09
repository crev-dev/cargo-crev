use crate::{id, proof, Result};
use chrono::{self, prelude::*};
use crev_common;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::{self, default::Default, fmt, path::PathBuf};

use crev_common::serde::{as_base64, as_rfc3339_fixed, from_base64, from_rfc3339_fixed};

const BEGIN_BLOCK: &str = "-----BEGIN CODE REVIEW-----";
const BEGIN_SIGNATURE: &str = "-----BEGIN CODE REVIEW SIGNATURE-----";
const END_BLOCK: &str = "-----END CODE REVIEW-----";

const CURRENT_CODE_REVIEW_PROOF_SERIALIZATION_VERSION: i64 = -1;

fn cur_version() -> i64 {
    CURRENT_CODE_REVIEW_PROOF_SERIALIZATION_VERSION
}

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
    #[builder(default = "cur_version()")]
    version: i64,
    #[builder(default = "crev_common::now()")]
    #[serde(
        serialize_with = "as_rfc3339_fixed",
        deserialize_with = "from_rfc3339_fixed"
    )]
    date: chrono::DateTime<FixedOffset>,
    pub from: crate::PubId,
    #[serde(rename = "package")]
    pub package: proof::PackageInfo,
    #[serde(flatten)]
    #[builder(default = "Default::default()")]
    review: super::Review,
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

impl Code {
    pub fn apply_draft(&self, draft: CodeDraft) -> Code {
        let mut copy = self.clone();
        copy.review = draft.review;
        copy.comment = draft.comment;
        copy
    }
}

/// Like `Code` but serializes for interactive editing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodeDraft {
    review: super::Review,
    #[serde(default = "Default::default")]
    comment: String,
}

impl From<Code> for CodeDraft {
    fn from(code: Code) -> Self {
        CodeDraft {
            review: code.review,
            comment: code.comment,
        }
    }
}

impl Code {
    pub(crate) const BEGIN_BLOCK: &'static str = BEGIN_BLOCK;
    pub(crate) const BEGIN_SIGNATURE: &'static str = BEGIN_SIGNATURE;
    pub(crate) const END_BLOCK: &'static str = END_BLOCK;
}

impl proof::content::ContentCommon for Code {
    fn date(&self) -> &chrono::DateTime<FixedOffset> {
        &self.date
    }

    fn set_date(&mut self, date: &chrono::DateTime<FixedOffset>) {
        self.date = *date;
    }

    fn author(&self) -> &crate::PubId {
        &self.from
    }

    fn set_author(&mut self, id: &crate::PubId) {
        self.from = id.clone();
    }

    fn draft_title(&self) -> String {
        format!(
            "Code Review of {} files of {} {}",
            self.files.len(),
            self.package.name,
            self.package.version
        )
    }

    fn validate_data(&self) -> Result<()> {
        Ok(())
    }

    fn parse(s: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(&s)?)
    }

    fn parse_draft(&self, s: &str) -> Result<Self> {
        let proof: Code = self.apply_draft(CodeDraft::parse(&s)?).into();
        proof.validate_data()?;
        Ok(proof)
    }

    fn proof_type(&self) -> proof::ProofType {
        proof::ProofType::Code
    }

    fn to_draft_string(&self) -> String {
        CodeDraft::from(self.clone()).to_string()
    }
}

impl super::Common for Code {
    fn review(&self) -> &super::Review {
        &self.review
    }
}

impl Code {
    pub fn sign_by(self, id: &id::OwnId) -> Result<proof::Proof> {
        proof::content::Content::from(self).sign_by(id)
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
