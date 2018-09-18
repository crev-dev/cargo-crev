//! Bunch of code that is auxiliary and common for all `crev`

pub mod serde;

use blake2;
use chrono;

use rprompt;

use blake2::{digest::FixedOutput, Digest};
use std::{
    fs,
    io::{self, BufRead},
    path::Path,
};

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
    let mut hasher = blake2::Blake2b::new();
    read_file_to_digest_input(path, &mut hasher)?;
    Ok(hasher.fixed_result().to_vec())
}

pub fn read_file_to_digest_input(
    path: &Path,
    input: &mut dyn blake2::digest::Input,
) -> io::Result<()> {
    let file = fs::File::open(path)?;

    let mut reader = io::BufReader::new(file);

    loop {
        let length = {
            let buffer = reader.fill_buf()?;
            input.process(buffer);
            buffer.len()
        };
        if length == 0 {
            break;
        }
        reader.consume(length);
    }

    Ok(())
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
