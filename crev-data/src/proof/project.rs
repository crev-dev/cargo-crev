use base64;
use rand::{self, Rng};

#[derive(Clone, Debug, Builder, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub id: String,
    #[serde(
        rename = "id-type",
        skip_serializing_if = "crate::id::equals_default_id_type",
        default = "crate::id::default_id_type"
    )]
    pub id_type: String,
    #[serde(flatten)]
    pub url: Option<crate::Url>,
}

impl Project {
    pub fn generate() -> Self {
        let project_id: Vec<u8> = rand::thread_rng()
            .sample_iter(&rand::distributions::Standard)
            .take(32)
            .collect();
        Self {
            id: base64::encode_config(&project_id, base64::URL_SAFE),
            id_type: "crev".into(),
            url: None,
        }
    }
    pub fn from_id(id: String) -> Self {
        Self {
            id: id,
            id_type: "crev".into(),
            url: None,
        }
    }
}
