use crate::Url;
use chrono::{self, prelude::*};
use failure::bail;
use std::fmt;

use self::super::review;
use self::super::trust::{Trust, TrustDraft};
use crate::proof;
use crate::proof::{Proof, ProofType};
use crate::Result;

pub type Date = chrono::DateTime<FixedOffset>;

pub trait ContentCommon {
    fn date(&self) -> &Date;
    fn set_date(&mut self, date: &Date);
    fn date_utc(&self) -> chrono::DateTime<Utc> {
        self.date().with_timezone(&Utc)
    }

    fn author(&self) -> &crate::PubId;
    fn set_author(&mut self, id: &crate::PubId);

    fn author_id(&self) -> crate::Id {
        self.author().id.clone()
    }

    fn author_url(&self) -> Url {
        self.author().url.clone()
    }

    fn draft_title(&self) -> String;
    fn validate_data(&self) -> Result<()>;

    fn parse(s: &str) -> Result<Self>
    where
        Self: Sized;

    fn parse_draft(&self, s: &str) -> Result<Self>
    where
        Self: Sized;

    fn proof_type(&self) -> ProofType;
}

/// Content is an enumerator of possible proof contents
#[derive(Debug, Clone)]
pub enum Content {
    Trust(Trust),
    Package(Box<review::Package>),
    Code(Box<review::Code>),
}

impl fmt::Display for Content {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Content::*;
        match self {
            Trust(trust) => trust.fmt(f),
            Code(code) => code.fmt(f),
            Package(package) => package.fmt(f),
        }
    }
}

impl From<review::Code> for Content {
    fn from(review: review::Code) -> Self {
        Content::Code(Box::new(review))
    }
}

impl From<review::Package> for Content {
    fn from(review: review::Package) -> Self {
        Content::Package(Box::new(review))
    }
}

impl From<Trust> for Content {
    fn from(review: Trust) -> Self {
        Content::Trust(review)
    }
}

impl Content {
    pub fn draft_title(&self) -> String {
        use self::Content::*;
        match self {
            Trust(trust) => trust.draft_title(),
            Code(review) => review.draft_title(),
            Package(review) => review.draft_title(),
        }
    }

    pub fn validate_data(&self) -> Result<()> {
        use self::Content::*;
        if let Package(review) = self {
            review.validate_data()?
        }

        Ok(())
    }

    pub fn parse(s: &str, type_: ProofType) -> Result<Content> {
        Ok(match type_ {
            ProofType::Code => review::Code::parse(&s)?.into(),
            ProofType::Package => review::Package::parse(&s)?.into(),
            ProofType::Trust => Trust::parse(&s)?.into(),
        })
    }

    pub fn parse_draft(original_proof: &Content, s: &str) -> Result<Content> {
        let proof: Content = match original_proof {
            Content::Code(code) => code.apply_draft(review::CodeDraft::parse(&s)?).into(),
            Content::Package(package) => {
                package.apply_draft(review::PackageDraft::parse(&s)?).into()
            }
            Content::Trust(trust) => trust.apply_draft(TrustDraft::parse(&s)?).into(),
        };
        proof.validate_data()?;
        Ok(proof)
    }

    pub fn sign_by(&self, id: &crate::id::OwnId) -> Result<Proof> {
        let body = self.to_string();
        let signature = id.sign(&body.as_bytes());
        Ok(Proof {
            digest: crev_common::blake2b256sum(&body.as_bytes()),
            body,
            signature: crev_common::base64_encode(&signature),
            content: self.clone(),
        })
    }

    pub fn proof_type(&self) -> ProofType {
        use self::Content::*;
        match self {
            Trust(_trust) => ProofType::Trust,
            Code(_review) => ProofType::Code,
            Package(_review) => ProofType::Package,
        }
    }

    pub fn date(&self) -> &Date {
        use self::Content::*;
        match self {
            Trust(trust) => trust.date(),
            Code(review) => review.date(),
            Package(review) => review.date(),
        }
    }

    pub fn author_id(&self) -> crate::Id {
        use self::Content::*;
        match self {
            Trust(trust) => trust.author_id(),
            Code(review) => review.author_id(),
            Package(review) => review.author_id(),
        }
    }

    pub fn set_author(&mut self, id: &crate::PubId) {
        use self::Content::*;
        match self {
            Trust(trust) => trust.set_author(id),
            Code(review) => review.set_author(id),
            Package(review) => review.set_author(id),
        }
    }

    pub fn set_date(&mut self, date: &Date) {
        use self::Content::*;
        match self {
            Trust(trust) => trust.set_date(date),
            Code(review) => review.set_date(date),
            Package(review) => review.set_date(date),
        }
    }

    pub fn author_url(&self) -> Url {
        use self::Content::*;
        match self {
            Trust(trust) => trust.author_url(),
            Code(review) => review.author_url(),
            Package(review) => review.author_url(),
        }
    }

    pub fn to_draft_string(&self) -> String {
        use self::Content::*;
        match self.clone() {
            Trust(trust) => TrustDraft::from(trust).to_string(),
            Code(review) => review::CodeDraft::from(*review).to_string(),
            Package(review) => review::PackageDraft::from(*review).to_string(),
        }
    }

    /// Ensure the proof generated from this `Content` is going to deserialize
    pub fn ensure_serializes_to_valid_proof(&self) -> Result<()> {
        let body = self.to_string();
        let signature = "somefakesignature";
        let proof = proof::Proof {
            digest: crev_common::blake2b256sum(&body.as_bytes()),
            body,
            signature: crev_common::base64_encode(&signature),
            content: self.clone(),
        };
        let parsed = proof::Proof::parse(std::io::Cursor::new(proof.to_string().as_bytes()))?;

        if parsed.len() != 1 {
            bail!("Serialized to {} proofs", parsed.len());
        }

        Ok(())
    }
}
