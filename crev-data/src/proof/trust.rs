use crate::{
    proof::{self, content::ValidationResult, CommonOps, Content},
    serde_content_serialize, serde_draft_serialize, Error, Level, ParseError, Result,
};

use derive_builder::Builder;
use serde::{Deserialize, Serialize};

use std::fmt;

use super::{OverrideItem, OverrideItemDraft};

const CURRENT_TRUST_PROOF_SERIALIZATION_VERSION: i64 = -1;

fn cur_version() -> i64 {
    CURRENT_TRUST_PROOF_SERIALIZATION_VERSION
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TrustLevel {
    Distrust,
    None,
    Low,
    Medium,
    High,
}

impl Default for TrustLevel {
    fn default() -> Self {
        TrustLevel::Medium
    }
}

impl fmt::Display for TrustLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TrustLevel::*;
        f.pad(match self {
            Distrust => "distrust",
            None => "none",
            Low => "low",
            Medium => "medium",
            High => "high",
        })
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Can't convert string to TrustLevel. Possible values are: \"none\" or \"untrust\", \"low\", \"medium\", \"high\" and \"distrust\".")]
pub struct FromStrErr;

impl std::str::FromStr for TrustLevel {
    type Err = FromStrErr;

    fn from_str(s: &str) -> std::result::Result<TrustLevel, FromStrErr> {
        Ok(match s {
            "none" | "untrust" => TrustLevel::None,
            "low" => TrustLevel::Low,
            "medium" => TrustLevel::Medium,
            "high" => TrustLevel::High,
            "distrust" => TrustLevel::Distrust,
            _ => return Err(FromStrErr),
        })
    }
}

impl std::convert::From<Level> for TrustLevel {
    fn from(l: Level) -> Self {
        match l {
            Level::None => TrustLevel::None,
            Level::Low => TrustLevel::Low,
            Level::Medium => TrustLevel::Medium,
            Level::High => TrustLevel::High,
        }
    }
}

impl TrustLevel {
    #[allow(unused)]
    fn from_str(s: &str) -> Result<TrustLevel> {
        Ok(match s {
            "distrust" => TrustLevel::Distrust,
            "none" => TrustLevel::None,
            "low" => TrustLevel::Low,
            "medium" => TrustLevel::Medium,
            "high" => TrustLevel::High,
            _ => return Err(Error::UnknownLevel(s.into())),
        })
    }
}

/// Body of a Trust Proof
#[derive(Clone, Debug, Builder, Serialize, Deserialize)]
pub struct Trust {
    #[serde(flatten)]
    pub common: proof::Common,
    pub ids: Vec<crate::PublicId>,
    #[builder(default = "Default::default()")]
    pub trust: TrustLevel,
    #[serde(skip_serializing_if = "String::is_empty", default = "Default::default")]
    #[builder(default = "Default::default()")]
    pub comment: String,
    #[serde(
        default = "Default::default",
        skip_serializing_if = "Vec::is_empty",
        rename = "override"
    )]
    #[builder(default = "Default::default()")]
    pub override_: Vec<OverrideItem>,
}

impl TrustBuilder {
    pub fn from<VALUE: Into<crate::PublicId>>(&mut self, value: VALUE) -> &mut Self {
        if let Some(ref mut common) = self.common {
            common.from = value.into();
        } else {
            self.common = Some(proof::Common {
                kind: Some(Trust::KIND.into()),
                version: cur_version(),
                date: crev_common::now(),
                from: value.into(),
                original: None,
            });
        }
        self
    }
}

impl fmt::Display for Trust {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.serialize_to(f).map_err(|_| fmt::Error)
    }
}

impl proof::CommonOps for Trust {
    fn common(&self) -> &proof::Common {
        &self.common
    }

    fn kind(&self) -> &str {
        // Backfill the `kind` if it is empty (legacy format)
        self.common.kind.as_deref().unwrap_or(Self::KIND)
    }
}

impl Trust {
    pub const KIND: &'static str = "trust";

    pub fn touch_date(&mut self) {
        self.common.date = crev_common::now();
    }
}

/// Like `Trust` but serializes for interactive editing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Draft {
    pub trust: TrustLevel,
    #[serde(default = "Default::default", skip_serializing_if = "String::is_empty")]
    comment: String,
    #[serde(
        default = "Default::default",
        skip_serializing_if = "Vec::is_empty",
        rename = "override"
    )]
    override_: Vec<OverrideItemDraft>,
}

impl From<Trust> for Draft {
    fn from(trust: Trust) -> Self {
        Draft {
            trust: trust.trust,
            comment: trust.comment,
            override_: trust.override_.into_iter().map(Into::into).collect(),
        }
    }
}

impl fmt::Display for Draft {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        serde_draft_serialize!(self, fmt);
        Ok(())
    }
}

impl proof::Content for Trust {
    fn serialize_to(&self, fmt: &mut dyn std::fmt::Write) -> fmt::Result {
        serde_content_serialize!(self, fmt);
        Ok(())
    }

    fn validate_data(&self) -> ValidationResult<()> {
        self.ensure_kind_is(Self::KIND)?;
        Ok(())
    }
}

impl Trust {
    fn draft_title(&self) -> String {
        match self.ids.len() {
            0 => "Trust for noone?!".into(),
            1 => format!("Trust for {} {}", self.ids[0].id, self.ids[0].url_display()),
            n => format!(
                "Trust for {} {} and {} other",
                self.ids[0].id,
                self.ids[0].url_display(),
                n - 1
            ),
        }
    }
}

impl proof::ContentWithDraft for Trust {
    fn to_draft(&self) -> proof::Draft {
        proof::Draft {
            title: self.draft_title(),
            body: Draft::from(self.clone()).to_string(),
        }
    }

    fn apply_draft(&self, s: &str) -> Result<Self> {
        let draft = Draft::parse(s)?;

        let mut copy = self.clone();
        copy.trust = draft.trust;
        copy.comment = draft.comment;
        copy.override_ = draft.override_.into_iter().map(Into::into).collect();

        copy.validate_data()?;
        Ok(copy)
    }
}

impl Draft {
    pub fn parse(s: &str) -> std::result::Result<Self, ParseError> {
        serde_yaml::from_str(s).map_err(ParseError::Draft)
    }
}
