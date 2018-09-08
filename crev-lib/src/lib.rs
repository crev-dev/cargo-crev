extern crate argonautica;
extern crate crev_common;
extern crate crev_data;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate app_dirs;
extern crate base64;
extern crate common_failures;
extern crate git2;
extern crate miscreant;
extern crate rand;
extern crate serde_cbor;
extern crate serde_yaml;
extern crate tempdir;
extern crate walkdir;
extern crate chrono;

use common_failures::prelude::*;

#[macro_use]
extern crate failure;

pub mod id;

pub mod index;
pub mod local;
pub mod repo;
pub mod proof;
pub mod trust_graph;
mod util;

#[cfg(test)]
mod tests;
