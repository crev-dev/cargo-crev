use std::fmt::Display;

use security_framework::os::macos::keychain::SecKeychain;

const NOT_FOUND: i32 = -25300; // errSecItemNotFound
const SERVICE: &str = "cargo-crev:id";
/// Used to save or retrieve passphrase from KeyChain as "for any id" if we have only one id.
pub const NO_ID: &str = "";

pub fn retrieve_existing_passphrase(id: &str) -> Result<String, anyhow::Error> {
    let keychain = SecKeychain::default()?;
    let not_found = security_framework::base::Error::from(NOT_FOUND).code();

    match keychain.find_generic_password(SERVICE, id) {
        Ok((pass, _)) => {
            let password = String::from_utf8(pass.as_ref().to_vec())?;
            Ok(password)
        }
        Err(e) if e.code() == not_found => Err(Error::NotFound.into()),
        Err(e) => Err(e.into()),
    }
}

pub fn save_new_passphrase(id: &str, password: &str) -> Result<(), anyhow::Error> {
    let keychain = SecKeychain::default()?;
    let not_found = security_framework::base::Error::from(NOT_FOUND).code();

    match keychain.find_generic_password(SERVICE, id) {
        Err(e) => {
            if e.code() == not_found {
                keychain.add_generic_password(SERVICE, id, password.as_bytes())?;
            }
        }
        Ok((_, mut item)) => {
            item.set_password(password.as_bytes())?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub enum Error {
    NotFound,
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotFound => write!(f, "Credentials not found"),
        }
    }
}
