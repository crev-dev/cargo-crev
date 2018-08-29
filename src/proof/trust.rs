use base64;
use blake2::{self, Digest};
use chrono::{self, prelude::*};
use common_failures::prelude::*;
use git2;
use id::OwnId;
use id::PubId;
use level::Level;
use proof::Content;
use serde_yaml;
use std::collections::{hash_map::Entry, HashMap};
use std::{fmt, io::Write, mem, path::PathBuf};
use util::{
    self,
    serde::{as_hex, as_rfc3339_fixed, from_hex, from_rfc3339_fixed},
};

const BEGIN_BLOCK: &str = "-----BEGIN CODE REVIEW TRUST-----";
const BEGIN_SIGNATURE: &str = "-----BEGIN CODE REVIEW TRUST SIGNATURE-----";
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

impl super::Content for Trust {
    const BEGIN_BLOCK: &'static str = BEGIN_BLOCK;
    const BEGIN_SIGNATURE: &'static str = BEGIN_SIGNATURE;
    const END_BLOCK: &'static str = END_BLOCK;
    const CONTENT_TYPE_NAME: &'static str = "trust";

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

pub type TrustProof = super::Proof<Trust>;
