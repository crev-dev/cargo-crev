use chrono::{self, prelude::*};
use failure::bail;

use crate::proof;
use crate::proof::Proof;
use crate::Result;
use crev_common::{
    self,
    serde::{as_rfc3339_fixed, from_rfc3339_fixed},
};
use derive_builder::Builder;
use serde::{self, Deserialize, Serialize};
use std::io;

pub type Date = chrono::DateTime<FixedOffset>;

/// A `Common` part of every `Content` format
#[derive(Clone, Builder, Debug, Serialize, Deserialize)]
pub struct Common {
    /// A version, to allow future backward-incompatible extensions
    /// and changes.
    pub version: i64,
    #[builder(default = "crev_common::now()")]
    #[serde(
        serialize_with = "as_rfc3339_fixed",
        deserialize_with = "from_rfc3339_fixed"
    )]
    /// Timestamp of proof creation
    pub date: chrono::DateTime<FixedOffset>,
    /// Author of the proof
    pub from: crate::PubId,
}

/// Proof Content
///
/// `Content` is a standardized format of a crev proof body
/// (part that is being signed over).
///
/// It is open-ended, and different software
/// can implement their own formats.
pub trait Content {
    const TYPE_NAME: &'static str;

    fn type_name(&self) -> &str {
        &Self::TYPE_NAME
    }

    fn common(&self) -> &Common;

    fn validate_data(&self) -> Result<()> {
        // typically just OK
        Ok(())
    }

    fn serialize_to(&self, fmt: &mut dyn std::fmt::Write) -> Result<()>;
}

/// A Proof Content `Draft`
///
/// A simplified version of content, used
/// for user interaction - editing the parts
/// that are not neccessary for the user to see.
pub struct Draft {
    title: String,
    body: String,
}

impl Draft {
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn body(&self) -> &str {
        &self.body
    }
}

/// A content with draft support
///
/// Draft is a compact, human
pub trait ContentWithDraft: Content {
    fn to_draft(&self) -> Draft;

    fn apply_draft(&self, body: &str) -> Result<Self>
    where
        Self: Sized;
}

pub trait ContentExt: Content {
    fn serialize(&self) -> Result<String> {
        let mut body = String::new();
        self.serialize_to(&mut body)?;
        Ok(body)
    }

    fn sign_by(&self, id: &crate::id::OwnId) -> Result<Proof> {
        let body = self.serialize()?;
        let signature = id.sign(&body.as_bytes());
        Ok(Proof {
            digest: crev_common::blake2b256sum(body.as_bytes()),
            body,
            signature: crev_common::base64_encode(&signature),
            common_content: self.common().clone(),
            type_name: self.type_name().to_owned(),
        })
    }

    /// Ensure the proof generated from this `Content` is going to deserialize
    fn ensure_serializes_to_valid_proof(&self) -> Result<()> {
        let body = self.serialize()?;
        let signature = "somefakesignature";
        let proof = proof::Proof {
            digest: crev_common::blake2b256sum(&body.as_bytes()),
            body,
            signature: crev_common::base64_encode(&signature),
            type_name: self.type_name().to_owned(),
            common_content: self.common().to_owned(),
        };
        let parsed = proof::Proof::parse(std::io::Cursor::new(proof.to_string().as_bytes()))?;

        if parsed.len() != 1 {
            bail!("Serialized to {} proofs", parsed.len());
        }

        Ok(())
    }

    fn deserialize_from<T>(io: &mut T) -> Result<Self>
    where
        T: io::Read,
        Self: Sized,
        Self: serde::de::DeserializeOwned + Content,
    {
        let s: Self = serde_yaml::from_reader(io)?;

        s.validate_data()?;

        Ok(s)
    }
}

impl<T> ContentExt for T where T: Content {}
