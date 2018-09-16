use app_dirs;
use base64;
use crev_common;
use crev_data::proof;
use std::{
    self, env, ffi, fmt, fs,
    io::{self, Read, Write},
    path::Path,
    process,
};
use tempdir;
use Result;

pub const APP_INFO: app_dirs::AppInfo = app_dirs::AppInfo {
    name: "crev",
    author: "Dawid Ciężarkiewicz",
};

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
    fs::create_dir_all(path.parent().expect("Not a root path"))?;
    let tmp_path = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp_path)?;
    file.write_all(&s.as_bytes())?;
    file.flush()?;
    drop(file);
    fs::rename(tmp_path, path)?;
    Ok(())
}

pub fn store_to_file_with(path: &Path, f: impl Fn(&mut io::Write) -> Result<()>) -> Result<()> {
    fs::create_dir_all(path.parent().expect("Not a root path"))?;
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

pub fn edit_proof_content_iteractively(
    content: &proof::Content,
    type_: proof::ProofType,
) -> Result<proof::Content> {
    let mut text = content.to_string();
    loop {
        text = edit_text_iteractively(text)?;
        match proof::Content::parse(&text, type_) {
            Err(e) => {
                eprintln!("There was an error parsing content: {}", e);
                if !crev_common::yes_or_no_was_y("Try again (y/n) ")? {
                    bail!("User canceled");
                }
            }
            Ok(content) => return Ok(content),
        }
    }
}

pub fn random_id_str() -> String {
    use rand::{self, Rng};
    let project_id: Vec<u8> = rand::thread_rng()
        .sample_iter(&rand::distributions::Standard)
        .take(32)
        .collect();
    base64::encode_config(&project_id, base64::URL_SAFE)
}

pub fn err_eprint_and_ignore<O, E: fmt::Display>(res: std::result::Result<O, E>) {
    match res {
        Err(e) => eprintln!("{}", e),
        Ok(_) => {}
    }
}
