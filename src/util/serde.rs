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

pub fn from_base64<'d, T, D>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'d>,
    T: MyTryFromBytes,
{
    use serde::de::Error;
    String::deserialize(deserializer)
        .and_then(|string| {
            base64::decode_config(&string, base64::URL_SAFE)
                .map_err(|err| Error::custom(err.to_string()))
        }).and_then(|ref bytes| {
            T::try_from(bytes)
                .map_err(|err| Error::custom(format!("{}", &err as &::std::error::Error)))
        })
}

pub fn as_base64<T, S>(key: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: AsRef<[u8]>,
    S: serde::Serializer,
{
    serializer.serialize_str(&base64::encode_config(key.as_ref(), base64::URL_SAFE))
}

pub fn from_hex<'d, T, D>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'d>,
    T: MyTryFromBytes,
{
    use serde::de::Error;
    String::deserialize(deserializer)
        .and_then(|string| {
            FromHex::from_hex(string.as_str())
                .map_err(|err: FromHexError| Error::custom(err.to_string()))
        }).and_then(|bytes: Vec<u8>| {
            T::try_from(&bytes)
                .map_err(|err| Error::custom(format!("{}", &err as &::std::error::Error)))
        })
}

pub fn as_hex<T, S>(key: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: AsRef<[u8]>,
    S: serde::Serializer,
{
    serializer.serialize_str(&hex::encode(key))
}

pub fn from_rfc3339<'d, D>(deserializer: D) -> Result<chrono::DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'d>,
{
    use serde::de::Error;
    String::deserialize(deserializer)
        .and_then(|string| {
            DateTime::<FixedOffset>::parse_from_rfc3339(&string)
                .map_err(|err| Error::custom(err.to_string()))
        }).map(|dt| dt.with_timezone(&Utc))
}

pub fn as_rfc3339<S>(key: &chrono::DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&key.to_rfc3339())
}

pub fn from_rfc3339_fixed<'d, D>(deserializer: D) -> Result<chrono::DateTime<FixedOffset>, D::Error>
where
    D: serde::Deserializer<'d>,
{
    use serde::de::Error;
    String::deserialize(deserializer)
        .and_then(|string| {
            DateTime::<FixedOffset>::parse_from_rfc3339(&string)
                .map_err(|err| Error::custom(err.to_string()))
        }).map(|dt| dt.with_timezone(&dt.timezone()))
}

pub fn as_rfc3339_fixed<S>(
    key: &chrono::DateTime<FixedOffset>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&key.to_rfc3339())
}
