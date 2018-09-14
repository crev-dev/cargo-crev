//! Some common stuff for both Review and Trust Proofs

use base64;
use blake2;
use chrono::{self, prelude::*};
use crev_common;
use ed25519_dalek;
use id;
use std::{default, fmt, fs, io, mem, path::Path};

pub mod review;
pub mod trust;

pub use review::*;
pub use trust::*;

use Result;

#[derive(Copy, Clone, Debug)]
pub enum ProofType {
    Review,
    Trust,
}

impl ProofType {
    fn begin_block(&self) -> &'static str {
        match self {
            ProofType::Review => Review::BEGIN_BLOCK,
            ProofType::Trust => Trust::BEGIN_BLOCK,
        }
    }
    fn begin_signature(&self) -> &'static str {
        match self {
            ProofType::Review => Review::BEGIN_SIGNATURE,
            ProofType::Trust => Trust::BEGIN_SIGNATURE,
        }
    }
    fn end_block(&self) -> &'static str {
        match self {
            ProofType::Review => Review::END_BLOCK,
            ProofType::Trust => Trust::END_BLOCK,
        }
    }
}

/// A signed proof containing some signed `Content`
#[derive(Debug, Clone)]
pub(crate) struct Serialized {
    pub body: String,
    pub signature: String,
    pub type_: ProofType,
}

#[derive(Debug, Clone)]
pub enum Content {
    Trust(Trust),
    Review(Review),
}

impl fmt::Display for Content {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Content::*;
        match self {
            Trust(trust) => trust.fmt(f),
            Review(review) => review.fmt(f),
        }
    }
}

impl From<Review> for Content {
    fn from(review: Review) -> Self {
        Content::Review(review)
    }
}

impl From<Trust> for Content {
    fn from(review: Trust) -> Self {
        Content::Trust(review)
    }
}

impl Content {
    pub fn parse(s: &str, type_: ProofType) -> Result<Content> {
        Ok(match type_ {
            ProofType::Review => Content::Review(Review::parse(&s)?),
            ProofType::Trust => Content::Trust(Trust::parse(&s)?),
        })
    }

    pub fn sign(&self, id: &id::OwnId) -> Result<Proof> {
        let body = self.to_string();
        let signature = id.sign(&body.as_bytes());
        Ok(Proof {
            digest: crev_common::blake2sum(&body.as_bytes()),
            body: body,
            signature: base64::encode_config(&signature, base64::URL_SAFE),
            content: self.clone(),
        })
    }

    pub fn proof_type(&self) -> ProofType {
        use self::Content::*;
        match self {
            Trust(_trust) => ProofType::Trust,
            Review(_review) => ProofType::Review,
        }
    }

    pub fn date(&self) -> chrono::DateTime<FixedOffset> {
        use self::Content::*;
        match self {
            Trust(trust) => trust.date(),
            Review(review) => review.date(),
        }
    }

    pub fn from_pubid(&self) -> String {
        use self::Content::*;
        match self {
            Trust(trust) => trust.from_pubid(),
            Review(review) => review.from_pubid(),
        }
    }

    pub fn from_url(&self) -> String {
        use self::Content::*;
        match self {
            Trust(trust) => trust.from_url(),
            Review(review) => review.from_url(),
        }
    }

    pub fn project_id(&self) -> Option<&str> {
        use self::Content::*;
        match self {
            Trust(trust) => trust.project_id(),
            Review(review) => review.project_id(),
        }
    }
}

#[derive(Debug, Clone)]
/// A `Proof` with it's content parsed and ready.
pub struct Proof {
    pub body: String,
    pub signature: String,
    pub digest: Vec<u8>,
    pub content: Content,
}

impl fmt::Display for Serialized {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.type_.begin_block())?;
        f.write_str("\n")?;
        f.write_str(&self.body)?;
        f.write_str(self.type_.begin_signature())?;
        f.write_str("\n")?;
        f.write_str(&self.signature)?;
        f.write_str("\n")?;
        f.write_str(self.type_.end_block())?;
        f.write_str("\n")?;

        Ok(())
    }
}

