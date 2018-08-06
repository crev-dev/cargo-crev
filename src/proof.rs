use blake2;
use common_failures::prelude::*;
use std::collections::{hash_map::Entry, HashMap};
use std::mem;

use chrono::{self, prelude::*};

#[allow(unused)]
pub struct ReviewProof {
    date: chrono::DateTime<FixedOffset>,
    revision: String,
    hash: String,
    scope: Vec<String>,
    comment: Vec<String>,
    signed_by: String,
    signed_by_id: String,
    signature: String,
}

impl ReviewProof {
    pub fn from_map(kvs: HashMap<&str, Vec<&str>>) -> Result<Self> {
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
            date: chrono::DateTime::parse_from_rfc3339(date)
                .with_context(|e| format!("While parsing date `{}`: {}", date, e))?,
            revision: get_single_required(&kvs, "revision")?.to_owned(),
            hash: get_single_required(&kvs, "hash")?.to_owned(),
            signed_by: get_single_required(&kvs, "signed-by")?.to_owned(),
            signed_by_id: get_single_required(&kvs, "signed-by-id")?.to_owned(),
            signature: get_single_required(&kvs, "signature")?.to_owned(),
            scope: get_at_least_one(&kvs, "scope")?.to_owned(),
            comment: get_vec(&kvs, "scope")?.to_owned(),
        })
    }

    pub fn parse(input: &str) -> Result<Vec<Self>> {
        #[derive(Default)]
        struct State<'a> {
            kvs: HashMap<&'a str, Vec<&'a str>>,
            hash: blake2::Blake2b,
            parsed: Vec<ReviewProof>,
        }

        impl<'a> State<'a> {
            fn new() -> Self {
                Default::default()
            }

            fn reset(&mut self) {
                *self = State {
                    parsed: mem::replace(&mut self.parsed, vec![]),
                    ..Default::default()
                };
            }

            fn is_started(&self) -> bool {
                !self.kvs.is_empty()
            }

            fn hash_line(&mut self, line: &str) {
                use blake2::Digest;
                self.hash.input(line.as_bytes())
            }

            fn process_line(&mut self, line: &'a str) -> Result<()> {
                if line.trim().is_empty() {
                    if self.is_started() {
                        self.hash_line(&line);
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
                self.hash_line(line);
                self.kvs
                    .entry(k)
                    .and_modify(|e| e.push(v))
                    .or_insert_with(|| vec![v]);

                if k == "signature" {
                    self.parsed.push(ReviewProof::from_map(mem::replace(
                        &mut self.kvs,
                        Default::default(),
                    ))?);
                    self.reset();
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
signed-by: a
signed-by-id: a
signature: sig
"#;

    let proofs = ReviewProof::parse(&s)?;
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
signature: sig
date: 1996-12-19T16:39:57-00:00
revision: a
hash: a
signed-by: a
signed-by-id: a
signature: sig
"#;

    let proofs = ReviewProof::parse(&s)?;
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

    assert!(ReviewProof::parse(&s).is_err());

    let s = r#"
date: 1996-12-19T16:39:57-08:00
revision: a
signed-by: a
signed-by-id: a
signature: sig
"#;
    assert!(ReviewProof::parse(&s).is_err());

    Ok(())
}
