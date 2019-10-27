use crate::{proof, Result};
use crev_common::{
    self,
    serde::{as_base64, from_base64},
};
use derive_builder::Builder;
use proof::{CommonOps, Content};
use serde::{Deserialize, Serialize};
use std::{self, default::Default, fmt, path::PathBuf};

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
    #[serde(flatten)]
    pub common: proof::Common,
    #[serde(rename = "package")]
    pub package: proof::PackageInfo,
    #[serde(flatten)]
    #[builder(default = "Default::default()")]
    pub review: super::Review,
    #[serde(skip_serializing_if = "String::is_empty", default = "Default::default")]
    #[builder(default = "Default::default()")]
    pub comment: String,
    #[serde(
        skip_serializing_if = "std::vec::Vec::is_empty",
        default = "std::vec::Vec::new"
    )]
    #[builder(default = "Default::default()")]
    pub files: Vec<File>,
}

impl Code {
    pub const KIND: &'static str = "code review";
}

impl CodeBuilder {
    pub fn from<VALUE: Into<crate::PubId>>(&mut self, value: VALUE) -> &mut Self {
        if let Some(ref mut common) = self.common {
            common.from = value.into();
        } else {
            self.common = Some(proof::Common {
                kind: Some(Code::KIND.into()),
                version: cur_version(),
                date: crev_common::now(),
                from: value.into(),
            });
        }
        self
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crev_common::serde::write_as_headerless_yaml(self, f)
    }
}

impl proof::CommonOps for Code {
    fn common(&self) -> &proof::Common {
        &self.common
    }

    fn kind(&self) -> &str {
        // Backfill the `kind` if it is empty (legacy format)
        self.common
            .kind
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or(Self::KIND)
    }
}

/// Like `Code` but serializes for interactive editing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Draft {
    review: super::Review,
    #[serde(default = "Default::default")]
    comment: String,
}

impl Draft {
    pub fn parse(s: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(&s)?)
    }
}

impl From<Code> for Draft {
    fn from(code: Code) -> Self {
        Draft {
            review: code.review,
            comment: code.comment,
        }
    }
}

impl proof::WithReview for Code {
    fn review(&self) -> &super::Review {
        &self.review
    }
}

impl proof::content::Content for Code {
    fn validate_data(&self) -> Result<()> {
        self.ensure_kind_is(Code::KIND)?;
        Ok(())
    }

    fn serialize_to(&self, fmt: &mut dyn std::fmt::Write) -> Result<()> {
        if self.common.kind.is_none() {
            // backfill during serialization
            let mut copy = self.clone();
            copy.common.kind = Some(Self::KIND.into());
            Ok(crev_common::serde::write_as_headerless_yaml(&self, fmt)?)
        } else {
            Ok(crev_common::serde::write_as_headerless_yaml(&self, fmt)?)
        }
    }
}

impl proof::ContentWithDraft for Code {
    fn to_draft(&self) -> proof::Draft {
        proof::Draft {
            title: format!(
                "Code Review of {} files of {} {}",
                self.files.len(),
                self.package.name,
                self.package.version
            ),
            body: Draft::from(self.clone()).to_string(),
        }
    }

    fn apply_draft(&self, s: &str) -> Result<Self> {
        let draft = Draft::parse(&s)?;

        let mut copy = self.clone();
        copy.review = draft.review;
        copy.comment = draft.comment;

        copy.validate_data()?;
        Ok(copy)
    }
}

impl fmt::Display for Draft {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crev_common::serde::write_as_headerless_yaml(self, f)
    }
}
