use crate::{
    proof::{self, CommonOps, Content},
    serde_content_serialize, serde_draft_serialize, Level, Result,
};
use crev_common;
use derive_builder::Builder;
use failure::bail;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::fmt;

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
        use self::TrustLevel::*;
        f.pad(match self {
            Distrust => "distrust",
            None => "none",
            Low => "low",
            Medium => "medium",
            High => "high",
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
            _ => bail!("Unknown level: {}", s),
        })
    }
}

/// Body of a Trust Proof
#[derive(Clone, Debug, Builder, Serialize, Deserialize)]
pub struct Trust {
    #[serde(flatten)]
    pub common: proof::Common,
    pub ids: Vec<crate::PubId>,
    #[builder(default = "Default::default()")]
    pub trust: TrustLevel,
    #[serde(skip_serializing_if = "String::is_empty", default = "Default::default")]
    #[builder(default = "Default::default()")]
    pub comment: String,
}

impl TrustBuilder {
    pub fn from<VALUE: Into<crate::PubId>>(&mut self, value: VALUE) -> &mut Self {
        if let Some(ref mut common) = self.common {
            common.from = value.into();
        } else {
            self.common = Some(proof::Common {
                kind: Some(Trust::KIND.into()),
                version: cur_version(),
                date: crev_common::now(),
                from: value.into(),
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
        self.common
            .kind
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or(Self::KIND)
    }
}

impl Trust {
    pub const KIND: &'static str = "trust";
}

/// Like `Trust` but serializes for interactive editing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Draft {
    pub trust: TrustLevel,
    #[serde(default = "Default::default", skip_serializing_if = "String::is_empty")]
    comment: String,
}

impl From<Trust> for Draft {
    fn from(trust: Trust) -> Self {
        Draft {
            trust: trust.trust,
            comment: trust.comment,
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
    fn serialize_to(&self, fmt: &mut dyn std::fmt::Write) -> Result<()> {
        serde_content_serialize!(self, fmt);
        Ok(())
    }

    fn validate_data(&self) -> Result<()> {
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
        let draft = Draft::parse(&s)?;

        let mut copy = self.clone();
        copy.trust = draft.trust;
        copy.comment = draft.comment;

        copy.validate_data()?;
        Ok(copy)
    }
}

impl Draft {
    pub fn parse(s: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(&s)?)
    }
}
