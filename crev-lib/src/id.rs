//! `LockedId` is for you, the local crev user. `Id` is for identifying other users.

use crate::{Error, Result};
use aes_siv::KeyInit;
use argon2::{self, Config};
use crev_common::{
    rand::random_vec,
    serde::{as_base64, from_base64},
};
use crev_data::id::{PublicId, UnlockedId};
use serde::{Deserialize, Serialize};
use std::{self, fmt, io::BufReader, path::Path};

const CURRENT_LOCKED_ID_SERIALIZATION_VERSION: i64 = -1;

/// Callback to read the password
pub type PassphraseFn<'a> = &'a dyn Fn() -> std::io::Result<String>;

/// Stored in your config to know how to hash your passphrase
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PassphraseConfig {
    version: u32,
    variant: String,
    iterations: u32,
    #[serde(rename = "memory-size")]
    memory_size: u32,
    lanes: Option<u32>,
    #[serde(serialize_with = "as_base64", deserialize_with = "from_base64")]
    salt: Vec<u8>,
}

/// Serialized, stored on disk
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LockedId {
    version: i64,

    /// Where your crev-proofs git repo is
    #[serde(flatten)]
    pub url: Option<crev_data::Url>,

    /// This is used in `PublicId` to identify users
    #[serde(serialize_with = "as_base64", deserialize_with = "from_base64")]
    #[serde(rename = "public-key")]
    pub public_key: Vec<u8>,

    /// Needs passphrase
    #[serde(serialize_with = "as_base64", deserialize_with = "from_base64")]
    #[serde(rename = "sealed-secret-key")]
    sealed_secret_key: Vec<u8>,

    #[serde(serialize_with = "as_base64", deserialize_with = "from_base64")]
    #[serde(rename = "seal-nonce")]
    seal_nonce: Vec<u8>,

    #[serde(rename = "pass")]
    passphrase_config: PassphraseConfig,
}

impl fmt::Display for LockedId {
    /// Somewhat surprisingly, you get full YAML of the file
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_yaml::to_string(self).map_err(|_| fmt::Error)?)
    }
}

/// Parses YAML file
impl std::str::FromStr for LockedId {
    type Err = serde_yaml::Error;

    fn from_str(yaml_str: &str) -> std::result::Result<Self, Self::Err> {
        serde_yaml::from_str::<LockedId>(yaml_str)
    }
}

impl LockedId {
    /// Encrypt and throw away the key
    pub fn from_unlocked_id(unlocked_id: &UnlockedId, passphrase: &str) -> Result<LockedId> {
        let config = if !passphrase.is_empty() {
            Config {
                variant: argon2::Variant::Argon2id,
                version: argon2::Version::Version13,

                hash_length: 64,
                mem_cost: 4096,
                time_cost: 192,

                lanes: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as u32,

                ad: &[],
                secret: &[],
            }
        } else {
            Self::weak_passphrase_config()
        };

        let pwsalt = random_vec(32);
        let pwhash =
            argon2::hash_raw(passphrase.as_bytes(), &pwsalt, &config).map_err(Error::Passphrase)?;

        let seal_nonce = random_vec(32);
        let sealed_secret_key = {
            use aes_siv::{aead::generic_array::GenericArray, siv::IV_SIZE};

            let secret = unlocked_id.keypair.secret.as_bytes();
            let mut siv = aes_siv::siv::Aes256Siv::new(&GenericArray::clone_from_slice(&pwhash));
            let mut buffer = vec![0; IV_SIZE + secret.len()];
            buffer[IV_SIZE..].copy_from_slice(secret);
            let tag = siv
                .encrypt_in_place_detached([&[] as &[u8], &seal_nonce], &mut buffer[IV_SIZE..])
                .expect("aes-encrypt");
            buffer[..IV_SIZE].copy_from_slice(&tag);
            buffer
        };

        Ok(LockedId {
            version: CURRENT_LOCKED_ID_SERIALIZATION_VERSION,
            public_key: unlocked_id.keypair.public.to_bytes().to_vec(),
            sealed_secret_key,
            seal_nonce,
            url: unlocked_id.url().cloned(),
            passphrase_config: PassphraseConfig {
                salt: pwsalt,
                iterations: config.time_cost,
                memory_size: config.mem_cost,
                version: 0x13,
                lanes: Some(config.lanes),
                variant: config.variant.as_lowercase_str().to_string(),
            },
        })
    }

