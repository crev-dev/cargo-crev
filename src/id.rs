use argonautica;
use argonautica::Hasher;
use base64;
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
                    keypair: ed25519_dalek::Keypair {
                        secret: sec_key,
                        public: calculated_pub_key,
                    },
                })
            }
        }
    }
}

#[derive(Debug)]
pub enum PubId {
    Crev { name: String, id: Vec<u8> },
}

impl PubId {
    pub fn from_name_and_id_string(name: String, id_str: &str) -> Result<Self> {
        let mut split = id_str.split('=');
        let key = split
            .next()
            .map(|s| s.trim())
            .ok_or_else(|| format_err!("missing key"))?;
        let val = split
            .next()
            .map(|s| s.trim())
            .ok_or_else(|| format_err!("missing value"))?;

        Ok(match key {
            "crev" => PubId::Crev {
                name,
                id: base64::decode(val)?,
            },
            _ => bail!("Unknown id type key {}", val),
        })
    }
}

#[derive(Debug)]
pub enum Id {
    Crev {
        name: String,
        keypair: ed25519_dalek::Keypair,
    },
}

impl Id {
    pub fn sign(&self, msg: &[u8]) -> Vec<u8> {
        match self {
            Id::Crev { name, keypair } => keypair.sign::<blake2::Blake2b>(&msg).to_bytes().to_vec(),
        }
    }
    pub fn to_pubid(&self) -> PubId {
        match self {
            Id::Crev { name, keypair } => PubId::Crev {
                name: name.to_owned(),
                id: keypair.public.as_bytes().to_vec(),
            },
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Id::Crev { name, keypair } => name,
        }
    }
    pub fn pub_key_as_bytes(&self) -> &[u8] {
        match self {
            Id::Crev { name, keypair } => keypair.public.as_bytes(),
        }
    }

    fn generate(name: String) -> Self {
        let mut csprng: OsRng = OsRng::new().unwrap();
        Id::Crev {
            name,
            keypair: ed25519_dalek::Keypair::generate::<blake2::Blake2b, _>(&mut csprng),
        }
    }

    fn to_locked(&self, passphrase: &str) -> Result<LockedId> {
        match self {
            Id::Crev { name, keypair } => {
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
                    pub_key: keypair.public.to_bytes().to_vec(),
                    sealed_sec_key: siv.seal(&seal_nonce, &[], keypair.secret.as_bytes()),
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
