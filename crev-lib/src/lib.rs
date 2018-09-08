extern crate crev_data;
extern crate argonautica;
extern crate crev_common;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
extern crate serde_cbor;
extern crate miscreant;
extern crate walkdir;
extern crate app_dirs;
extern crate base64;
extern crate rand;
extern crate tempdir;
extern crate git2;
extern crate common_failures;

use common_failures::prelude::*;

#[macro_use]
extern crate failure;


pub mod id;

mod util;
pub mod repo;
pub mod local;
pub mod index;
pub mod trust_graph;

#[cfg(test)]
mod tests;

