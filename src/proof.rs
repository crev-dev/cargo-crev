use base64;
use blake2::{self, Digest};
use chrono::{self, prelude::*};
use common_failures::prelude::*;
use git2;
use id::PubId;
use index;
use serde_yaml;
use std::collections::{hash_map::Entry, HashMap};
use std::{io::Write, mem, path::PathBuf};
use util::serde::{as_hex, as_rfc3339_fixed, from_hex, from_rfc3339_fixed};

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
    path: PathBuf,
    #[serde(serialize_with = "as_hex", deserialize_with = "from_hex")]
    digest: Vec<u8>,
    #[serde(rename = "digest-type")]
    digest_type: String,
}

#[derive(Clone, Builder, Debug, Serialize, Deserialize)]
// TODO: validate setters(no newlines, etc)
// TODO: https://github.com/colin-kiegel/rust-derive-builder/issues/136
/// Unsigned proof of code review
pub struct ReviewProof {
    #[builder(default = "now()")]
    #[serde(
        serialize_with = "as_rfc3339_fixed",
        deserialize_with = "from_rfc3339_fixed"
    )]
    date: chrono::DateTime<FixedOffset>,
    from: String,
    #[serde(rename = "from-id")]
    from_id: String,
    project: String,
    #[serde(rename = "from-id-type")]
    from_id_type: String,
    files: Vec<ReviewProofFile>,
    revision: Option<String>,
    #[serde(rename = "revision-type")]
    revision_type: String,
    comment: Option<String>,
    thoroughness: Level,
    understanding: Level,
}

use id::OwnId;

fn now() -> DateTime<FixedOffset> {
    let date = chrono::offset::Local::now();
    date.with_timezone(&date.offset())
}

impl ReviewProof {
    pub fn from_staged(own_id: &OwnId, _staged: &index::Staged) -> Result<Self> {
        let mut proof = ReviewProofBuilder::default();

        proof
            .from(own_id.name().into())
            .from_id(own_id.pub_key_as_base64())
            .from_id_type(own_id.type_as_string());
        unimplemented!();
    }
    /*
    // TODO: Make a builder
    pub fn new(
        revision: String,
        file_hash: String,
        thoroughness: Level,
        understanding: Level,
    ) -> Self {
        let date = chrono::offset::Local::now();
        // TODO: validate (no newlines, etc)
        Self {
            date: date.with_timezone(&date.offset()),
            revision,
            file_hash,
            thoroughness,
            understanding,
            // TODO:
            comment: None,
        }
    }*/

    pub fn to_string(&self) -> Result<String> {
        Ok(serde_yaml::to_string(self)?)
    }

    pub fn sign(&self, id: &OwnId) -> Result<SignedReviewProof> {
        let body = self.to_string()?;
        let signature = id.sign(&body.as_bytes());
        Ok(SignedReviewProof {
            body: body,
            signature: base64::encode(&signature),
        })
    }
    /*
    pub fn sign(&self, id: &OwnId) -> SignedReviewProof {
        let mut out = vec![];
        write!(out, "date: {}", self.date.to_rfc3339()).unwrap();
        if let Some(ref revision) = self.revision {
            write!(out, "revision: {}", revision).unwrap();
        }
        if let Some(ref file_hash) = self.file_hash {
            write!(out, "file-hash: {}", file_hash).unwrap();
        }
        write!(out, "thoroughness: {}", self.thoroughness.as_str()).unwrap();
        write!(out, "understanding: {}", self.understanding.as_str()).unwrap();
        if let Some(ref comment) = &self.comment {
            write!(out, "comment: {}", comment).unwrap();
        }
        write!(out, "signed-by: {}", id.name()).unwrap();
        write!(
            out,
            "signed-by-id: crev={}",
            base64::encode(id.pub_key_as_bytes())
        ).unwrap();

        let signature = id.sign(&out);
        write!(out, "signature: {}", base64::encode(&signature)).unwrap();
        SignedReviewProof {
            serialized: out,
            review_proof: self.to_owned(),
            signed_by: id.to_pubid(),
            signature: signature,
        }
    }
    */
}

#[derive(Debug)]
pub struct SignedReviewProof {
    //review_proof: ReviewProof,
    body: String,
    //signed_by: PubId,
    signature: String,
}

