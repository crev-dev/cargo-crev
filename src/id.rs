use argonautica::{self, Hasher};
use base64;
use blake2;
use common_failures::prelude::*;
use ed25519_dalek::{self, PublicKey, SecretKey, Signature};
use miscreant;
use rand::{self, OsRng, Rng};
use serde_yaml;
use std::{
    self, fs,
    io::{self, Read, Write},
    path::Path,
};
use util::{
    self,
    serde::{as_base64, from_base64},
};

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
pub struct LockedId {
    version: u16,
    url: String,
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
}

impl LockedId {
    pub fn to_pubid(&self) -> PubId {
        PubId::Crev {
            url: self.url.to_owned(),
            id: self.pub_key.to_owned(),
        }
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        let id = serde_yaml::to_string(&self)?;
        write!(file, "{}", id)?;

        Ok(())
    }

    pub fn read_from_yaml_file(path: &Path) -> Result<Self> {
        let mut file = std::fs::File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        Ok(serde_yaml::from_str::<LockedId>(&content)?)
    }

    pub fn to_unlocked(&self, passphrase: &str) -> Result<OwnId> {
        let LockedId {
            ref version,
            ref url,
            ref pub_key,
            ref sealed_sec_key,
            ref seal_nonce,
            ref pass,
        } = self;
        {
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

            let calculated_pub_key: PublicKey = PublicKey::from_secret::<blake2::Blake2b>(&sec_key);

            if ed25519_dalek::PublicKey::from_bytes(&pub_key)? != calculated_pub_key {
                bail!("PubKey mismatch");
            }

            Ok(OwnId::Crev {
                url: url.clone(),
                keypair: ed25519_dalek::Keypair {
                    secret: sec_key,
                    public: calculated_pub_key,
                },
            })
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "id-type")]
pub enum PubId {
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
            PubId::Crev { url, id } => base64::encode_config(id, base64::URL_SAFE),
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
            OwnId::Crev { url, keypair } => {
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
            OwnId::Crev { url, keypair } => url,
        }
    }

    pub fn pub_key_as_bytes(&self) -> &[u8] {
        match self {
            OwnId::Crev { url, keypair } => keypair.public.as_bytes(),
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

    pub fn to_locked(&self, passphrase: &str) -> Result<LockedId> {
        match self {
            OwnId::Crev { url, keypair } => {
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
                Ok(LockedId {
                    version: 0,
                    pub_key: keypair.public.to_bytes().to_vec(),
                    sealed_sec_key: siv.seal(&seal_nonce, &[], keypair.secret.as_bytes()),
                    seal_nonce: seal_nonce,
                    url: url.clone(),
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
    let id = OwnId::generate("Dawid Ciężarkiewicz".into());

    let id_relocked = id.to_locked("password")?.to_unlocked("password")?;
    assert_eq!(id.pub_key_as_bytes(), id_relocked.pub_key_as_bytes());

    assert!(
        id.to_locked("password")?
            .to_unlocked("wrongpassword")
            .is_err()
    );

    let id_stored = serde_yaml::to_string(&id.to_locked("pass")?)?;
    let id_restored: OwnId = serde_yaml::from_str::<LockedId>(&id_stored)?.to_unlocked("pass")?;

    println!("{}", id_stored);

    assert_eq!(id.pub_key_as_bytes(), id_restored.pub_key_as_bytes());
    Ok(())
}
