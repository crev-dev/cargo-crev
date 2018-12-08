use crate::{id, proof, Result};
use chrono::{self, prelude::*};
use crev_common::{
    self,
    serde::{as_rfc3339_fixed, from_rfc3339_fixed},
};
use serde_yaml;
use std::{default::Default, fmt};

const BEGIN_BLOCK: &str = "-----BEGIN CREV PROJECT REVIEW-----";
const BEGIN_SIGNATURE: &str = "-----BEGIN CREV PROJECT REVIEW SIGNATURE-----";
const END_BLOCK: &str = "-----END CREV PROJECT REVIEW-----";

/// Body of a Project Review Proof
#[derive(Clone, Builder, Debug, Serialize, Deserialize)]
// TODO: validate setters(no newlines, etc)
// TODO: https://github.com/colin-kiegel/rust-derive-builder/issues/136
/// Unsigned proof of code review
pub struct Project {
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
    #[builder(default = "Default::default()")]
    review: super::Score,
    #[serde(skip_serializing_if = "String::is_empty", default = "Default::default")]
    #[builder(default = "Default::default()")]
    comment: String,
}

/// Like `Project` but serializes for interactive editing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectDraft {
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
}

impl From<Project> for ProjectDraft {
    fn from(project: Project) -> Self {
        ProjectDraft {
            version: project.version,
            date: project.date,
            from: project.from,
            project: project.project,
            review: project.review,
            comment: project.comment,
        }
    }
}

impl From<ProjectDraft> for Project {
    fn from(project: ProjectDraft) -> Self {
        Project {
            version: project.version,
            date: project.date,
            from: project.from,
            project: project.project,
            review: project.review,
            comment: project.comment,
        }
    }
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
    fn score(&self) -> &super::Score {
        &self.review
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

impl ProjectDraft {
    pub fn parse(s: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(&s)?)
    }
}

impl fmt::Display for Project {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crev_common::serde::write_as_headerless_yaml(self, f)
    }
}

impl fmt::Display for ProjectDraft {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crev_common::serde::write_as_headerless_yaml(self, f)
    }
}
