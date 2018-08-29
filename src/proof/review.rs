use base64;
use blake2::{self, Digest};
use chrono::{self, prelude::*};
use common_failures::prelude::*;
use git2;
use id::PubId;
use level::Level;
use proof;
use serde_yaml;
use std::collections::{hash_map::Entry, HashMap};
use std::{fmt, io::Write, marker, mem, path::PathBuf};
use util::{
    self,
    serde::{as_hex, as_rfc3339_fixed, from_hex, from_rfc3339_fixed},
};

const BEGIN_BLOCK: &str = "-----BEGIN CODE REVIEW-----";
const BEGIN_SIGNATURE: &str = "-----BEGIN CODE REVIEW SIGNATURE-----";
const END_BLOCK: &str = "-----END CODE REVIEW-----";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReviewFile {
    pub path: PathBuf,
    #[serde(serialize_with = "as_hex", deserialize_with = "from_hex")]
    pub digest: Vec<u8>,
    #[serde(rename = "digest-type")]
    #[serde(
        skip_serializing_if = "equals_blake2b",
        default = "default_blake2b_value"
    )]
    pub digest_type: String,
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

#[derive(Clone, Builder, Debug, Serialize, Deserialize)]
// TODO: validate setters(no newlines, etc)
// TODO: https://github.com/colin-kiegel/rust-derive-builder/issues/136
/// Unsigned proof of code review
pub struct Review {
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
    #[builder(default = "\"crev\".into()")]
    #[serde(
        skip_serializing_if = "equals_crev",
        default = "default_crev_value"
    )]
    from_type: String,
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
    files: Vec<ReviewFile>,
}

use id::OwnId;

impl proof::Content for Review {
    const BEGIN_BLOCK: &'static str = BEGIN_BLOCK;
    const BEGIN_SIGNATURE: &'static str = BEGIN_SIGNATURE;
    const END_BLOCK: &'static str = END_BLOCK;
    const CONTENT_TYPE_NAME: &'static str = "review";

    fn date(&self) -> chrono::DateTime<FixedOffset> {
        self.date
    }
    fn from_pubid(&self) -> String {
        self.from.clone()
    }
    fn from_name(&self) -> String {
        self.from_name.clone()
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

pub type ReviewProof = super::Proof<Review>;

#[test]
fn signed_parse() -> Result<()> {
    let s = r#"
-----BEGIN CODE REVIEW-----
foo
-----BEGIN CODE REVIEW SIGNATURE-----
sig
-----END CODE REVIEW-----
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
-----BEGIN CODE REVIEW-----
foo1
-----BEGIN CODE REVIEW SIGNATURE-----
sig1
-----END CODE REVIEW-----
-----BEGIN CODE REVIEW-----
foo2
-----BEGIN CODE REVIEW SIGNATURE-----
sig2
-----END CODE REVIEW-----
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

-----BEGIN CODE REVIEW-----
foo1
-----BEGIN CODE REVIEW SIGNATURE-----
sig1
-----END CODE REVIEW-----


-----BEGIN CODE REVIEW-----
foo2
-----BEGIN CODE REVIEW SIGNATURE-----
sig2
-----END CODE REVIEW-----"#;

    let proofs = ReviewProof::parse(&s)?;
    assert_eq!(proofs.len(), 2);
    assert_eq!(proofs[0].body, "foo1\n");
    assert_eq!(proofs[0].signature, "sig1\n");
    assert_eq!(proofs[1].body, "foo2\n");
    assert_eq!(proofs[1].signature, "sig2\n");
    Ok(())
}