impl SignedReviewProof {
    /*
    pub fn from_map(kvs: HashMap<&str, Vec<&str>>, serialized: Vec<u8>) -> Result<Self> {
        fn get_single_required<'a, 'b>(
            kvs: &'a HashMap<&'a str, Vec<&'a str>>,
            key: &str,
        ) -> Result<&'a str> {
            let v = kvs
                .get(key)
                .ok_or_else(|| format_err!("`{}` key missing", key))?;
            if v.is_empty() {
                bail!("`{}` has no values", key);
            }
            if v.len() > 1 {
                bail!("`{}` has multiple values", key);
            }

            Ok(v[0])
        }

        fn get_single_maybe<'a, 'b>(
            kvs: &'a HashMap<&'a str, Vec<&'a str>>,
            key: &str,
        ) -> Result<Option<&'a str>> {
            let v = kvs.get(key);
            if v.is_none() {
                return Ok(None);
            }
            let v = v.unwrap();

            if v.len() > 1 {
                bail!("`{}` has multiple values", key);
            }

            Ok(Some(v[0]))
        }
        fn get_at_least_one<'a, 'b>(
            kvs: &'a HashMap<&'a str, Vec<&'a str>>,
            key: &str,
        ) -> Result<Vec<String>> {
            Ok(kvs
                .get(key)
                .map(|v| v.as_slice())
                .unwrap_or_else(|| &[])
                .iter()
                .map(|s| s.to_string())
                .collect())
        }

        fn get_vec<'a, 'b>(
            kvs: &'a HashMap<&'a str, Vec<&'a str>>,
            key: &str,
        ) -> Result<Vec<String>> {
            Ok(kvs
                .get(key)
                .map(|v| v.as_slice())
                .unwrap_or_else(|| &[])
                .iter()
                .map(|s| s.to_string())
                .collect())
        }

        let date = get_single_required(&kvs, "date")?;
        Ok(Self {
            review_proof: ReviewProof {
                date: chrono::DateTime::parse_from_rfc3339(date)
                    .with_context(|e| format!("While parsing date `{}`: {}", date, e))?,
                revision: Some(get_single_required(&kvs, "revision")?.to_owned()),
                file_hash: Some(get_single_required(&kvs, "file-hash")?.to_owned()),
                thoroughness: Level::from_str(
                    get_single_maybe(&kvs, "thoroughness")?.unwrap_or("good"),
                )?,
                understanding: Level::from_str(
                    get_single_maybe(&kvs, "understanding")?.unwrap_or("good"),
                )?,
                comment: get_single_maybe(&kvs, "scope")?.map(|s| s.to_owned()),
            },
            serialized: serialized,
            signed_by: PubId::from_name_and_id_string(
                get_single_required(&kvs, "signed-by")?.to_owned(),
                get_single_required(&kvs, "signed-by-id")?,
            )?,
            signature: base64::decode(get_single_required(&kvs, "signature")?)?,
        })
    }
*/
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
            proofs: Vec<SignedReviewProof>,
        }

        impl State {
            fn process_line(&mut self, line: &str) -> Result<()> {
                match self.stage {
                    Stage::None => {
                        if line.trim().is_empty() {
                        } else if line.trim() == "-----BEGIN CODE REVIEW PROOF-----" {
                            self.stage = Stage::Body;
                        } else {
                            bail!("Parsing error when looking for start of code review proof");
                        }
                    }
                    Stage::Body => {
                        if line.trim() == "-----BEGIN CODE REVIEW PROOF SIGNATURE-----" {
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
                        if line.trim() == "-----END CODE REVIEW PROOF-----" {
                            self.stage = Stage::None;
                            self.proofs.push(SignedReviewProof {
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

            fn finish(self) -> Result<Vec<SignedReviewProof>> {
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
            parsed: Vec<SignedReviewProof>,
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
                    self.parsed.push(SignedReviewProof::from_map(
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

    let proofs = SignedReviewProof::parse(&s)?;
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
-----BEGIN CODE REVIEW SIGNATURE-----
sig2
-----END CODE REVIEW PROOF-----
"#;

    let proofs = SignedReviewProof::parse(&s)?;
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
-----BEGIN CODE REVIEW SIGNATURE-----
sig1
-----END CODE REVIEW PROOF-----


-----BEGIN CODE REVIEW PROOF-----
foo2
-----BEGIN CODE REVIEW SIGNATURE-----
sig2
-----END CODE REVIEW PROOF-----"#;

    let proofs = SignedReviewProof::parse(&s)?;
    assert_eq!(proofs.len(), 2);
    assert_eq!(proofs[0].body, "foo1\n");
    assert_eq!(proofs[0].signature, "sig1\n");
    assert_eq!(proofs[1].body, "foo2\n");
    assert_eq!(proofs[1].signature, "sig2\n");
    Ok(())
}
/*
#[test]
fn multiple() -> Result<()> {
    let s = r#"
date: 1996-12-19T16:39:57-08:00
revision: a
hash: a
signed-by: a
signed-by-id: a
signature: crev=sig
date: 1996-12-19T16:39:57-00:00
revision: a
hash: a
signed-by: Name
signed-by-id: crev=aa
signature: crev=aa
"#;

    let proofs = SignedReviewProof::parse(&s)?;
    assert_eq!(proofs.len(), 2);
    Ok(())
}

#[test]
fn missing_value() -> Result<()> {
    let s = r#"
date: 1996-12-19T16:39:57-08:00
revision: a
hash:
signed-by: a
signed-by-id: a
signature: sig
"#;

    assert!(SignedReviewProof::parse(&s).is_err());

    let s = r#"
date: 1996-12-19T16:39:57-08:00
revision: a
signed-by: a
signed-by-id: a
signature: sig
"#;
    assert!(SignedReviewProof::parse(&s).is_err());

    Ok(())
}
*/
