//! Some common stuff for both Review and Trust Proofs

use base64;
use chrono::{self, prelude::*};
use id;
use serde;
use serde_yaml;
use std::{default, fmt, io, marker, mem, path::PathBuf};
use util;

pub mod review;
pub mod trust;

pub use review::*;
pub use trust::*;

use Result;

pub trait Content:
    Sized + for<'a> serde::Deserialize<'a> + serde::Serialize + fmt::Display
{
    const BEGIN_BLOCK: &'static str;
    const BEGIN_SIGNATURE: &'static str;
    const END_BLOCK: &'static str;

    const CONTENT_TYPE_NAME: &'static str;
    const PROOF_EXTENSION: &'static str;

    fn date(&self) -> chrono::DateTime<FixedOffset>;
    fn from_pubid(&self) -> String;
    fn from_url(&self) -> String;
    fn project_id(&self) -> Option<&str>;

    /// The path to use under project `.crev/`
    fn rel_project_path(&self) -> PathBuf {
        PathBuf::from(self.from_pubid())
            .join(Self::CONTENT_TYPE_NAME)
            .join(self.date().with_timezone(&Utc).format("%Y-%m").to_string())
            .with_extension(Self::PROOF_EXTENSION)
    }

    /// The path to use under user store
    fn rel_store_path(&self) -> PathBuf {
        let mut path = PathBuf::from(self.from_pubid()).join(Self::CONTENT_TYPE_NAME);

        if let Some(project_id) = self.project_id() {
            path = path.join(project_id)
        }

        path.join(self.date().with_timezone(&Utc).format("%Y-%m").to_string())
            .with_extension(Self::PROOF_EXTENSION)
    }

    fn sign(&self, id: &id::OwnId) -> Result<Serialized<Self>> {
        let body = self.to_string();
        let signature = id.sign(&body.as_bytes());
        Ok(Serialized {
            body: body,
            signature: base64::encode_config(&signature, base64::URL_SAFE),
            phantom: marker::PhantomData,
        })
    }

    fn parse(s: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(&s)?)
    }
}

/// A signed proof containing some signed `Content`
#[derive(Debug, Clone)]
pub struct Serialized<T> {
    pub body: String,
    pub signature: String,
    phantom: marker::PhantomData<T>,
}

#[derive(Debug, Clone)]
/// A `Proof` with it's content parsed and ready.
pub struct Parsed<T> {
    pub body: String,
    pub signature: String,
    pub digest: Vec<u8>,
    pub content: T,
}

impl<T: Content> fmt::Display for Serialized<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(T::BEGIN_BLOCK)?;
        f.write_str("\n")?;
        f.write_str(&self.body)?;
        f.write_str(T::BEGIN_SIGNATURE)?;
        f.write_str("\n")?;
        f.write_str(&self.signature)?;
        f.write_str("\n")?;
        f.write_str(T::END_BLOCK)?;
        f.write_str("\n")?;

        Ok(())
    }
}

impl<T: Content> fmt::Display for Parsed<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(T::BEGIN_BLOCK)?;
        f.write_str("\n")?;
        f.write_str(&self.body)?;
        f.write_str(T::BEGIN_SIGNATURE)?;
        f.write_str("\n")?;
        f.write_str(&self.signature)?;
        f.write_str("\n")?;
        f.write_str(T::END_BLOCK)?;
        f.write_str("\n")?;

        Ok(())
    }
}
impl<T: Content> Serialized<T> {
    pub fn to_parsed(&self) -> Result<Parsed<T>> {
        Ok(Parsed {
            body: self.body.clone(),
            signature: self.signature.clone(),
            digest: util::blaze2sum(&self.body.as_bytes()),
            content: <T as Content>::parse(&self.body)?,
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

        struct State<T> {
            stage: Stage,
            body: String,
            signature: String,
            proofs: Vec<Serialized<T>>,
        }

        impl<T> default::Default for State<T> {
            fn default() -> Self {
                State {
                    stage: Default::default(),
                    body: Default::default(),
                    signature: Default::default(),
                    proofs: vec![],
                }
            }
        }

        impl<T: Content> State<T> {
            fn process_line(&mut self, line: &str) -> Result<()> {
                match self.stage {
                    Stage::None => {
                        if line.trim().is_empty() {
                        } else if line.trim() == T::BEGIN_BLOCK {
                            self.stage = Stage::Body;
                        } else {
                            bail!("Parsing error when looking for start of code review proof");
                        }
                    }
                    Stage::Body => {
                        if line.trim() == T::BEGIN_SIGNATURE {
                            self.stage = Stage::Signature;
                        } else {
                            self.body += line;
                            self.body += "\n";
                        }
                        if self.body.len() > 16_000 {
                            bail!("Parsed body too long");
                        }
                    }
                    Stage::Signature => {
                        if line.trim() == T::END_BLOCK {
                            self.stage = Stage::None;
                            self.proofs.push(Serialized {
                                body: mem::replace(&mut self.body, String::new()),
                                signature: mem::replace(&mut self.signature, String::new()),
                                phantom: marker::PhantomData,
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

            fn finish(self) -> Result<Vec<Serialized<T>>> {
                if self.stage != Stage::None {
                    bail!("Unexpected EOF while parsing");
                }
                Ok(self.proofs)
            }
        }

        let mut state: State<T> = Default::default();

        for line in reader.lines() {
            state.process_line(&line?)?;
        }

        state.finish()
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
