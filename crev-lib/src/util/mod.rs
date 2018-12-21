use crate::Result;
use app_dirs;
use crev_common;
use crev_data::proof;
use std::fmt::Write as FmtWrite;
use std::{
    self, env, ffi, fs,
    io::{self, Read, Write},
    path::Path,
    process,
};
use tempdir;

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

pub fn store_to_file_with(path: &Path, f: impl Fn(&mut dyn io::Write) -> Result<()>) -> Result<()> {
    fs::create_dir_all(path.parent().expect("Not a root path"))?;
    let tmp_path = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp_path)?;
    f(&mut file)?;
    file.flush()?;
    file.sync_data()?;
    drop(file);
    fs::rename(tmp_path, path)?;
    Ok(())
}

fn edit_text_iteractively(text: &str) -> Result<String> {
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

pub fn edit_file(path: &Path) -> Result<()> {
    let editor = get_editor_to_use();
    let status = process::Command::new(editor).arg(&path).status()?;

    if !status.success() {
        bail!("Editor returned {}", status);
    }
    Ok(())
}

pub fn get_documentation_for(content: &proof::Content) -> &'static str {
    use crev_data::proof::Content;
    match content {
        Content::Trust(_) => include_str!("../../rc/doc/editing-trust.md"),
        Content::Code(_) => include_str!("../../rc/doc/editing-code-review.md"),
        Content::Package(_) => include_str!("../../rc/doc/editing-package-review.md"),
    }
}

pub fn edit_proof_content_iteractively(content: &proof::Content) -> Result<proof::Content> {
    let mut text = String::new();

    text.write_str(&format!("# {}\n", content.draft_title()))?;
    text.write_str(&content.to_draft_string())?;
    text.write_str("\n\n")?;
    for line in get_documentation_for(content).lines() {
        text.write_fmt(format_args!("# {}\n", line))?;
    }
    loop {
        text = edit_text_iteractively(&text)?;
        match proof::Content::parse_draft(content, &text) {
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

pub fn err_eprint_and_ignore<O, E: std::error::Error>(res: std::result::Result<O, E>) -> bool {
    match res {
        Err(e) => {
            eprintln!("{}", e);
            false
        }
        Ok(_) => true,
    }
}
