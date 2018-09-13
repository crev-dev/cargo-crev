extern crate argonautica;
extern crate crev_common;
extern crate crev_data;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate app_dirs;
extern crate base64;
extern crate chrono;
extern crate common_failures;
extern crate git2;
extern crate miscreant;
extern crate rand;
extern crate serde_cbor;
extern crate serde_yaml;
extern crate tempdir;
extern crate walkdir;

use common_failures::prelude::*;

#[macro_use]
extern crate failure;

pub mod id;

pub mod local;
pub mod proof;
pub mod repo;
pub mod staging;
pub mod trustdb;
mod util;

#[cfg(test)]
mod tests;
