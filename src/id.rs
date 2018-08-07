use argonautica;
use argonautica::Hasher;
use blake2;
use common_failures::prelude::*;
use ed25519_dalek;
use ed25519_dalek::PublicKey;
use ed25519_dalek::SecretKey;
use ed25519_dalek::Signature;
use miscreant;
use rand;
use rand::OsRng;
use rand::Rng;
use serde_yaml;
use util::serde::{as_base64, from_base64};

/// Serialized, stored on disk
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum LockedId {
    Crev {
        version: u16,
        name: String,
        #[serde(
            serialize_with = "as_base64",
            deserialize_with = "from_base64"
        )]
        pub_key: Vec<u8>,
        #[serde(
            serialize_with = "as_base64",
            deserialize_with = "from_base64"
        )]
        sealed_sec_key: Vec<u8>,

        #[serde(
            serialize_with = "as_base64",
            deserialize_with = "from_base64"
        )]
        seal_nonce: Vec<u8>,

        #[serde(
            serialize_with = "as_base64",
            deserialize_with = "from_base64"
        )]
        pass_salt: Vec<u8>,
    },
}

impl LockedId {
    fn to_unlocked(&self, passphrase: &str) -> Result<Id> {
        match self {
            LockedId::Crev {
                ref version,
                ref name,
                ref pub_key,
                ref sealed_sec_key,
                ref seal_nonce,
                ref pass_salt,
            } => {
                if *version != 0 {
                    bail!("Unsupported version");
                }
                use miscreant::aead::Algorithm;
                let mut hasher = Hasher::default();

                hasher
                    .configure_memory_size(4096)
                    .with_salt(pass_salt)
                    .configure_hash_len(64)
                    .opt_out_of_secret_key(true);

                let pwhash = hasher.with_password(passphrase).hash_raw()?;

                let mut siv = miscreant::aead::Aes256Siv::new(pwhash.raw_hash_bytes());

                let sec_key = siv.open(&seal_nonce, &[], &sealed_sec_key)?;
                let sec_key = ed25519_dalek::SecretKey::from_bytes(&sec_key)?;

                let calculated_pub_key: PublicKey =
                    PublicKey::from_secret::<blake2::Blake2b>(&sec_key);

                if ed25519_dalek::PublicKey::from_bytes(&pub_key)? != calculated_pub_key {
                    bail!("PubKey mismatch");
                }

                Ok(Id::Crev {
                    name: name.clone(),
                    sec_key: sec_key,
                    pub_key: calculated_pub_key,
                })
            }
        }
    }
}

#[derive(Debug)]
pub enum Id {
    Crev {
        name: String,
        sec_key: ed25519_dalek::SecretKey,
        pub_key: ed25519_dalek::PublicKey,
    },
}

impl Id {
    fn pub_key_as_bytes(&self) -> &[u8] {
        match self {
            Id::Crev {
                name,
                sec_key,
                pub_key,
            } => pub_key.as_bytes(),
        }
    }
    fn generate(name: String) -> Self {
        let mut csprng: OsRng = OsRng::new().unwrap();
        let sec_key: SecretKey = SecretKey::generate(&mut csprng);

        let pub_key: PublicKey = PublicKey::from_secret::<blake2::Blake2b>(&sec_key);

        Id::Crev {
            name,
            sec_key,
            pub_key,
        }
    }

    fn to_locked(&self, passphrase: &str) -> Result<LockedId> {
        match self {
            Id::Crev {
                name,
                sec_key,
                pub_key,
            } => {
                use miscreant::aead::Algorithm;
                let mut hasher = Hasher::default();

                hasher
                    .configure_memory_size(4096)
                    .configure_hash_len(64)
                    .opt_out_of_secret_key(true);

                let pwhash = hasher.with_password(passphrase).hash_raw()?;

                let mut siv = miscreant::aead::Aes256Siv::new(pwhash.raw_hash_bytes());

                let mut seal_nonce: Vec<u8> = rand::thread_rng()
                    .sample_iter(&rand::distributions::Standard)
                    .take(32)
                    .collect();

                Ok(LockedId::Crev {
                    version: 0,
                    pub_key: pub_key.to_bytes().to_vec(),
                    sealed_sec_key: siv.seal(&seal_nonce, &[], sec_key.as_bytes()),
                    seal_nonce: seal_nonce,
                    pass_salt: pwhash.raw_salt_bytes().to_vec(),
                    name: name.clone(),
                })
            }
        }
    }
}

#[test]
fn lock_and_unlock() -> Result<()> {
    let id = Id::generate("Dawid Ciężarkiewicz".into());

    let id_relocked = id.to_locked("password")?.to_unlocked("password")?;
    assert_eq!(id.pub_key_as_bytes(), id_relocked.pub_key_as_bytes());

    assert!(
        id.to_locked("password")?
            .to_unlocked("wrongpassword")
            .is_err()
    );

    let id_stored = serde_yaml::to_string(&id.to_locked("pass")?)?;
    let id_restored: Id = serde_yaml::from_str::<LockedId>(&id_stored)?.to_unlocked("pass")?;

    assert_eq!(id.pub_key_as_bytes(), id_restored.pub_key_as_bytes());
    Ok(())
}
