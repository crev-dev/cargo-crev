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

/// Body of a Package Review Proof
#[derive(Clone, Builder, Debug, Serialize, Deserialize)]
// TODO: https://github.com/colin-kiegel/rust-derive-builder/issues/136
pub struct Package {
    #[builder(default = "crate::current_version()")]
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
    review: super::Review,
    #[serde(skip_serializing_if = "String::is_empty", default = "Default::default")]
    #[builder(default = "Default::default()")]
    comment: String,
}

/// Like `Package` but serializes for interactive editing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageDraft {
    #[serde(skip_serializing, default = "crate::current_version")]
    version: i64,
    #[serde(
        serialize_with = "as_rfc3339_fixed",
        deserialize_with = "from_rfc3339_fixed"
    )]
    date: chrono::DateTime<FixedOffset>,
    pub from: crate::PubId,
    #[serde(rename = "package")]
    pub package: proof::PackageInfo,
    review: super::Review,
    #[serde(default = "Default::default")]
    comment: String,
}

impl From<Package> for PackageDraft {
    fn from(package: Package) -> Self {
        PackageDraft {
            version: package.version,
            date: package.date,
            from: package.from,
            package: package.package,
            review: package.review,
            comment: package.comment,
        }
    }
}

impl From<PackageDraft> for Package {
    fn from(package: PackageDraft) -> Self {
        Package {
            version: package.version,
            date: package.date,
            from: package.from,
            package: package.package,
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

    fn author(&self) -> &crate::PubId {
        &self.from
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
