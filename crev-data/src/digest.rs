use std::fmt;

pub struct Digest(pub Vec<u8>);

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&base64::encode_config(&self.0, base64::URL_SAFE))
    }
}
