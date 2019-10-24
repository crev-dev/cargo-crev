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

pub use self::{package_info::*, revision::*, trust::*};
pub use crate::proof::content::Common as ContentCommon;
pub use crate::proof::content::{Content, ContentExt, ContentWithDraft, Draft};
pub use review::*;

use crate::Result;

const MAX_PROOF_BODY_LENGTH: usize = 32_000;

pub type Date = chrono::DateTime<FixedOffset>;

/// Serialized Proof
///
/// A signed proof containing some signed `Content`
#[derive(Debug, Clone)]
pub(crate) struct Proof {
    /// Serialized content
    pub body: String,
    /// Signature over the body
    pub signature: String,
    /// Type of the `body` (`Content`)
    pub type_name: String,

    /// Common informations that should be in any  proof
    pub common_content: ContentCommon,

    /// Digest, lazily calculated
    pub digest: Vec<u8>,
}

const PROOF_HEADER_PREFIX: &str = "-----BEGIN CREV ";
const PROOF_HEADER_SUFFIX: &str = "-----";
const PROOF_SIGNATURE_PREFIX: &str = "-----BEGIN CREV PACKAGE ";
const PROOF_SIGNATURE_SUFFIX: &str = "-----";
const PROOF_FOOTER_PREFIX: &str = "-----END CREV ";
const PROOF_FOOTER_SUFFIX: &str = "-----";

fn is_header_line(line: &str) -> Option<String> {
    let trimmed = line.trim();

    if trimmed.starts_with(PROOF_HEADER_PREFIX) && trimmed.ends_with(PROOF_HEADER_SUFFIX) {
        let type_name = &trimmed[PROOF_HEADER_PREFIX.len()..];
        let type_name = &type_name[..(type_name.len() - PROOF_HEADER_SUFFIX.len())];

        Some(type_name.to_lowercase())
    } else {
        None
    }
}

fn is_signature_line(line: &str) -> Option<String> {
    let trimmed = line.trim();

    if trimmed.starts_with(PROOF_SIGNATURE_PREFIX) && trimmed.ends_with(PROOF_SIGNATURE_SUFFIX) {
        let type_name = &trimmed[PROOF_SIGNATURE_PREFIX.len()..];
        let type_name = &type_name[..(type_name.len() - PROOF_SIGNATURE_SUFFIX.len())];

        Some(type_name.to_lowercase())
    } else {
        None
    }
}

fn is_footer_line(line: &str) -> Option<String> {
    let trimmed = line.trim();

    if trimmed.starts_with(PROOF_FOOTER_PREFIX) && trimmed.ends_with(PROOF_FOOTER_SUFFIX) {
        let type_name = &trimmed[PROOF_FOOTER_PREFIX.len()..];
        let type_name = &type_name[..(type_name.len() - PROOF_FOOTER_SUFFIX.len())];

        Some(type_name.to_lowercase())
    } else {
        None
    }
}

impl fmt::Display for Proof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let type_upper = self.type_name.to_uppercase();
        f.write_fmt(format_args!(
            "{}{}{}",
            PROOF_HEADER_PREFIX, type_upper, PROOF_HEADER_SUFFIX
        ));
        f.write_str("\n")?;
        f.write_str(&self.body)?;
        f.write_fmt(format_args!(
            "{}{}{}",
            PROOF_SIGNATURE_PREFIX, type_upper, PROOF_SIGNATURE_SUFFIX
        ))?;
        f.write_str("\n")?;
        f.write_str(&self.signature)?;
        f.write_str("\n")?;
        f.write_fmt(format_args!(
            "{}{}{}",
            PROOF_FOOTER_PREFIX, type_upper, PROOF_FOOTER_SUFFIX
        ))?;
        f.write_str("\n")?;

        Ok(())
    }
}

impl Proof {
    pub fn parse(reader: impl io::Read) -> Result<Vec<Self>> {
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
                        } else if let Some(type_name) = is_header_line(line) {
                            self.type_name = Some(type_name);
                        } else {
                            bail!("Parsing error when looking for start of code review proof");
                        }
                    }
                    Stage::Body => {
                        if let Some(type_name) = is_signature_line(line) {
                            if Some(type_name) != self.type_name {
                                bail!("Parsing error: type name mismatch in the signature");
                            }
                            self.stage = Stage::Signature;
                        } else {
                            self.body += line;
                            self.body += "\n";
                        }
                        if self.body.len() > MAX_PROOF_BODY_LENGTH {
                            bail!("Proof body too long");
                        }
                    }
                    Stage::Signature => {
                        if let Some(type_name) = is_footer_line(line) {
                            if Some(type_name) != self.type_name {
                                bail!("Parsing error: type name mismatch in the footer");
                            }
                            self.stage = Stage::None;
                            self.type_name = None;
                            let common_content = serde_yaml::from_str(&self.body)?;
                            let digest = crev_common::blake2b256sum(&self.body.as_bytes());
                            self.proofs.push(Proof {
                                body: mem::replace(&mut self.body, String::new()),
                                signature: mem::replace(&mut self.signature, String::new()),
                                type_name,
                                common_content,
                                digest,
                            });
                        } else {
                            self.signature += line;
                            self.signature += "\n";
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

    pub fn signature(&self) -> &str {
        self.signature.trim()
    }

    pub fn digest(&self) -> &[u8] {
        self.digest.as_slice()
    }

    pub fn verify(&self) -> Result<()> {
        let pubkey = self.common_content.from.id;
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
