use base64;
use chrono::{self, offset::FixedOffset, prelude::*};
use ed25519_dalek;
use hex::{self, FromHex, FromHexError};
use serde::{self, Deserialize};
use std::io;

pub trait MyTryFromBytes: Sized {
    type Err: 'static + Sized + ::std::error::Error;
    fn try_from(&[u8]) -> Result<Self, Self::Err>;
}

impl MyTryFromBytes for ed25519_dalek::PublicKey {
    type Err = io::Error;
    fn try_from(slice: &[u8]) -> Result<Self, Self::Err> {
        ed25519_dalek::PublicKey::from_bytes(slice).map_err(|_e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "can't derive PublicKey from invalid binary data",
            )
        })
    }
}

/*
impl MyTryFromBytes for secretbox::Nonce {
    type Err = io::Error;
    fn try_from(slice: &[u8]) -> Result<Self, Self::Err> {
        secretbox::Nonce::from_slice(slice).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "can't derive Nonce from invalid binary data",
            )
        })
    }
}
*/

/*
impl MyTryFromBytes for pwhash::Salt {
    type Err = io::Error;
    fn try_from(slice: &[u8]) -> Result<Self, Self::Err> {
        pwhash::Salt::from_slice(slice).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "can't derive Nonce from invalid binary data",
            )
        })
    }
}
*/

impl MyTryFromBytes for Vec<u8> {
    type Err = io::Error;
    fn try_from(slice: &[u8]) -> Result<Self, Self::Err> {
        Ok(Vec::from(slice))
    }
}


