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

fn default_trust_params() -> trustdb::TrustDistanceParams {
    trustdb::TrustDistanceParams  {
        max_distance: 10,
        high_trust_distance: 0,
        medium_trust_distance: 1,
        low_trust_distance: 5,
    }
}

#[cfg(test)]
mod tests;