impl fmt::Display for Proof {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.content.proof_type().begin_block())?;
        f.write_str("\n")?;
        f.write_str(&self.body)?;
        f.write_str(self.content.proof_type().begin_signature())?;
        f.write_str("\n")?;
        f.write_str(&self.signature)?;
        f.write_str("\n")?;
        f.write_str(self.content.proof_type().end_block())?;
        f.write_str("\n")?;

        Ok(())
    }
}

impl Serialized {
    pub fn to_parsed(&self) -> Result<Proof> {
        Ok(Proof {
            body: self.body.clone(),
            signature: self.signature.clone(),
            digest: crev_common::blake2sum(&self.body.as_bytes()),
            content: match self.type_ {
                ProofType::Review => Content::Review(Review::parse(&self.body)?),
                ProofType::Trust => Content::Trust(Trust::parse(&self.body)?),
            },
        })
    }

    pub fn parse(reader: impl io::BufRead) -> Result<Vec<Self>> {
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
            type_: ProofType,
            proofs: Vec<Serialized>,
        }

        impl default::Default for State {
            fn default() -> Self {
                State {
                    stage: Default::default(),
                    body: Default::default(),
                    signature: Default::default(),
                    type_: ProofType::Trust, // whatever
                    proofs: vec![],
                }
            }
        }

        impl State {
            fn process_line(&mut self, line: &str) -> Result<()> {
                match self.stage {
                    Stage::None => {
                        if line.trim().is_empty() {
                        } else if line.trim() == ProofType::Review.begin_block() {
                            self.type_ = ProofType::Review;
                            self.stage = Stage::Body;
                        } else if line.trim() == ProofType::Trust.begin_block() {
                            self.type_ = ProofType::Trust;
                            self.stage = Stage::Body;
                        } else {
                            bail!("Parsing error when looking for start of code review proof");
                        }
                    }
                    Stage::Body => {
                        if line.trim() == self.type_.begin_signature() {
                            self.stage = Stage::Signature;
                        } else {
                            self.body += line;
                            self.body += "\n";
                        }
                        if self.body.len() > 16_000 {
                            bail!("Proof body too long");
                        }
                    }
                    Stage::Signature => {
                        if line.trim() == self.type_.end_block() {
                            self.stage = Stage::None;
                            self.proofs.push(Serialized {
                                body: mem::replace(&mut self.body, String::new()),
                                signature: mem::replace(&mut self.signature, String::new()),
                                type_: self.type_,
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

            fn finish(self) -> Result<Vec<Serialized>> {
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
}

impl Proof {
    pub fn parse_from(path: &Path) -> Result<Vec<Self>> {
        let file = fs::File::open(path)?;
        Self::parse(io::BufReader::new(file))
    }

    pub fn parse(reader: impl io::BufRead) -> Result<Vec<Self>> {
        let mut v = vec![];
        for serialized in Serialized::parse(reader)?.into_iter() {
            v.push(serialized.to_parsed()?)
        }
        Ok(v)
    }

    pub fn signature(&self) -> Result<Vec<u8>> {
        let sig = self.signature.trim();
        Ok(base64::decode_config(sig, base64::URL_SAFE)?)
    }

    pub fn verify(&self) -> Result<()> {
        let pubkey_str = self.content.from_pubid();
        let pubkey_bytes = base64::decode_config(&pubkey_str, base64::URL_SAFE)?;
        let pubkey = ed25519_dalek::PublicKey::from_bytes(&pubkey_bytes)?;

        let signature = ed25519_dalek::Signature::from_bytes(&self.signature()?)?;

        pubkey.verify::<blake2::Blake2b>(self.body.as_bytes(), &signature)?;

        Ok(())
    }
}

fn equals_crev(s: &str) -> bool {
    s == "crev"
}

fn default_crev_value() -> String {
    "crev".into()
}

fn equals_blake2b(s: &str) -> bool {
    s == "blake2b"
}

fn default_blake2b_value() -> String {
    "blake2b".into()
}
