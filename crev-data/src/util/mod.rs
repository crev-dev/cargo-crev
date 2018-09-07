pub(crate) mod serde;

use base64;
use blake2::{self, digest::FixedOutput, Digest};
use chrono::{self, prelude::*};
use rand::{self, Rng};

pub fn now() -> DateTime<FixedOffset> {
    let date = chrono::offset::Local::now();
    date.with_timezone(&date.offset())
}

pub fn blaze2sum(bytes: &[u8]) -> Vec<u8> {
    let mut hasher = blake2::Blake2b::new();
    hasher.input(bytes);
    hasher.fixed_result().to_vec()
}

pub fn random_id_str() -> String {
    let project_id: Vec<u8> = rand::thread_rng()
        .sample_iter(&rand::distributions::Standard)
        .take(32)
        .collect();
    base64::encode_config(&project_id, base64::URL_SAFE)
}
