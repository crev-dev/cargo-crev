use base64;
use blake2::{self, Digest};
use chrono::{self, prelude::*};
use common_failures::prelude::*;
use git2;
use id::OwnId;
use id::PubId;
use level::Level;
use serde_yaml;
use std::collections::{hash_map::Entry, HashMap};
use std::{fmt, io::Write, mem, path::PathBuf};
use util::{
    self,
    serde::{as_hex, as_rfc3339_fixed, from_hex, from_rfc3339_fixed},
};

const BEGIN_BLOCK: &str = "-----BEGIN CODE REVIEW TRUST-----";
const SIGNATURE_BLOCK: &str = "-----BEGIN CODE REVIEW TRUST SIGNATURE-----";
const END_BLOCK: &str = "-----END CODE REVIEW TRUST-----";

#[derive(Clone, Debug, Builder, Serialize, Deserialize)]
pub struct Trust {
    #[builder(default = "util::now()")]
    #[serde(
        serialize_with = "as_rfc3339_fixed",
        deserialize_with = "from_rfc3339_fixed"
    )]
    date: chrono::DateTime<FixedOffset>,
    from: String,
    #[serde(rename = "from-name")]
    from_name: String,
    #[serde(rename = "from-id-type")]
    from_type: String,
    from_urls: Vec<String>,
    #[serde(rename = "trusted-ids")]
    trusted_ids: Vec<String>,
    #[serde(rename = "comment")]
    comment: Option<String>,
    trust: Level,
}

impl fmt::Display for Trust {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let yaml_document = serde_yaml::to_string(self).map_err(|_| fmt::Error)?;
        let mut lines = yaml_document.lines();
        let dropped_header = lines.next();
        assert_eq!(dropped_header, Some("---"));

        for line in lines {
            f.write_str(&line)?;
            f.write_str("\n")?;
        }
        Ok(())
    }
}

impl Trust {
    pub fn sign(&self, id: &OwnId) -> Result<TrustProof> {
        let body = self.to_string();
        let signature = id.sign(&body.as_bytes());
        Ok(TrustProof {
            body: body,
            signature: base64::encode(&signature),
        })
    }

    pub fn parse(s: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(&s)?)
    }
}

#[derive(Clone, Debug)]
pub struct TrustProof {
    pub body: String,
    pub signature: String,
}

impl fmt::Display for TrustProof {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(BEGIN_BLOCK)?;
        f.write_str("\n")?;
        f.write_str(&self.body)?;
        f.write_str(SIGNATURE_BLOCK)?;
        f.write_str("\n")?;
        f.write_str(&self.signature)?;
        f.write_str("\n")?;
        f.write_str(END_BLOCK)?;
        f.write_str("\n")?;

        Ok(())
    }
}

impl TrustProof {
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

        #[derive(Default)]
        struct State {
            stage: Stage,
            body: String,
            signature: String,
            proofs: Vec<TrustProof>,
        }

        impl State {
            fn process_line(&mut self, line: &str) -> Result<()> {
                match self.stage {
                    Stage::None => {
                        if line.trim().is_empty() {
                        } else if line.trim() == BEGIN_BLOCK {
                            self.stage = Stage::Body;
                        } else {
                            bail!(
                                "Parsing error when looking for start of code review trust proof"
                            );
                        }
                    }
                    Stage::Body => {
                        if line.trim() == SIGNATURE_BLOCK {
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
                        if line.trim() == END_BLOCK {
                            self.stage = Stage::None;
                            self.proofs.push(TrustProof {
                                body: mem::replace(&mut self.body, String::new()),
                                signature: mem::replace(&mut self.signature, String::new()),
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

            fn finish(self) -> Result<Vec<TrustProof>> {
                if self.stage != Stage::None {
                    bail!("Unexpected EOF while parsing");
                }
                Ok(self.proofs)
            }
        }

        let mut state: State = Default::default();

        for line in input.lines() {
            state.process_line(&line)?;
        }

        state.finish()
    }
}

/*
struct TrustGraph {
    ids: HashMap<usize, Pub
}
*/
