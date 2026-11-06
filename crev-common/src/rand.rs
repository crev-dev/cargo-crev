#[must_use]
pub fn random_vec(len: usize) -> Vec<u8> {
    let mut out = vec![0u8; len];
    rand::fill(&mut out);
    out
}
