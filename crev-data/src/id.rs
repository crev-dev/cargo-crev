use argonautica::{self, Hasher};
use base64;
use blake2;
use common_failures::prelude::*;
use ed25519_dalek::{self, PublicKey};
use miscreant;
use rand::{self, OsRng, Rng};
use serde_yaml;
use std::{
    self,
    io::{self, Read, Write},
    path::Path,
};
use crev_common::serde::{as_base64, from_base64};


#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "id-type")]
/// Public CrevId of someone
pub struct PubId {
    // One Variant, because I love slowing myself down for no good reason :/
    Crev {
        url: String,

        #[serde(
            serialize_with = "as_base64",
            deserialize_with = "from_base64"
        )]
        id: Vec<u8>,
    },
}

impl PubId {

    // TODO: This function sucks; it should be something else, or named better
    // or whatever
    pub fn write_to(&self, w: &mut io::Write) -> Result<()> {
        match self {
            PubId::Crev { url, id } => {
                writeln!(w, "id: {}", base64::encode_config(id, base64::URL_SAFE))?;
                writeln!(w, "url: {}", url)?;
            }
        }
        Ok(())
    }

    pub fn id_as_base64(&self) -> String {
        match self {
            PubId::Crev { id, .. } => base64::encode_config(id, base64::URL_SAFE),
        }
    }

    pub fn to_string(&self) -> String {
        let mut s = vec![];
        self.write_to(&mut s).unwrap();
        String::from_utf8_lossy(&s).into()
    }
}

#[derive(Debug)]
pub enum OwnId {
    Crev {
        url: String,
        keypair: ed25519_dalek::Keypair,
    },
}

impl OwnId {
    pub fn sign(&self, msg: &[u8]) -> Vec<u8> {
        match self {
            OwnId::Crev { keypair, .. } => {
                keypair.sign::<blake2::Blake2b>(&msg).to_bytes().to_vec()
            }
        }
    }

    pub fn type_as_string(&self) -> String {
        match self {
            OwnId::Crev { .. } => "crev".into(),
        }
    }
    pub fn to_pubid(&self) -> PubId {
        match self {
            OwnId::Crev { url, keypair } => PubId::Crev {
                url: url.to_owned(),
                id: keypair.public.as_bytes().to_vec(),
            },
        }
    }

    pub fn url(&self) -> &str {
        match self {
            OwnId::Crev { url, .. } => url,
        }
    }

    pub fn pub_key_as_bytes(&self) -> &[u8] {
        match self {
            OwnId::Crev { keypair, .. } => keypair.public.as_bytes(),
        }
    }

    pub fn pub_key_as_base64(&self) -> String {
        base64::encode_config(&self.pub_key_as_bytes(), base64::URL_SAFE)
    }

    pub fn generate(url: String) -> Self {
        let mut csprng: OsRng = OsRng::new().unwrap();
        OwnId::Crev {
            url,
            keypair: ed25519_dalek::Keypair::generate::<blake2::Blake2b, _>(&mut csprng),
        }
    }
}

