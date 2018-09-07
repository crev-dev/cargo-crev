extern crate crev_data;
extern crate argonautica;
extern crate crev_common;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
extern crate miscreant;
extern crate app_dirs;
extern crate base64;
extern crate rand;
extern crate rpassword;
extern crate rprompt;
extern crate tempdir;
extern crate common_failures;

use common_failures::prelude::*;

#[macro_use]
extern crate failure;


pub mod id;

mod util;

#[cfg(test)]
mod tests;

