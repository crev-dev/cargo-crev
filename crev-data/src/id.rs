use crate::{Result, Url};
use base64;
use blake2;
use crev_common::serde::{as_base64, from_base64};
use ed25519_dalek::{self, PublicKey, SecretKey};
use rand::OsRng;
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IdType {
    #[serde(rename = "crev")]
    Crev,
}

impl fmt::Display for IdType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::IdType::*;
        f.write_str(match self {
            Crev => "crev",
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "id-type")]
pub enum Id {
    #[serde(rename = "crev")]
    Crev {
        #[serde(serialize_with = "as_base64", deserialize_with = "from_base64")]
        id: Vec<u8>,
    },
}

impl Id {
    pub fn new_crevid_from_string(&self, s: &str) -> Result<Self> {
        let bytes = base64::decode_config(s, base64::URL_SAFE)?;

        Ok(Id::Crev { id: bytes })
    }

    pub fn verify_signature(&self, content: &[u8], sig_str: &str) -> Result<()> {
        match self {
            Id::Crev { id } => {
                let pubkey = ed25519_dalek::PublicKey::from_bytes(&id)?;

                let sig_bytes = base64::decode_config(sig_str, base64::URL_SAFE)?;
                let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes)?;

                pubkey.verify::<blake2::Blake2b>(content, &signature)?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Id::Crev { id } => f.write_str(&base64::encode_config(id, base64::URL_SAFE)),
        }
    }
}
/*
impl<T: Borrow<id::PubId>> From<T> for IdAndUrl {
    fn from(id: T) -> Self {
        let id = id.borrow();
        IdAndUrl {
            id: Id::Crev {
                id: id.pub_key_as_base64(),
            },
            url: Some(Url {
                url: id.url.clone(),
                url_type: super::url::default_url_type(),
            }),
        }
    }
}
*/

#[derive(Clone, Debug, Builder, Serialize, Deserialize)]
pub struct PubId {
    #[serde(flatten)]
    pub id: Id,
    #[serde(flatten)]
    pub url: Option<Url>,
}

impl PubId {
    pub fn new(v: Vec<u8>, url: String) -> Self {
        PubId {
            id: Id::Crev { id: v },
            url: Some(Url::new(url)),
        }
    }
    pub fn new_from_pubkey(v: Vec<u8>) -> Self {
        PubId {
            id: Id::Crev { id: v },
            url: None,
        }
    }

    pub fn new_crevid_from_base64(s: &str) -> Result<Self> {
        let v = base64::decode_config(s, base64::URL_SAFE)?;
        Ok(PubId {
            id: Id::Crev { id: v },
            url: None,
        })
    }
    pub fn set_git_url(&mut self, url: String) {
        self.url = Some(Url {
            url,
            url_type: super::url::default_url_type(),
        })
    }

    /*
    pub fn pub_key_as_base64(&self) -> String {
        base64::encode_config(&self.id.id, base64::URL_SAFE)
    }
    */
}

pub(crate) fn equals_default_id_type(s: &str) -> bool {
    s == default_id_type()
}

pub(crate) fn default_id_type() -> String {
    "crev".into()
}
/*
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Public CrevId of someone
pub struct PubId {
    #[serde(
        serialize_with = "as_base64",
        deserialize_with = "from_base64"
    )]
    pub id: Vec<u8>,
    #[serde(rename = "id-type")]
    pub id_type: IdType,
    pub url: String,
}

impl PubId {
    pub fn new(url: String, id: Vec<u8>) -> Self {
        Self {
            url,
            id,
            id_type: IdType::Crev,
        }
    }

    pub fn pub_key_as_base64(&self) -> String {
        base64::encode_config(&self.id, base64::URL_SAFE)
    }
}

impl fmt::Display for PubId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crev_common::serde::write_as_headerless_yaml(self, f)
    }
}
*/

/// A `PubId` with the corresponding secret key
#[derive(Debug)]
pub struct OwnId {
    pub id: PubId,
    pub keypair: ed25519_dalek::Keypair,
}

impl OwnId {
    pub fn new(url: String, sec_key: Vec<u8>) -> Result<Self> {
        let sec_key = SecretKey::from_bytes(&sec_key)?;
        let calculated_pub_key: PublicKey = PublicKey::from_secret::<blake2::Blake2b>(&sec_key);

        Ok(Self {
            id: crate::PubId::new(calculated_pub_key.as_bytes().to_vec(), url),
            keypair: ed25519_dalek::Keypair {
                secret: sec_key,
                public: calculated_pub_key,
            },
        })
    }

    pub fn sign(&self, msg: &[u8]) -> Vec<u8> {
        self.keypair
            .sign::<blake2::Blake2b>(&msg)
            .to_bytes()
            .to_vec()
    }

    pub fn type_as_string(&self) -> String {
        "crev".into()
    }

    pub fn as_pubid(&self) -> &PubId {
        &self.id
    }

    /*
    pub fn pub_key_as_base64(&self) -> String {
        self.id.pub_key_as_base64()
    }
    */

    pub fn generate(url: String) -> Self {
        let mut csprng: OsRng = OsRng::new().unwrap();
        let keypair = ed25519_dalek::Keypair::generate::<blake2::Blake2b, _>(&mut csprng);
        Self {
            id: PubId::new(keypair.public.as_bytes().to_vec(), url),
            keypair,
        }
    }
}
