use crate::{proof, proof::Proof, Error, ParseError, Result};
use chrono::{self, prelude::*};
use crev_common::{
    self,
    serde::{as_base64, as_rfc3339_fixed, from_base64, from_rfc3339_fixed},
};
use derive_builder::Builder;
use serde::{self, Deserialize, Serialize};
use std::{fmt, io};

pub type Date = chrono::DateTime<FixedOffset>;

/// Common operations on types containing `Common`
pub trait CommonOps {
    // until we support legacy, we have to stick to `Common` here
    fn common(&self) -> &Common;

    fn kind(&self) -> &str {
        self.common()
            .kind
            .as_ref()
            .expect("Common types are expected to always have the `kind` field backfilled")
    }

    fn from(&self) -> &crate::PublicId {
        &self.common().from
    }

    fn date(&self) -> &chrono::DateTime<chrono::offset::FixedOffset> {
        &self.common().date
    }

    fn date_utc(&self) -> chrono::DateTime<Utc> {
        self.date().with_timezone(&Utc)
    }

    fn author_id(&self) -> &crate::Id {
        &self.common().from.id
    }

    fn author_public_id(&self) -> &crate::PublicId {
        &self.common().from
    }

    fn ensure_kind_is(&self, kind: &str) -> ValidationResult<()> {
        let expected = self.kind();
        if expected != kind {
            return Err(ValidationError::InvalidKind(Box::new((
                expected.to_string(),
                kind.to_string(),
            ))));
        }
        Ok(())
    }
}

/// Reference to original proof when reissuing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OriginalReference {
    /// original proof digest (blake2b256)
    #[serde(serialize_with = "as_base64", deserialize_with = "from_base64")]
    pub proof: Vec<u8>,
    #[serde(skip_serializing_if = "String::is_empty", default = "Default::default")]
    pub comment: String,
}

/// A `Common` part of every `Content` format
#[derive(Clone, Builder, Debug, Serialize, Deserialize)]
pub struct Common {
    pub kind: Option<String>,
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
    pub from: crate::PublicId,
    /// Reference to original proof when reissuing
    #[serde(skip_serializing_if = "Option::is_none", default = "Option::default")]
    pub original: Option<OriginalReference>,
}

impl CommonOps for Common {
    fn common(&self) -> &Common {
        self
    }
}

pub trait WithReview {
    fn review(&self) -> &super::Review;
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Invalid kind: {}, expected: {}", _0.0, _0.1)]
    InvalidKind(Box<(String, String)>),

    #[error("Alternative source can't be empty")]
    AlternativeSourceCanNotBeEmpty,
    #[error("Alternative name can't be empty")]
    AlternativeNameCanNotBeEmpty,
    #[error("Issues with an empty `id` field are not allowed")]
    IssuesWithAnEmptyIDFieldAreNotAllowed,
    #[error("Advisories with no `id`s are not allowed")]
    AdvisoriesWithNoIDSAreNotAllowed,
    #[error("Advisories with an empty `id` field are not allowed")]
    AdvisoriesWithAnEmptyIDFieldAreNotAllowed,
}

pub type ValidationResult<T> = std::result::Result<T, ValidationError>;

/// Proof Content
///
/// `Content` is a standardized format of a crev proof body
/// (part that is being signed over).
///
/// It is open-ended, and different software
/// can implement their own formats.
pub trait Content: CommonOps {
    fn validate_data(&self) -> ValidationResult<()> {
        // typically just OK
        Ok(())
    }

    fn serialize_to(&self, fmt: &mut dyn std::fmt::Write) -> fmt::Result;
}

pub trait ContentDeserialize: Content + Sized {
    fn deserialize_from<IO>(io: IO) -> std::result::Result<Self, Error>
    where
        IO: io::Read;
}

impl<T> ContentDeserialize for T
where
    T: serde::de::DeserializeOwned + Content + Sized,
{
    fn deserialize_from<IO>(io: IO) -> std::result::Result<Self, Error>
    where
        IO: io::Read,
    {
        let s: Self = serde_yaml::from_reader(io).map_err(ParseError::Proof)?;

        s.validate_data()?;

        Ok(s)
    }
}

/// A Proof Content `Draft`
///
/// A simplified version of content, used
/// for user interaction - editing the parts
/// that are not neccessary for the user to see.
pub struct Draft {
    pub(crate) title: String,
    pub(crate) body: String,
}

impl Draft {
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    #[must_use]
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
        self.serialize_to(&mut body)
            .map_err(|e| crate::Error::YAMLFormat(e.to_string().into()))?;
        Ok(body)
    }

    fn sign_by(&self, id: &crate::id::UnlockedId) -> Result<Proof> {
        let body = self.serialize()?;
        let signature = id.sign(body.as_bytes());
        Ok(Proof {
            digest: crev_common::blake2b256sum(body.as_bytes()),
            body,
            signature: crev_common::base64_encode(&signature),
            common_content: self.common().clone(),
        })
    }

    /// Ensure the proof generated from this `Content` is going to deserialize
    fn ensure_serializes_to_valid_proof(&self) -> Result<()> {
        let body = self.serialize()?;
        let signature = "somefakesignature";
        let proof = proof::Proof {
            digest: crev_common::blake2b256sum(body.as_bytes()),
            body,
            signature: crev_common::base64_encode(&signature),
            common_content: self.common().clone(),
        };
        let parsed = proof::Proof::parse_from(std::io::Cursor::new(proof.to_string().as_bytes()))?;

        if parsed.len() != 1 {
            return Err(Error::SerializedTooManyProofs(parsed.len()));
        }

        Ok(())
    }
}

impl<T> ContentExt for T where T: Content {}
