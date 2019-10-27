//! Some common stuff for both Review and Trust Proofs

use chrono::{self, prelude::*};
use crev_common;
use failure::bail;
use std::{
    default, fmt,
    io::{self, BufRead},
    mem,
};

pub mod content;
pub mod package_info;
pub mod review;
pub mod revision;
pub mod trust;

pub use self::review::Code as CodeReview;
pub use self::review::Package as PackageReview;

pub use self::{package_info::*, revision::*, trust::*};
pub use crate::proof::content::{
    Common, CommonOps, Content, ContentDeserialize, ContentExt, ContentWithDraft, Draft, WithReview,
};
pub use review::*;

use crate::Result;

const MAX_PROOF_BODY_LENGTH: usize = 32_000;

pub type Date = chrono::DateTime<FixedOffset>;

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

    /// Digest
    digest: Vec<u8>,
}

impl Proof {
    pub fn from_parts(body: String, signature: String) -> Result<Self> {
        let common_content = serde_yaml::from_str(&body)?;
        let digest = crev_common::blake2b256sum(&body.as_bytes());
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
        let legacy_common_content: content::LegacyCommon = serde_yaml::from_str(&body)?;
        let digest = crev_common::blake2b256sum(&body.as_bytes());
        let signature = signature.trim().to_owned();
        Ok(Self {
            body,
            signature,
            common_content: legacy_common_content.into_common(type_name),
            digest,
        })
    }
    pub fn body(&self) -> &str {
        self.body.as_str()
    }

    pub fn signature(&self) -> &str {
        self.signature.as_str()
    }

    pub fn digest(&self) -> &[u8] {
        self.digest.as_slice()
    }

    pub fn parse_content<T: ContentDeserialize>(&self) -> Result<T> {
        Ok(T::deserialize_from(self.body.as_bytes())?)
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
                            bail!("Parsing error when looking for start of code review proof");
                        }
                    }
                    Stage::Body => {
                        if self.type_name.is_some() {
                            if let Some(type_name) = is_legacy_signature_line(line) {
                                if Some(type_name) != self.type_name {
                                    bail!("Parsing error: type name mismatch in the signature");
                                }
                                self.stage = Stage::Signature;
                            } else {
                                self.body += line;
                                self.body += "\n";
                            }
                        } else {
                            if is_signature_line(line) {
                                self.stage = Stage::Signature;
                            } else {
                                self.body += line;
                                self.body += "\n";
                            }
                        }
                        if self.body.len() > MAX_PROOF_BODY_LENGTH {
                            bail!("Proof body too long");
                        }
                    }
                    Stage::Signature => {
                        if self.type_name.is_some() {
                            if let Some(type_name) = is_legacy_end_line(line) {
                                if Some(&type_name) != self.type_name.as_ref() {
                                    bail!("Parsing error: type name mismatch in the footer");
                                }
                                self.stage = Stage::None;
                                self.type_name = None;
                                self.proofs.push(Proof::from_legacy_parts(
                                    mem::replace(&mut self.body, String::new()),
                                    mem::replace(&mut self.signature, String::new()),
                                    type_name,
                                )?);
                            } else {
                                self.signature += line;
                                self.signature += "\n";
                            }
                        } else {
                            if is_end_line(line) {
                                self.stage = Stage::None;
                                self.proofs.push(Proof::from_parts(
                                    mem::replace(&mut self.body, String::new()),
                                    mem::replace(&mut self.signature, String::new()),
                                )?);
                            } else {
                                self.signature += line;
                                self.signature += "\n";
                            }
                        }

                        if self.signature.len() > 2000 {
                            bail!("Signature too long");
                        }
                    }
                }
                Ok(())
            }

            fn finish(self) -> Result<Vec<Proof>> {
                if self.stage != Stage::None {
                    bail!("Unexpected EOF while parsing");
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
