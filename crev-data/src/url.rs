#[derive(Clone, Debug, Builder, Serialize, Deserialize, PartialEq)]
pub struct Url {
    pub url: String,
    #[serde(
        rename = "url-type",
        skip_serializing_if = "equals_default_url_type",
        default = "default_url_type"
    )]
    pub url_type: String,
}

impl Url {
    pub fn new(url: String) -> Self {
        Self {
            url,
            url_type: default_url_type(),
        }
    }
}
pub(crate) fn equals_default_url_type(s: &str) -> bool {
    s == default_url_type()
}

pub(crate) fn default_url_type() -> String {
    "git".into()
}
