use crate::{id, proof, Result};
use chrono::{self, prelude::*};
use crev_common::{
    self,
    serde::{as_rfc3339_fixed, from_rfc3339_fixed},
};
use serde_yaml;
use std::{default::Default, fmt};

const BEGIN_BLOCK: &str = "-----BEGIN CREV PACKAGE REVIEW-----";
const BEGIN_SIGNATURE: &str = "-----BEGIN CREV PACKAGE REVIEW SIGNATURE-----";
const END_BLOCK: &str = "-----END CREV PACKAGE REVIEW-----";

const CURRENT_PACKAGE_REVIEW_PROOF_SERIALIZATION_VERSION: i64 = -1;

fn cur_version() -> i64 {
    CURRENT_PACKAGE_REVIEW_PROOF_SERIALIZATION_VERSION
}

/// Body of a Package Review Proof
#[derive(Clone, Builder, Debug, Serialize, Deserialize)]
// TODO: https://github.com/colin-kiegel/rust-derive-builder/issues/136
pub struct Package {
    #[builder(default = "cur_version()")]
    version: i64,
    #[builder(default = "crev_common::now()")]
    #[serde(
        serialize_with = "as_rfc3339_fixed",
        deserialize_with = "from_rfc3339_fixed"
    )]
    pub date: chrono::DateTime<FixedOffset>,
    pub from: crate::PubId,
    #[serde(rename = "package")]
    pub package: proof::PackageInfo,
    #[builder(default = "Default::default()")]
    pub review: super::Review,
    #[serde(skip_serializing_if = "String::is_empty", default = "Default::default")]
    #[builder(default = "Default::default()")]
    pub comment: String,
}

impl Package {
    pub fn apply_draft(&self, draft: PackageDraft) -> Package {
        let mut copy = self.clone();
        copy.review = draft.review;
        copy.comment = draft.comment;
        copy
    }
}

/// Like `Package` but serializes for interactive editing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageDraft {
    review: super::Review,
    #[serde(default = "Default::default")]
    comment: String,
}

impl From<Package> for PackageDraft {
    fn from(package: Package) -> Self {
        PackageDraft {
            review: package.review,
            comment: package.comment,
        }
    }
}

impl Package {
    pub(crate) const BEGIN_BLOCK: &'static str = BEGIN_BLOCK;
    pub(crate) const BEGIN_SIGNATURE: &'static str = BEGIN_SIGNATURE;
    pub(crate) const END_BLOCK: &'static str = END_BLOCK;
}

impl proof::ContentCommon for Package {
    fn date(&self) -> &chrono::DateTime<FixedOffset> {
        &self.date
    }

    fn set_date(&mut self, date: &chrono::DateTime<FixedOffset>) {
        self.date = date.clone();
    }

    fn author(&self) -> &crate::PubId {
        &self.from
    }

    fn set_author(&mut self, id: &crate::PubId) {
        self.from = id.clone();
    }

    fn draft_title(&self) -> String {
        format!(
            "Package Review of {} {}",
            self.package.name, self.package.version
        )
    }
}

impl super::Common for Package {
    fn review(&self) -> &super::Review {
        &self.review
    }
}

impl Package {
    pub fn parse(s: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(&s)?)
    }

    pub fn sign_by(self, id: &id::OwnId) -> Result<proof::Proof> {
        proof::Content::from(self).sign_by(id)
    }
}

impl PackageDraft {
    pub fn parse(s: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(&s)?)
    }
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crev_common::serde::write_as_headerless_yaml(self, f)
    }
}

impl fmt::Display for PackageDraft {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crev_common::serde::write_as_headerless_yaml(self, f)
    }
}
