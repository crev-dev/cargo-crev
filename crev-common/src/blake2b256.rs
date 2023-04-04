use blake2::Blake2b;
use blake2::digest::consts::U32;

pub type Blake2b256 = Blake2b<U32>;
