use base64;
use blake2::{self, Digest};
use chrono::{self, prelude::*};
use common_failures::prelude::*;
use git2;
use id::PubId;
use index;
use serde_yaml;
use std::collections::{hash_map::Entry, HashMap};
use std::{fmt, io::Write, mem, path::PathBuf};
use util::serde::{as_hex, as_rfc3339_fixed, from_hex, from_rfc3339_fixed};

const BEGIN_BLOCK: &str = "-----BEGIN CODE REVIEW PROOF-----";
const SIGNATURE_BLOCK: &str = "-----BEGIN CODE REVIEW PROOF SIGNATURE-----";
const END_BLOCK: &str = "-----END CODE REVIEW PROOF-----";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Level {
    None,
    Some,
    Good,
    Ultimate,
}

impl Default for Level {
    fn default() -> Self {
        Level::Some
    }
}

impl Level {
    fn as_str(&self) -> &str {
        use self::Level::*;
        match self {
            None => "none",
            Some => "some",
            Good => "good",
            Ultimate => "ultimate",
        }
    }
    fn from_str(s: &str) -> Result<Level> {
        Ok(match s {
            "none" => Level::None,
            "some" => Level::Some,
            "good" => Level::Good,
            "ultimate" => Level::Ultimate,
            _ => bail!("Unknown level: {}", s),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReviewProofFile {
    pub path: PathBuf,
    #[serde(serialize_with = "as_hex", deserialize_with = "from_hex")]
    pub digest: Vec<u8>,
    #[serde(rename = "digest-type")]
    pub digest_type: String,
}

fn equals_crev(s: &str) -> bool {
    s == "crev"
}

fn default_crev_value() -> String {
    "crev".into()
}

#[derive(Clone, Builder, Debug, Serialize, Deserialize)]
// TODO: validate setters(no newlines, etc)
// TODO: https://github.com/colin-kiegel/rust-derive-builder/issues/136
/// Unsigned proof of code review
pub struct Review {
    #[builder(default = "now()")]
    #[serde(
        serialize_with = "as_rfc3339_fixed",
        deserialize_with = "from_rfc3339_fixed"
    )]
    date: chrono::DateTime<FixedOffset>,
    from: String,
    #[serde(rename = "from-id")]
    from_id: String,
    #[serde(rename = "from-id-type")]
    #[builder(default = "\"crev\".into()")]
    #[serde(
        skip_serializing_if = "equals_crev",
        default = "default_crev_value"
    )]
    from_id_type: String,
    project_urls: Vec<String>,
    revision: Option<String>,
    #[serde(rename = "revision-type")]
    #[builder(default = "\"git\".into()")]
    revision_type: String,
    #[builder(default = "None")]
    comment: Option<String>,
    thoroughness: Level,
    understanding: Level,
    trust: Level,
    files: Vec<ReviewProofFile>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrustProof {
    #[serde(
        serialize_with = "as_rfc3339_fixed",
        deserialize_with = "from_rfc3339_fixed"
    )]
    date: chrono::DateTime<FixedOffset>,
    from: String,
    #[serde(rename = "from-id")]
    from_id: String,
    #[serde(rename = "from-id-type")]
    from_id_type: String,
    ids: Vec<PubId>,
    #[serde(rename = "revision-type")]
    comment: Option<String>,
    trust: Level,
}

use id::OwnId;

fn now() -> DateTime<FixedOffset> {
    let date = chrono::offset::Local::now();
    date.with_timezone(&date.offset())
}

impl Review {
    pub fn from_staged(own_id: &OwnId, _staged: &index::Staged) -> Result<Self> {
        let mut proof = ReviewBuilder::default();

        proof
            .from(own_id.name().into())
            .from_id(own_id.pub_key_as_base64())
            .from_id_type(own_id.type_as_string());
        unimplemented!();
    }

    pub fn sign(&self, id: &OwnId) -> Result<ReviewProof> {
        let body = self.to_string();
        let signature = id.sign(&body.as_bytes());
        Ok(ReviewProof {
            body: body,
            signature: base64::encode(&signature),
        })
    }

    pub fn parse(s: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(&s)?)
    }
}

