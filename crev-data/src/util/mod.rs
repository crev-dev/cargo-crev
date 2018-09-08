use base64;
use rand::{self, Rng};

pub fn random_id_str() -> String {
    let project_id: Vec<u8> = rand::thread_rng()
        .sample_iter(&rand::distributions::Standard)
        .take(32)
        .collect();
    base64::encode_config(&project_id, base64::URL_SAFE)
}
