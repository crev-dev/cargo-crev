use crate::proof;

#[derive(Clone, Debug, Builder, Serialize, Deserialize)]
pub struct Revision {
    pub revision: String,
    #[serde(
        rename = "revision-type",
        skip_serializing_if = "proof::equals_default_revision_type",
        default = "proof::default_revision_type"
    )]
    #[builder(default = "\"git\".into()")]
    pub revision_type: String,
}
