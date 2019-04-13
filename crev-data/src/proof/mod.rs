//! Some common stuff for both Review and Trust Proofs

use crate::Url;
use chrono::{self, prelude::*};
use crev_common;
use std::io::BufRead;
use std::{default, fmt, fs, io, mem, path::Path};

pub mod package_info;
pub mod review;
pub mod revision;
pub mod trust;

pub use self::{package_info::*, revision::*, trust::*};

use crate::Result;

pub type Date = chrono::DateTime<FixedOffset>;

pub trait ContentCommon {
    fn date(&self) -> &Date;
    fn set_date(&mut self, date: &Date);

    fn author(&self) -> &crate::PubId;
    fn set_author(&mut self, id: &crate::PubId);

    fn date_utc(&self) -> chrono::DateTime<Utc> {
        self.date().with_timezone(&Utc)
    }

    fn author_id(&self) -> crate::Id {
        self.author().id.clone()
    }

    fn author_url(&self) -> Url {
        self.author().url.clone()
    }

    fn draft_title(&self) -> String;
}

#[derive(Copy, Clone, Debug)]
pub enum ProofType {
    Code,
    Package,
    Trust,
}

impl ProofType {
    fn begin_block(&self) -> &'static str {
        match self {
            ProofType::Code => review::Code::BEGIN_BLOCK,
            ProofType::Package => review::Package::BEGIN_BLOCK,
            ProofType::Trust => Trust::BEGIN_BLOCK,
        }
    }
    fn begin_signature(&self) -> &'static str {
        match self {
            ProofType::Code => review::Code::BEGIN_SIGNATURE,
            ProofType::Package => review::Package::BEGIN_SIGNATURE,
            ProofType::Trust => Trust::BEGIN_SIGNATURE,
        }
    }
    fn end_block(&self) -> &'static str {
        match self {
            ProofType::Code => review::Code::END_BLOCK,
            ProofType::Package => review::Package::END_BLOCK,
            ProofType::Trust => Trust::END_BLOCK,
        }
    }
}

/// Serialized Proof
///
/// A signed proof containing some signed `Content`
#[derive(Debug, Clone)]
pub(crate) struct Serialized {
    /// Serialized content
    pub body: String,
    /// Signature over the body
    pub signature: String,
    /// Type of the `body` (`Content`)
    pub type_: ProofType,
}

/// Content is an enumerator of possible proof contents
#[derive(Debug, Clone)]
pub enum Content {
    Trust(Trust),
    Package(review::Package),
    Code(review::Code),
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
        Content::Code(review)
    }
}

impl From<review::Package> for Content {
    fn from(review: review::Package) -> Self {
        Content::Package(review)
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
    pub fn parse(s: &str, type_: ProofType) -> Result<Content> {
        Ok(match type_ {
            ProofType::Code => Content::Code(review::Code::parse(&s)?),
            ProofType::Package => Content::Package(review::Package::parse(&s)?),
            ProofType::Trust => Content::Trust(Trust::parse(&s)?),
        })
    }

    pub fn parse_draft(original_proof: &Content, s: &str) -> Result<Content> {
        Ok(match original_proof {
            Content::Code(code) => {
                Content::Code(code.apply_draft(review::CodeDraft::parse(&s)?.into()))
            }
            Content::Package(package) => {
                Content::Package(package.apply_draft(review::PackageDraft::parse(&s)?.into()))
            }
            Content::Trust(trust) => {
                Content::Trust(trust.apply_draft(TrustDraft::parse(&s)?.into()))
            }
        })
    }

    pub fn sign_by(&self, id: &crate::id::OwnId) -> Result<Proof> {
        let body = self.to_string();
        let signature = id.sign(&body.as_bytes());
        Ok(Proof {
            digest: crev_common::blake2b256sum(&body.as_bytes()),
            body: body,
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
            Trust(trust) => format!("{}", TrustDraft::from(trust)),
            Code(review) => format!("{}", review::CodeDraft::from(review)),
            Package(review) => format!("{}", review::PackageDraft::from(review)),
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
            digest: crev_common::blake2b256sum(&self.body.as_bytes()),
            content: match self.type_ {
                ProofType::Code => Content::Code(review::Code::parse(&self.body)?),
                ProofType::Package => Content::Package(review::Package::parse(&self.body)?),
                ProofType::Trust => Content::Trust(Trust::parse(&self.body)?),
            },
        })
    }

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
                        let line = line.trim();
                        if line.is_empty() {
                        } else if line == ProofType::Code.begin_block() {
                            self.type_ = ProofType::Code;
                            self.stage = Stage::Body;
                        } else if line == ProofType::Trust.begin_block() {
                            self.type_ = ProofType::Trust;
                            self.stage = Stage::Body;
                        } else if line == ProofType::Package.begin_block() {
                            self.type_ = ProofType::Package;
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

    pub fn parse(reader: impl io::Read) -> Result<Vec<Self>> {
        let mut v = vec![];
        for serialized in Serialized::parse(reader)?.into_iter() {
            v.push(serialized.to_parsed()?)
        }
        Ok(v)
    }

    pub fn signature(&self) -> &str {
        self.signature.trim()
    }

    pub fn verify(&self) -> Result<()> {
        let pubkey = self.content.author_id();
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
