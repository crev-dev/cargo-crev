use rand::{self, Rng};

#[must_use]
pub fn random_vec(len: usize) -> Vec<u8> {
    rand::thread_rng()
        .sample_iter(&rand::distributions::Standard)
        .take(len)
        .collect()
}
