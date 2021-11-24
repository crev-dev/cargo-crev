//! Some common stuff for both Review and Trust Proofs

pub use crate::proof::content::{
    Common, CommonOps, Content, ContentDeserialize, ContentExt, ContentWithDraft, Draft, WithReview,
};
use crate::{Error, ParseError, Result};
use chrono::{self, prelude::*};
pub use package_info::*;
pub use review::{Code as CodeReview, Package as PackageReview, *};
pub use revision::*;
use std::{
    default, fmt,
    io::{self, BufRead},
};
pub use trust::*;

pub mod content;
pub mod package_info;
pub mod review;
pub mod revision;
pub mod trust;

const MAX_PROOF_BODY_LENGTH: usize = 32_000;

pub type Date = chrono::DateTime<FixedOffset>;
pub type DateUtc = chrono::DateTime<Utc>;

/// Serialized Proof
///
/// A signed proof containing some signed `Content`
#[derive(Debug, Clone)]
pub struct Proof {
    /// Serialized content
    body: String,

    /// Signature over the body
    signature: String,

    /// Common informations that should be in any  proof
    common_content: Common,

    /// Digest (blake2b256)
    digest: [u8; 32],
}

impl Proof {
    pub fn from_parts(body: String, signature: String) -> Result<Self> {
        let common_content: Common = serde_yaml::from_str(&body).map_err(ParseError::Proof)?;
        if common_content.kind.is_none() {
            return Err(Error::KindFieldMissing);
        }
        let digest = crev_common::blake2b256sum(body.as_bytes());
        let signature = signature.trim().to_owned();
        Ok(Self {
            body,
            signature,
            common_content,
            digest,
        })
    }

    pub fn from_legacy_parts(body: String, signature: String, type_name: String) -> Result<Self> {
        #[allow(deprecated)]
        let mut legacy_common_content: content::Common =
            serde_yaml::from_str(&body).map_err(ParseError::Proof)?;
        if legacy_common_content.kind.is_some() {
            return Err(Error::UnexpectedKindValueInALegacyFormat);
        }

        legacy_common_content.kind = Some(type_name);
        let digest = crev_common::blake2b256sum(body.as_bytes());
        let signature = signature.trim().to_owned();
        Ok(Self {
            body,
            signature,
            common_content: legacy_common_content,
            digest,
        })
    }
    pub fn body(&self) -> &str {
        self.body.as_str()
    }

    pub fn signature(&self) -> &str {
        self.signature.as_str()
    }

    pub fn digest(&self) -> &[u8; 32] {
        &self.digest
    }

    pub fn parse_content<T: ContentDeserialize>(&self) -> std::result::Result<T, Error> {
        T::deserialize_from(self.body.as_bytes())
    }
}

impl CommonOps for Proof {
    fn common(&self) -> &Common {
        &self.common_content
    }
}

const PROOF_START: &str = "----- BEGIN CREV PROOF -----";
const PROOF_SIGNATURE: &str = "----- SIGN CREV PROOF -----";
const PROOF_END: &str = "----- END CREV PROOF -----";

const LEGACY_PROOF_START_PREFIX: &str = "-----BEGIN CREV ";
const LEGACY_PROOF_START_SUFFIX: &str = "-----";
// There was a bug ... :D ... https://github.com/dpc/crev-proofs/blob/3ea7e440f1ed84f5a333741e71a90e2067fe9cfc/FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE/trust/2019-10-GkN7aw.proof.crev#L1
const LEGACY_PROOF_START_SUFFIX_ALT: &str = " -----";
const LEGACY_PROOF_SIGNATURE_PREFIX: &str = "-----BEGIN CREV ";
const LEGACY_PROOF_SIGNATURE_SUFFIX: &str = " SIGNATURE-----";
const LEGACY_PROOF_END_PREFIX: &str = "-----END CREV ";
const LEGACY_PROOF_END_SUFFIX: &str = "-----";

fn is_start_line(line: &str) -> bool {
    line.trim() == PROOF_START
}

fn is_signature_line(line: &str) -> bool {
    line.trim() == PROOF_SIGNATURE
}

fn is_end_line(line: &str) -> bool {
    line.trim() == PROOF_END
}

fn is_legacy_start_line(line: &str) -> Option<String> {
    let trimmed = line.trim();

    if trimmed.starts_with(LEGACY_PROOF_START_PREFIX)
        && trimmed.ends_with(LEGACY_PROOF_START_SUFFIX_ALT)
    {
        let type_name = &trimmed[LEGACY_PROOF_START_PREFIX.len()..];
        let type_name = &type_name[..(type_name.len() - LEGACY_PROOF_START_SUFFIX_ALT.len())];

        Some(type_name.to_lowercase())
    } else if trimmed.starts_with(LEGACY_PROOF_START_PREFIX)
        && trimmed.ends_with(LEGACY_PROOF_START_SUFFIX)
    {
        let type_name = &trimmed[LEGACY_PROOF_START_PREFIX.len()..];
        let type_name = &type_name[..(type_name.len() - LEGACY_PROOF_START_SUFFIX.len())];

        Some(type_name.to_lowercase())
    } else {
        None
    }
}

