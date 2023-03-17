use std::fmt;

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct Digest([u8; 32]);

impl From<[u8; 32]> for Digest {
    fn from(arr: [u8; 32]) -> Self {
        Self(arr)
    }
}

impl Digest {
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    #[must_use]
    pub fn from_vec(vec: Vec<u8>) -> Option<Self> {
        if vec.len() == 32 {
            let mut out = [0; 32];
            out.copy_from_slice(&vec);
            Some(Self(out))
        } else {
            None
        }
    }
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() == 32 {
            let mut out = [0; 32];
            out.copy_from_slice(bytes);
            Some(Self(out))
        } else {
            None
        }
    }

    #[must_use]
    pub fn into_vec(self) -> Vec<u8> {
        self.as_slice().to_vec()
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&crev_common::base64_encode(&self.0))
    }
}
