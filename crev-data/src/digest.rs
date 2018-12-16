use std::fmt;

pub struct Digest(Vec<u8>);

impl Digest {
    pub fn from_vec(v: Vec<u8>) -> Self {
        // we only need 256bit security
        assert_eq!(v.len(), 32);
        Digest(v)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&crev_common::base64_encode(&self.0))
    }
}
