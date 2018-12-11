//! This crate contains only code handling data types
//! used by `crev`, without getting into details
//! how actually `crev` works (where and how it manages data).

#[macro_use]
extern crate failure;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate derive_builder;

use common_failures::prelude::*;

pub mod digest;
pub mod id;
pub mod level;
mod prelude;
pub mod proof;
pub mod url;
pub mod util;

pub use crate::{
    digest::Digest,
    id::{Id, OwnId, PubId},
    level::Level,
    proof::review::Review,
    url::Url,
};

/// Current API version
pub fn current_version() -> i64 {
    -1 // still in alpha, so less than 0
}

#[cfg(test)]
mod tests;