fn is_legacy_signature_line(line: &str) -> Option<String> {
    let trimmed = line.trim();

    if trimmed.starts_with(LEGACY_PROOF_SIGNATURE_PREFIX)
        && trimmed.ends_with(LEGACY_PROOF_SIGNATURE_SUFFIX)
    {
        let type_name = &trimmed[LEGACY_PROOF_SIGNATURE_PREFIX.len()..];
        let type_name = &type_name[..(type_name.len() - LEGACY_PROOF_SIGNATURE_SUFFIX.len())];

        Some(type_name.to_lowercase())
    } else {
        None
    }
}

fn is_legacy_end_line(line: &str) -> Option<String> {
    let trimmed = line.trim();

    if trimmed.starts_with(LEGACY_PROOF_END_PREFIX) && trimmed.ends_with(LEGACY_PROOF_END_SUFFIX) {
        let type_name = &trimmed[LEGACY_PROOF_END_PREFIX.len()..];
        let type_name = &type_name[..(type_name.len() - LEGACY_PROOF_END_SUFFIX.len())];

        Some(type_name.to_lowercase())
    } else {
        None
    }
}

impl fmt::Display for Proof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(PROOF_START)?;
        f.write_str("\n")?;
        f.write_str(&self.body)?;
        f.write_str(PROOF_SIGNATURE)?;
        f.write_str("\n")?;
        f.write_str(&self.signature)?;
        f.write_str("\n")?;
        f.write_str(PROOF_END)?;
        f.write_str("\n")?;

        Ok(())
    }
}

impl Proof {
    pub fn parse_from(reader: impl io::Read) -> Result<Vec<Self>> {
        let reader = std::io::BufReader::new(reader);

        #[derive(PartialEq, Eq)]
        enum Stage {
            None,
            Body,
            Signature,
        }

        impl Default for Stage {
            fn default() -> Self {
                Stage::None
            }
        }

        struct State {
            stage: Stage,
            body: String,
            signature: String,
            type_name: Option<String>,
            proofs: Vec<Proof>,
        }

        impl default::Default for State {
            fn default() -> Self {
                State {
                    stage: Default::default(),
                    body: Default::default(),
                    signature: Default::default(),
                    type_name: None,
                    proofs: vec![],
                }
            }
        }

        impl State {
            fn process_line(&mut self, line: &str) -> Result<()> {
                match self.stage {
                    Stage::None => {
                        let line = line.trim();
                        if line.is_empty() {
                        } else if let Some(type_name) = is_legacy_start_line(line) {
                            self.type_name = Some(type_name);
                            self.stage = Stage::Body;
                        } else if is_start_line(line) {
                            assert!(self.type_name.is_none());
                            self.stage = Stage::Body;
                        } else {
                            return Err(Error::ParsingErrorWhenLookingForStartOfCodeReviewProof);
                        }
                    }
                    Stage::Body => {
                        if self.type_name.is_some() {
                            if let Some(type_name) = is_legacy_signature_line(line) {
                                if Some(type_name) != self.type_name {
                                    return Err(Error::ParsingErrorTypeNameMismatchInTheSignature);
                                }
                                self.stage = Stage::Signature;
                            } else {
                                self.body += line;
                                self.body += "\n";
                            }
                        } else if is_signature_line(line) {
                            self.stage = Stage::Signature;
                        } else {
                            self.body += line;
                            self.body += "\n";
                        }
                        if self.body.len() > MAX_PROOF_BODY_LENGTH {
                            return Err(Error::ProofBodyTooLong);
                        }
                    }
                    Stage::Signature => {
                        if self.type_name.is_some() {
                            if let Some(type_name) = is_legacy_end_line(line) {
                                if Some(&type_name) != self.type_name.as_ref() {
                                    return Err(Error::ParsingErrorTypeNameMismatchInTheFooter);
                                }
                                self.stage = Stage::None;
                                self.type_name = None;
                                self.proofs.push(Proof::from_legacy_parts(
                                    std::mem::take(&mut self.body),
                                    std::mem::take(&mut self.signature),
                                    type_name,
                                )?);
                            } else {
                                self.signature += line;
                                self.signature += "\n";
                            }
                        } else if is_end_line(line) {
                            self.stage = Stage::None;
                            self.proofs.push(Proof::from_parts(
                                std::mem::take(&mut self.body),
                                std::mem::take(&mut self.signature),
                            )?);
                        } else {
                            self.signature += line;
                            self.signature += "\n";
                        }

                        if self.signature.len() > 2000 {
                            return Err(Error::SignatureTooLong);
                        }
                    }
                }
                Ok(())
            }

            fn finish(self) -> Result<Vec<Proof>> {
                if self.stage != Stage::None {
                    return Err(Error::UnexpectedEOFWhileParsing);
                }
                Ok(self.proofs)
            }
        }

        let mut state: State = Default::default();

        for line in reader.lines() {
            state.process_line(&line?)?;
        }

        state.finish()
    }

    pub fn verify(&self) -> Result<()> {
        let pubkey = &self.from().id;
        pubkey.verify_signature(self.body.as_bytes(), self.signature())?;

        Ok(())
    }
}

fn equals_default_digest_type(s: &str) -> bool {
    s == default_digest_type()
}

pub fn default_digest_type() -> String {
    "blake2b".into()
}

fn equals_default_revision_type(s: &str) -> bool {
    s == default_revision_type()
}

pub fn default_revision_type() -> String {
    "git".into()
}

fn equals_default<T: Default + PartialEq>(t: &T) -> bool {
    *t == Default::default()
}
