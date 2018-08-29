//! Some common stuff for both Review and Trust Proofs

use base64;
use blake2::{self, Digest};
use chrono::{self, prelude::*};
use common_failures::prelude::*;
use git2;
use id;
use level::Level;
use serde;
use serde_yaml;
use std::{
    self,
    collections::{hash_map::Entry, HashMap},
    default, fmt,
    io::Write,
    marker, mem,
    path::PathBuf,
};
use util::{
    self,
    serde::{as_hex, as_rfc3339_fixed, from_hex, from_rfc3339_fixed},
};

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

    fn date(&self) -> chrono::DateTime<FixedOffset>;
    fn from_pubid(&self) -> String;
    fn from_name(&self) -> String;

    fn rel_store_path(&self) -> PathBuf {
        PathBuf::from(self.from_pubid())
            .join(Self::CONTENT_TYPE_NAME)
            .join(self.date().with_timezone(&Utc).format("%Y-%m").to_string())
            .with_extension("crev")
    }

    fn sign(&self, id: &id::OwnId) -> Result<Proof<Self>> {
        let body = self.to_string();
        let signature = id.sign(&body.as_bytes());
        Ok(Proof {
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
pub struct Proof<T> {
    pub body: String,
    pub signature: String,
    phantom: marker::PhantomData<T>,
}

impl<T: Content> fmt::Display for Proof<T> {
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

impl<T: Content> Proof<T> {
    pub fn parse_content(&self) -> Result<T> {
        <T as Content>::parse(&self.body)
    }

    pub fn parse(input: &str) -> Result<Vec<Self>> {
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
            proofs: Vec<Proof<T>>,
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
                            self.proofs.push(Proof {
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

            fn finish(self) -> Result<Vec<Proof<T>>> {
                if self.stage != Stage::None {
                    bail!("Unexpected EOF while parsing");
                }
                Ok(self.proofs)
            }
        }

        let mut state: State<T> = Default::default();

        for line in input.lines() {
            state.process_line(&line)?;
        }

        state.finish()
    }
}
