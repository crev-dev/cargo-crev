use base64;
use blake2::{self, Digest};
use chrono::{self, prelude::*};
use common_failures::prelude::*;
use id::PubId;
use std::collections::{hash_map::Entry, HashMap};
use std::{io::Write, mem};

#[derive(Debug, Clone)]
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

#[derive(Clone, Builder, Debug)]
// TODO: validate setters(no newlines, etc)
// TODO: https://github.com/colin-kiegel/rust-derive-builder/issues/136
pub struct ReviewProof {
    #[builder(default = "now()")]
    date: chrono::DateTime<FixedOffset>,
    revision: Option<String>,
    file_hash: Option<String>,
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
}

#[derive(Debug)]
pub struct SignedReviewProof {
    review_proof: ReviewProof,
    serialized: Vec<u8>,
    signed_by: PubId,
    signature: Vec<u8>,
}

impl SignedReviewProof {
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
}

#[test]
fn simple() -> Result<()> {
    let s = r#"
date: 1996-12-19T16:39:57-08:00
revision: a
hash: a
signed-by: some name
signed-by-id: crev=a
signature: sig
"#;

    let proofs = SignedReviewProof::parse(&s)?;
    Ok(())
}

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
