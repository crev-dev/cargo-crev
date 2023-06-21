use blake2::digest::consts::U32;
use blake2::Blake2b;

pub type Blake2b256 = Blake2b<U32>;
