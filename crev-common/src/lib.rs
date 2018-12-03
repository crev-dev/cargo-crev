//! Bunch of code that is auxiliary and common for all `crev`

pub mod serde;

use blake2;
use chrono;

use blake2::{digest::FixedOutput, Digest};
use rpassword;
use rprompt;
use std::{
    env, fs,
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
    input: &mut impl blake2::digest::Input,
) -> io::Result<()> {
    let file = fs::File::open(path)?;

    let mut reader = io::BufReader::new(file);

    loop {
        let length = {
            let buffer = reader.fill_buf()?;
            input.input(buffer);
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

pub fn read_passphrase() -> io::Result<String> {
    if let Ok(pass) = env::var("CREV_PASSPHRASE") {
        eprint!("Using passphrase set in CREV_PASSPHRASE\n");
        return Ok(pass);
    }
    eprint!("Enter passphrase to unlock: ");
    rpassword::read_password()
}

pub fn read_new_passphrase() -> io::Result<String> {
    if let Ok(pass) = env::var("CREV_PASSPHRASE") {
        eprint!("Using passphrase set in CREV_PASSPHRASE\n");
        return Ok(pass);
    }
    loop {
        eprint!("Enter new passphrase: ");
        let p1 = rpassword::read_password()?;
        eprint!("Enter new passphrase again: ");
        let p2 = rpassword::read_password()?;
        if p1 == p2 {
            return Ok(p1);
        }
        eprintln!("\nPassphrases don't match, try again.");
    }
}
