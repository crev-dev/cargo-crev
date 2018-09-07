//! This crate contains only code handling data types
//! used by `crev`, without getting into details
//! how actually `crev` works (where and how it manages data).

extern crate argonautica;
extern crate base64;
extern crate blake2;
extern crate chrono;
extern crate common_failures;
extern crate ed25519_dalek;
#[macro_use]
extern crate failure;
extern crate hex;
extern crate miscreant;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
#[macro_use]
extern crate derive_builder;

use common_failures::prelude::*;

pub mod review {
    pub use super::proof::review::*;
}
pub mod trust {
    pub use super::proof::trust::*;
}
pub mod id;
pub mod level;
pub mod proof;
pub mod util;

#[cfg(test)]
mod tests;
