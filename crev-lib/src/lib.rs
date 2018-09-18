#[macro_use]
extern crate serde_derive;

use common_failures::prelude::*;

#[macro_use]
extern crate failure;

pub mod id;

pub mod local;
pub mod proof;
pub mod recursive_digest;
pub mod repo;
pub mod staging;
pub mod trustdb;
mod util;

#[cfg(test)]
mod tests;
