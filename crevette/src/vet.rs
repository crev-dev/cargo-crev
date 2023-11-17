use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
#[serde(untagged)]
pub enum StringOrVec {
    String(String),
    Vec(Vec<String>),
}

#[derive(Serialize)]
pub struct AuditEntry {
    pub who: StringOrVec,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub violation: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub criteria: Vec<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(rename = "aggregated-from")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aggregated_from: Vec<String>,
}

#[derive(Serialize)]
pub struct CriteriaEntry {
    pub description: Option<&'static str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub implies: Vec<&'static str>,
    #[serde(rename = "aggregated-from")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aggregated_from: Vec<String>,
}

#[derive(Serialize)]
pub struct AuditsFile {
    pub audits: BTreeMap<String, Vec<AuditEntry>>,
    pub criteria: BTreeMap<&'static str, CriteriaEntry>,
}
