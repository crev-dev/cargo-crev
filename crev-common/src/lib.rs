//! Bunch of code that is auxiliary and common for all `crev`

pub mod serde;
extern crate base64;
extern crate hex;
extern crate chrono;
extern crate serde_yaml;
extern crate blake2;
extern crate rprompt;

use blake2::{digest::FixedOutput, Digest};
use std::{fs, io::{self, BufRead}, path::Path};

/// Now with a fixed offset of the current system timezone
pub fn now() -> chrono::DateTime<chrono::offset::FixedOffset> {
    let date = chrono::offset::Local::now();
    date.with_timezone(&date.offset())
}

pub fn blake2sum(bytes: &[u8]) -> Vec<u8> {
    let mut hasher = blake2::Blake2b::new();
    hasher.input(bytes);
    hasher.fixed_result().to_vec()
}

pub fn blake2sum_file(path: &Path) -> io::Result<Vec<u8>> {
    let file = fs::File::open(path)?;

    let mut reader = io::BufReader::new(file);
    let mut hasher = blake2::Blake2b::new();

    loop {
        let length = {
            let buffer = reader.fill_buf()?;
            hasher.input(buffer);
            buffer.len()
        };
        if length == 0 {
            break;
        }
        reader.consume(length);
    }
    Ok(hasher.fixed_result().to_vec())
}

pub fn yes_or_no_was_y(msg: &str) -> io::Result<bool> {
    loop {
        let reply = rprompt::prompt_reply_stderr(msg)?;

        match reply.as_str() {
            "y" | "Y" => return Ok(true),
            "n" | "N" => return Ok(false),
            _ => {}
        }
    }
}
