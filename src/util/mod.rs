use common_failures::prelude::*;
use rpassword;
use rprompt;
use std::{
    env, ffi, fs, io,
    io::{Read, Write},
    path::{Path, PathBuf},
    process,
};
use tempdir;
use Result;
use {id, repo, review, util};

pub mod serde;

use app_dirs::{app_root, get_app_root, AppDataType, AppInfo};

pub const APP_INFO: AppInfo = AppInfo {
    name: "crev",
    author: "Dawid Ciężarkiewicz",
};

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
fn get_editor_to_use() -> ffi::OsString {
    if let Some(v) = env::var_os("VISUAL") {
        return v;
    } else if let Some(v) = env::var_os("EDITOR") {
        return v;
    } else {
        return "vi".into();
    }
}

pub fn read_file_to_string(path: &Path) -> Result<String> {
    let mut file = fs::File::open(&path)?;
    let mut res = String::new();
    file.read_to_string(&mut res)?;

    Ok(res)
}

pub fn store_str_to_file(path: &Path, s: &str) -> Result<()> {
    fs::create_dir_all(path.parent().expect("Not a root path"));
    let tmp_path = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp_path)?;
    file.write_all(&s.as_bytes())?;
    file.flush()?;
    drop(file);
    fs::rename(tmp_path, path)?;
    Ok(())
}

pub fn store_to_file_with(path: &Path, f: impl Fn(&mut io::Write) -> Result<()>) -> Result<()> {
    fs::create_dir_all(path.parent().expect("Not a root path"));
    let tmp_path = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp_path)?;
    f(&mut file)?;
    file.flush()?;
    drop(file);
    fs::rename(tmp_path, path)?;
    Ok(())
}

fn edit_text_iteractively(text: String) -> Result<String> {
    let editor = get_editor_to_use();
    let dir = tempdir::TempDir::new("crev")?;
    let file_path = dir.path().join("crev.review");
    let mut file = fs::File::create(&file_path)?;
    file.write_all(text.as_bytes())?;
    file.flush()?;
    drop(file);

    let status = process::Command::new(editor).arg(&file_path).status()?;

    if !status.success() {
        bail!("Editor returned {}", status);
    }

    Ok(read_file_to_string(&file_path)?)
}

fn yes_or_no_was_y() -> Result<bool> {
    loop {
        let reply = rprompt::prompt_reply_stderr("Try again (y/n)")?;

        match reply.as_str() {
            "y" | "Y" => return Ok(true),
            "n" | "N" => return Ok(false),
            _ => {}
        }
    }
}

pub fn edit_review_iteractively(review: review::Review) -> Result<review::Review> {
    let mut text = review.to_string();
    loop {
        text = edit_text_iteractively(text)?;
        match review::Review::parse(&text) {
            Err(e) => {
                eprintln!("There was an error parsing review: {}", e);
                if !yes_or_no_was_y()? {
                    bail!("User canceled");
                }
            }
            Ok(review) => return Ok(review),
        }
    }
}
