use chrono::{self, prelude::*};
use crev_common;
use id;
use level::Level;
use proof::{self, Proof};
use serde_yaml;
use std::{fmt, path::PathBuf};

use Result;

use crev_common::serde::{as_hex, as_rfc3339_fixed, from_hex, from_rfc3339_fixed};

const BEGIN_BLOCK: &str = "-----BEGIN CODE REVIEW-----";
const BEGIN_SIGNATURE: &str = "-----BEGIN CODE REVIEW SIGNATURE-----";
const END_BLOCK: &str = "-----END CODE REVIEW-----";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReviewFile {
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
pub struct Review {
    #[builder(default = "crev_common::now()")]
    #[serde(
        serialize_with = "as_rfc3339_fixed",
        deserialize_with = "from_rfc3339_fixed"
    )]
    date: chrono::DateTime<FixedOffset>,
    pub from: proof::Id,
    #[serde(rename = "project-id")]
    project_id: String,
    revision: String,
    #[serde(rename = "revision-type")]
    #[builder(default = "\"git\".into()")]
    revision_type: String,
    #[builder(default = "None")]
    comment: Option<String>,
    pub thoroughness: Level,
    pub understanding: Level,
    pub trust: Level,
    pub files: Vec<ReviewFile>,
}

impl Review {
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
        Some(&self.project_id)
    }

    pub fn from_pubid(&self) -> String {
        self.from.id.clone()
    }

    pub fn from_url(&self) -> Option<String> {
        self.from.url.as_ref().map(|v| v.url.to_owned())
    }

    pub fn parse(s: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(&s)?)
    }

    pub fn sign(self, id: &id::OwnId) -> Result<Proof> {
        super::Content::from(self).sign(id)
    }
}

impl fmt::Display for Review {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
