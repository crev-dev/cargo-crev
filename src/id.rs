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
use std;
use util::serde::{as_base64, from_base64};

#[derive(Serialize, Deserialize, Debug)]
pub struct PassConfig {
    version: u32,
    variant: String,
    iterations: u32,
    memory_size: u32,
    #[serde(
        serialize_with = "as_base64",
        deserialize_with = "from_base64"
    )]
    salt: Vec<u8>,
}

/// Serialized, stored on disk
#[derive(Serialize, Deserialize, Debug)]
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
        pass: PassConfig,
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
                ref pass,
            } => {
                if *version != 0 {
                    bail!("Unsupported version");
                }
                use miscreant::aead::Algorithm;

                let mut hasher = Hasher::default();

                hasher
                    .configure_memory_size(pass.memory_size)
                    .configure_version(argonautica::config::Version::from_u32(pass.version)?)
                    .configure_iterations(pass.iterations)
                    .configure_variant(std::str::FromStr::from_str(&pass.variant)?)
                    .with_salt(&pass.salt)
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

                let hasher_config = hasher.config();

                assert_eq!(hasher_config.version(), argonautica::config::Version::_0x13);
                Ok(LockedId::Crev {
                    version: 0,
                    pub_key: pub_key.to_bytes().to_vec(),
                    sealed_sec_key: siv.seal(&seal_nonce, &[], sec_key.as_bytes()),
                    seal_nonce: seal_nonce,
                    name: name.clone(),
                    pass: PassConfig {
                        salt: pwhash.raw_salt_bytes().to_vec(),
                        iterations: hasher_config.iterations(),
                        memory_size: hasher_config.memory_size(),
                        version: 0x13,
                        variant: hasher_config.variant().as_str().to_string(),
                    },
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

    println!("{}", id_stored);

    assert_eq!(id.pub_key_as_bytes(), id_restored.pub_key_as_bytes());
    Ok(())
}
