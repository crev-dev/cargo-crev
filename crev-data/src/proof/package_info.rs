use crate::proof;

use crev_common::serde::{as_base64, from_base64};
use derive_builder::Builder;
use semver::Version;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Builder, Serialize, Deserialize, PartialEq)]
pub struct PackageInfo {
    #[serde(flatten)]
    pub id: Option<crate::id::PubId>,
    pub source: String,
    pub name: String,
    pub version: Version,

    #[serde(skip_serializing_if = "proof::equals_default", default)]
    pub revision: String,
    #[serde(
        rename = "revision-type",
        skip_serializing_if = "proof::equals_default_revision_type",
        default = "proof::default_revision_type"
    )]
    pub revision_type: String,

    #[serde(serialize_with = "as_base64", deserialize_with = "from_base64")]
    pub digest: Vec<u8>,
    #[serde(
        skip_serializing_if = "proof::equals_default_digest_type",
        default = "proof::default_digest_type"
    )]
    pub digest_type: String,
}