    /// Extract only the public identity part from all data. Useful for displaying user's identity.
    #[must_use]
    pub fn to_public_id(&self) -> PublicId {
        PublicId::new_from_pubkey(self.public_key.clone(), self.url.clone())
            .expect("Invalid locked id.")
    }

    #[must_use]
    pub fn pub_key_as_base64(&self) -> String {
        crev_common::base64_encode(&self.public_key)
    }

    /// Write the Id to this file, overwriting it
    pub fn save_to(&self, path: &Path) -> Result<()> {
        let s = self.to_string();
        crev_common::store_str_to_file(path, &s).map_err(|e| Error::FileWrite(e, path.into()))
    }

    pub fn read_from_yaml_file(path: &Path) -> Result<Self> {
        let mut file = BufReader::new(
            std::fs::File::open(path)
                .map_err(|e| Error::IdLoadError(Box::new((path.into(), e))))?,
        );

        Ok(serde_yaml::from_reader(&mut file)?)
    }

    /// Decrypt
    pub fn to_unlocked(&self, passphrase: &str) -> Result<UnlockedId> {
        let LockedId {
            ref version,
            ref url,
            ref public_key,
            ref sealed_secret_key,
            ref seal_nonce,
            ref passphrase_config,
        } = self;
        {
            if *version > CURRENT_LOCKED_ID_SERIALIZATION_VERSION {
                return Err(Error::UnsupportedVersion(*version));
            }
            let mut config = Config {
                variant: argon2::Variant::from_str(&passphrase_config.variant)?,
                version: argon2::Version::Version13,

                hash_length: 64,
                mem_cost: passphrase_config.memory_size,
                time_cost: passphrase_config.iterations,

                lanes: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as u32,

                ad: &[],
                secret: &[],
            };

            if let Some(lanes) = passphrase_config.lanes {
                config.lanes = lanes;
            } else {
                log::error!(
                    "`lanes` not configured. Old bug. See: https://github.com/crev-dev/cargo-crev/issues/151"
                );
                log::info!("Using `lanes: {}`", config.lanes);
            }

            let passphrase_hash =
                argon2::hash_raw(passphrase.as_bytes(), &passphrase_config.salt, &config)?;

            let secret_key = {
                use aes_siv::{aead::generic_array::GenericArray, siv::IV_SIZE, Tag};

                let mut siv =
                    aes_siv::siv::Aes256Siv::new(&GenericArray::clone_from_slice(&passphrase_hash));
                let mut buffer = sealed_secret_key.clone();
                let tag = Tag::clone_from_slice(&buffer[..IV_SIZE]);
                siv.decrypt_in_place_detached(
                    [&[] as &[u8], seal_nonce],
                    &mut buffer[IV_SIZE..],
                    &tag,
                )
                .map_err(|_| Error::IncorrectPassphrase)?;
                buffer.drain(..IV_SIZE);
                buffer
            };

            assert!(!secret_key.is_empty());

            let result = UnlockedId::new(url.clone(), &secret_key)?;
            if public_key != &result.keypair.public.to_bytes() {
                return Err(Error::PubKeyMismatch);
            }
            Ok(result)
        }
    }

    /// Used for temporary/default identity, but obviously not very secure to store
    #[must_use]
    pub fn has_no_passphrase(&self) -> bool {
        self.passphrase_config.iterations == 1 && self.to_unlocked("").is_ok()
    }

    /// Config for empty passphrase. User chose no security, so they're getting none.
    fn weak_passphrase_config() -> Config<'static> {
        Config {
            variant: argon2::Variant::Argon2id,
            version: argon2::Version::Version13,

            hash_length: 64,
            mem_cost: 16,
            time_cost: 1,

            lanes: 1,

            ad: &[],
            secret: &[],
        }
    }
}
