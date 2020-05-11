use crate::{proof, proof::ContentExt, Result, Url};
use crev_common::{
    self,
    serde::{as_base64, from_base64},
};
use derive_builder::Builder;
use ed25519_dalek::{self, PublicKey, SecretKey};
use failure::format_err;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
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

/// An Id supported by `crev` system
///
/// Right now it's only native CrevID, but in future at least GPG
/// should be supported.
#[derive(Clone, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "id-type")]
pub enum Id {
    #[serde(rename = "crev")]
    Crev {
        #[serde(serialize_with = "as_base64", deserialize_with = "from_base64")]
        id: Vec<u8>,
    },
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Id::Crev { id } => f.write_str(&crev_common::base64_encode(id)),
        }
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Id::Crev { id } => f.write_str(&crev_common::base64_encode(id)),
        }
    }
}

impl Id {
    pub fn new_crev(bytes: Vec<u8>) -> Result<Self> {
        if bytes.len() != 32 {
            failure::bail!(
                "wrong length of crev id, expected 32 bytes, got {}",
                bytes.len()
            );
        }
        Ok(Id::Crev { id: bytes })
    }

    pub fn crevid_from_str(s: &str) -> Result<Self> {
        let bytes = crev_common::base64_decode(s)?;
        Self::new_crev(bytes)
    }

    pub fn verify_signature(&self, content: &[u8], sig_str: &str) -> Result<()> {
        match self {
            Id::Crev { id } => {
                let pubkey = ed25519_dalek::PublicKey::from_bytes(&id)?;

                let sig_bytes = crev_common::base64_decode(sig_str)?;
                let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes)?;
                pubkey.verify(&content, &signature)?;
            }
        }

        Ok(())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Id::Crev { id } => id.clone(),
        }
    }
}

/// A unique ID accompanied by publically identifying data.
#[derive(Clone, Debug, Builder, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PublicId {
    #[serde(flatten)]
    pub id: Id,
    #[serde(flatten)]
    pub url: Option<Url>,
}

impl PublicId {
    pub fn new(id: Id, url: Url) -> Self {
        Self { id, url: Some(url) }
    }

    pub fn new_id_only(id: Id) -> Self {
        Self { id, url: None }
    }

    pub fn new_from_pubkey(v: Vec<u8>, url: Url) -> Result<Self> {
        Ok(Self {
            id: Id::new_crev(v)?,
            url: Some(url),
        })
    }

    pub fn new_crevid_from_base64(s: &str, url: Url) -> Result<Self> {
        let v = crev_common::base64_decode(s)?;
        Ok(Self {
            id: Id::new_crev(v)?,
            url: Some(url),
        })
    }

    pub fn create_trust_proof<'a>(
        &self,
        ids: impl IntoIterator<Item = &'a PublicId>,
        trust_level: proof::trust::TrustLevel,
    ) -> Result<proof::Trust> {
        Ok(proof::TrustBuilder::default()
            .from(self.clone())
            .trust(trust_level)
            .ids(ids.into_iter().cloned().collect())
            .build()
            .map_err(|e| format_err!("{}", e))?)
    }

    pub fn create_package_review_proof(
        &self,
        package: proof::PackageInfo,
        review: proof::review::Review,
        comment: String,
    ) -> Result<proof::review::Package> {
        Ok(proof::review::PackageBuilder::default()
            .from(self.clone())
            .package(package)
            .review(review)
            .comment(comment)
            .build()
            .map_err(|e| format_err!("{}", e))?)
    }

    pub fn url_display(&self) -> &str {
        match &self.url {
            Some(url) => &url.url,
            None => "(no url)",
        }
    }
}

/// A `PublicId` with the corresponding secret key
#[derive(Debug)]
pub struct UnlockedId {
    pub id: PublicId,
    pub keypair: ed25519_dalek::Keypair,
}

impl AsRef<Id> for UnlockedId {
    fn as_ref(&self) -> &Id {
        &self.id.id
    }
}

impl AsRef<PublicId> for UnlockedId {
    fn as_ref(&self) -> &PublicId {
        &self.id
    }
}

impl UnlockedId {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(url: Url, sec_key: Vec<u8>) -> Result<Self> {
        let sec_key = SecretKey::from_bytes(&sec_key)?;
        let calculated_pub_key: PublicKey = PublicKey::from(&sec_key);

        Ok(Self {
            id: crate::PublicId::new_from_pubkey(calculated_pub_key.as_bytes().to_vec(), url)?,
            keypair: ed25519_dalek::Keypair {
                secret: sec_key,
                public: calculated_pub_key,
            },
        })
    }

    pub fn sign(&self, msg: &[u8]) -> Vec<u8> {
        self.keypair.sign(msg).to_bytes().to_vec()
    }

    pub fn type_as_string(&self) -> String {
        "crev".into()
    }

    pub fn as_public_id(&self) -> &PublicId {
        &self.id
    }

    pub fn url(&self) -> &Url {
        self.id.url.as_ref().expect("UnlockedId must have a URL")
    }

    pub fn generate_for_git_url(url: &str) -> Self {
        Self::generate(Url::new_git(url.to_owned()))
    }

    pub fn generate(url: Url) -> Self {
        let keypair = ed25519_dalek::Keypair::generate(&mut OsRng);
        Self {
            id: PublicId::new_from_pubkey(keypair.public.as_bytes().to_vec(), url)
                .expect("should be valid keypair"),
            keypair,
        }
    }

    pub fn create_signed_trust_proof<'a>(
        &self,
        ids: impl IntoIterator<Item = &'a PublicId>,
        trust_level: proof::trust::TrustLevel,
    ) -> Result<proof::Proof> {
        self.id.create_trust_proof(ids, trust_level)?.sign_by(&self)
    }
}
