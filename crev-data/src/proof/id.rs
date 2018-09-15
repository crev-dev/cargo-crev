use id;

use std::borrow::Borrow;

#[derive(Clone, Debug, Builder, Serialize, Deserialize)]
pub struct IdUrl {
    pub url: String,
    #[serde(
        rename = "url-type",
        skip_serializing_if = "equals_default_url_type",
        default = "default_url_type"
    )]
    pub url_type: String,
}

#[derive(Clone, Debug, Builder, Serialize, Deserialize)]
pub struct Id {
    pub id: String,
    #[serde(
        rename = "id-type",
        skip_serializing_if = "equals_default_id_type",
        default = "default_id_type"
    )]
    pub id_type: String,
    #[serde(flatten)]
    pub url: Option<IdUrl>,
}

impl<T: Borrow<id::PubId>> From<T> for Id {
    fn from(id: T) -> Self {
        let id = id.borrow();
        Id {
            id: id.pub_key_as_base64(),
            id_type: default_id_type(),
            url: Some(IdUrl {
                url: id.url.clone(),
                url_type: default_url_type(),
            }),
        }
    }
}

impl Id {
    pub fn new_from_string(s: String) -> Self {
        Id {
            id: s,
            id_type: default_id_type(),
            url: None,
        }
    }
    pub fn set_git_url(&mut self, url: String) {
        self.url = Some(IdUrl {
            url,
            url_type: default_url_type(),
        })
    }
}

fn equals_default_id_type(s: &str) -> bool {
    s == default_id_type()
}

fn default_id_type() -> String {
    "crev".into()
}

fn equals_default_url_type(s: &str) -> bool {
    s == default_url_type()
}

fn default_url_type() -> String {
    "git".into()
}