impl fmt::Display for Review {
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

/// A `Review` that was signed by someone
#[derive(Debug)]
pub struct ReviewProof {
    pub body: String,
    pub signature: String,
}

impl fmt::Display for ReviewProof {
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

impl ReviewProof {
    pub fn parse_review(&self) -> Result<Review> {
        Review::parse(&self.body)
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

        #[derive(Default)]
        struct State {
            stage: Stage,
            body: String,
            signature: String,
            proofs: Vec<ReviewProof>,
        }

        impl State {
            fn process_line(&mut self, line: &str) -> Result<()> {
                match self.stage {
                    Stage::None => {
                        if line.trim().is_empty() {
                        } else if line.trim() == BEGIN_BLOCK {
                            self.stage = Stage::Body;
                        } else {
                            bail!("Parsing error when looking for start of code review proof");
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
                            self.proofs.push(ReviewProof {
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

            fn finish(self) -> Result<Vec<ReviewProof>> {
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

    /*
    pub fn parse(input: &str) -> Result<Vec<Self>> {
        #[derive(Default)]
        struct State<'a> {
            cur_proof_kvs: HashMap<&'a str, Vec<&'a str>>,
            cur_proof_data_hash: blake2::Blake2b,
            parsed: Vec<ReviewProof>,
        }

        impl<'a> State<'a> {
            fn new() -> Self {
                Default::default()
            }

            fn is_started(&self) -> bool {
                !self.cur_proof_kvs.is_empty()
            }

            fn hash_line(&mut self, line: &str) {
                use blake2::Digest;
                self.cur_proof_data_hash.input(line.as_bytes())
            }

            fn process_line(&mut self, untrimmed_line: &'a str) -> Result<()> {
                let line = untrimmed_line.trim();
                if line.is_empty() {
                    if self.is_started() {
                        self.hash_line(&untrimmed_line);
                    }
                    return Ok(());
                }

                let mut kv = line.splitn(2, ":");
                let k = if let Some(k) = kv.next() {
                    k.trim()
                } else {
                    bail!("missing key");
                };

                let v = if let Some(v) = kv.next() {
                    v.trim()
                } else {
                    bail!("missing value for key {}", k);
                };

                if k.is_empty() {
                    bail!("empty key");
                }

                if v.is_empty() {
                    bail!("value for key {} is empty", k);
                }

                if k == "date" {
                    if self.is_started() {
                        bail!("new `date` key found, before finishing previous one");
                    }
                }
                if k != "signature" {
                    self.hash_line(untrimmed_line);
                }
                self.cur_proof_kvs
                    .entry(k)
                    .and_modify(|e| e.push(v))
                    .or_insert_with(|| vec![v]);

                if k == "signature" {
                    self.parsed.push(ReviewProof::from_map(
                        mem::replace(&mut self.cur_proof_kvs, Default::default()),
                        mem::replace(&mut self.cur_proof_data_hash, Default::default())
                            .result()
                            .as_slice()
                            .into(),
                    )?);
                }

                Ok(())
            }
        }

        let mut state = State::new();

        for line in input.lines() {
            state.process_line(&line)?;
        }

        Ok(state.parsed)
    }
        */
}

#[test]
fn signed_parse() -> Result<()> {
    let s = r#"
-----BEGIN CODE REVIEW PROOF-----
foo
-----BEGIN CODE REVIEW PROOF SIGNATURE-----
sig
-----END CODE REVIEW PROOF-----
"#;

    let proofs = ReviewProof::parse(&s)?;
    assert_eq!(proofs.len(), 1);
    assert_eq!(proofs[0].body, "foo\n");
    assert_eq!(proofs[0].signature, "sig\n");
    Ok(())
}

#[test]
fn signed_parse_multiple() -> Result<()> {
    let s = r#"
-----BEGIN CODE REVIEW PROOF-----
foo1
-----BEGIN CODE REVIEW PROOF SIGNATURE-----
sig1
-----END CODE REVIEW PROOF-----
-----BEGIN CODE REVIEW PROOF-----
foo2
-----BEGIN CODE REVIEW PROOF SIGNATURE-----
sig2
-----END CODE REVIEW PROOF-----
"#;

    let proofs = ReviewProof::parse(&s)?;
    assert_eq!(proofs.len(), 2);
    assert_eq!(proofs[0].body, "foo1\n");
    assert_eq!(proofs[0].signature, "sig1\n");
    assert_eq!(proofs[1].body, "foo2\n");
    assert_eq!(proofs[1].signature, "sig2\n");
    Ok(())
}

#[test]
fn signed_parse_multiple_newlines() -> Result<()> {
    let s = r#"

-----BEGIN CODE REVIEW PROOF-----
foo1
-----BEGIN CODE REVIEW PROOF SIGNATURE-----
sig1
-----END CODE REVIEW PROOF-----


-----BEGIN CODE REVIEW PROOF-----
foo2
-----BEGIN CODE REVIEW PROOF SIGNATURE-----
sig2
-----END CODE REVIEW PROOF-----"#;

    let proofs = ReviewProof::parse(&s)?;
    assert_eq!(proofs.len(), 2);
    assert_eq!(proofs[0].body, "foo1\n");
    assert_eq!(proofs[0].signature, "sig1\n");
    assert_eq!(proofs[1].body, "foo2\n");
    assert_eq!(proofs[1].signature, "sig2\n");
    Ok(())
}
