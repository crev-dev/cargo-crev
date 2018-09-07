use chrono::{self, prelude::*};
use level::Level;
use proof;
use serde_yaml;
use std::fmt;
use crev_common::{self, serde::{as_rfc3339_fixed, from_rfc3339_fixed}};

const BEGIN_BLOCK: &str = "-----BEGIN CODE REVIEW TRUST-----";
const BEGIN_SIGNATURE: &str = "-----BEGIN CODE REVIEW TRUST SIGNATURE-----";
const END_BLOCK: &str = "-----END CODE REVIEW TRUST-----";
pub const PROOF_EXTENSION: &str = "trust.crev";

#[derive(Clone, Debug, Builder, Serialize, Deserialize)]
pub struct Trust {
    #[builder(default = "crev_common::now()")]
    #[serde(
        serialize_with = "as_rfc3339_fixed",
        deserialize_with = "from_rfc3339_fixed"
    )]
    date: chrono::DateTime<FixedOffset>,
    from: String,
    #[serde(rename = "from-url")]
    from_url: String,
    #[serde(
        rename = "from-type",
        skip_serializing_if = "proof::equals_crev",
        default = "proof::default_crev_value"
    )]
    #[builder(default = "\"crev\".into()")]
    from_type: String,
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
    const PROOF_EXTENSION: &'static str = PROOF_EXTENSION;

    fn date(&self) -> chrono::DateTime<FixedOffset> {
        self.date
    }
    fn project_id(&self) -> Option<&str> {
        None
    }
    fn from_pubid(&self) -> String {
        self.from.clone()
    }
    fn from_url(&self) -> String {
        self.from_url.clone()
    }
}

pub type TrustProof = super::Serialized<Trust>;
