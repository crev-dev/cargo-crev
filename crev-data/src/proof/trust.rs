use self::super::content;
use crate::{id, proof, Level, Result};
use chrono::{self, prelude::*};
use crev_common::{
    self,
    serde::{as_rfc3339_fixed, from_rfc3339_fixed},
};
use derive_builder::Builder;
use failure::bail;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::fmt;

const BEGIN_BLOCK: &str = "-----BEGIN CREV TRUST -----";
const BEGIN_SIGNATURE: &str = "-----BEGIN CREV TRUST SIGNATURE-----";
const END_BLOCK: &str = "-----END CREV TRUST-----";

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
    #[builder(default = "cur_version()")]
    version: i64,
    #[builder(default = "crev_common::now()")]
    #[serde(
        serialize_with = "as_rfc3339_fixed",
        deserialize_with = "from_rfc3339_fixed"
    )]
    pub date: chrono::DateTime<FixedOffset>,
    pub from: crate::PubId,
    pub ids: Vec<crate::PubId>,
    #[builder(default = "Default::default()")]
    pub trust: TrustLevel,
    #[serde(skip_serializing_if = "String::is_empty", default = "Default::default")]
    #[builder(default = "Default::default()")]
    comment: String,
}

impl Trust {
    pub fn apply_draft(&self, draft: TrustDraft) -> Trust {
        let mut copy = self.clone();
        copy.trust = draft.trust;
        copy.comment = draft.comment;
        copy
    }
}

/// Like `Trust` but serializes for interactive editing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrustDraft {
    pub trust: TrustLevel,
    #[serde(default = "Default::default")]
    comment: String,
}

impl From<Trust> for TrustDraft {
    fn from(trust: Trust) -> Self {
        TrustDraft {
            trust: trust.trust,
            comment: trust.comment,
        }
    }
}

impl fmt::Display for Trust {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crev_common::serde::write_as_headerless_yaml(self, f)
    }
}

impl fmt::Display for TrustDraft {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crev_common::serde::write_as_headerless_yaml(self, f)
    }
}

impl Trust {
    pub(crate) const BEGIN_BLOCK: &'static str = BEGIN_BLOCK;
    pub(crate) const BEGIN_SIGNATURE: &'static str = BEGIN_SIGNATURE;
    pub(crate) const END_BLOCK: &'static str = END_BLOCK;
}

impl proof::content::ContentCommon for Trust {
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
        match self.ids.len() {
            0 => "Trust for noone?!".into(),
            1 => format!("Trust for {} {}", self.ids[0].id, self.ids[0].url.url),
            n => format!(
                "Trust for {} {} and {} other",
                self.ids[0].id,
                self.ids[0].url.url,
                n - 1
            ),
        }
    }

    fn validate_data(&self) -> Result<()> {
        Ok(())
    }

    fn parse(s: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(&s)?)
    }

    fn parse_draft(&self, s: &str) -> Result<Self> {
        let proof: Trust = self.apply_draft(TrustDraft::parse(&s)?).into();
        proof.validate_data()?;
        Ok(proof)
    }

    fn proof_type(&self) -> proof::ProofType {
        proof::ProofType::Trust
    }

    fn to_draft_string(&self) -> String {
        TrustDraft::from(self.clone()).to_string()
    }
}

impl Trust {
    pub fn sign_by(self, id: &id::OwnId) -> Result<proof::Proof> {
        content::Content::from(self).sign_by(id)
    }
}

impl TrustDraft {
    pub fn parse(s: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(&s)?)
    }
}
