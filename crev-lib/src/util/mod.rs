pub mod git;

use crate::prelude::*;
use crev_common;
use crev_data::proof::{self, ContentExt};
use failure::bail;
use git2;
use std::{self, env, ffi, fmt::Write as FmtWrite, fs, io, io::Write, path::Path};
use tempdir;

pub use crev_common::{
    read_file_to_string, run_with_shell_cmd, store_str_to_file, store_to_file_with,
};

fn get_git_default_editor() -> Result<String> {
    let cfg = git2::Config::open_default()?;
    Ok(cfg.get_string("core.editor")?)
}

fn get_editor_to_use() -> Result<ffi::OsString> {
    Ok(if let Some(v) = env::var_os("VISUAL") {
        v
    } else if let Some(v) = env::var_os("EDITOR") {
        v
    } else if let Ok(v) = get_git_default_editor() {
        v.into()
    } else {
        "vi".into()
    })
}

/// Retruns the edited string, and bool indicating if the file was ever written to/ (saved).
fn edit_text_iteractively_raw(text: &str) -> Result<(String, bool)> {
    let dir = tempdir::TempDir::new("crev")?;
    let file_path = dir.path().join("crev.review");
    let mut file = fs::File::create(&file_path)?;
    file.write_all(text.as_bytes())?;
    file.flush()?;
    drop(file);

    let starting_ts = std::fs::metadata(&file_path)?
        .modified()
        .unwrap_or_else(|_| std::time::SystemTime::now());

    edit_file(&file_path)?;

    let modified_ts = std::fs::metadata(&file_path)?
        .modified()
        .unwrap_or_else(|_| std::time::SystemTime::now());

    Ok((read_file_to_string(&file_path)?, starting_ts != modified_ts))
}

pub fn edit_text_iteractively(text: &str) -> Result<String> {
    Ok(edit_text_iteractively_raw(text)?.0)
}

pub fn edit_text_iteractively_until_writen_to(text: &str) -> Result<String> {
    loop {
        let (text, modified) = edit_text_iteractively_raw(text)?;
        if !modified {
            eprintln!(
                "File not written to. Make sure to save it at least once to confirm the data."
            );
            crev_common::try_again_or_cancel()?;
            continue;
        }

        return Ok(text);
    }
}

pub fn edit_file(path: &Path) -> Result<()> {
    let editor = get_editor_to_use()?;

    let status = run_with_shell_cmd(editor, Some(path))?;

    if !status.success() {
        bail!("Editor returned {}", status);
    }
    Ok(())
}

pub fn get_documentation_for(content: &impl proof::Content) -> &'static str {
    match content.type_name() {
        "trust" => include_str!("../../rc/doc/editing-trust.md"),
        "code review" => include_str!("../../rc/doc/editing-code-review.md"),
        "package review" => include_str!("../../rc/doc/editing-package-review.md"),
        _ => "unknown proof type",
    }
}

pub fn edit_proof_content_iteractively<C: proof::ContentWithDraft>(
    content: &C,
    previous_date: Option<&proof::Date>,
    base_version: Option<&semver::Version>,
) -> Result<C> {
    let mut text = String::new();
    if let Some(date) = previous_date {
        text.write_str(&format!(
            "# Overwriting existing proof created on {}\n",
            date.to_rfc3339()
        ))?;
    }
    let draft = content.to_draft();

    text.write_str(&format!("# {}\n", draft.title()))?;
    if let Some(base_version) = base_version {
        text.write_str(&format!("# Diff base version: {}\n", base_version))?;
    }
    text.write_str(&draft.body())?;
    text.write_str("\n\n")?;
    for line in get_documentation_for(content).lines() {
        text.write_fmt(format_args!("# {}\n", line))?;
    }
    loop {
        text = edit_text_iteractively_until_writen_to(&text)?;
        match content.apply_draft(&text) {
            Err(e) => {
                eprintln!("There was an error parsing content: {}", e);
                crev_common::try_again_or_cancel()?;
            }
            Ok(content) => {
                if let Err(e) = content.ensure_serializes_to_valid_proof() {
                    eprintln!("There was an error validating serialized proof: {}", e);
                    crev_common::try_again_or_cancel()?;
                } else {
                    return Ok(content);
                }
            }
        }
    }
}

pub fn err_eprint_and_ignore<O, E: std::error::Error>(res: std::result::Result<O, E>) -> Option<O> {
    match res {
        Err(e) => {
            eprintln!("{}", e);
            None
        }
        Ok(o) => Some(o),
    }
}

#[cfg(target_family = "unix")]
pub fn chmod_path_to_600(path: &Path) -> io::Result<()> {
    use std::{fs::Permissions, os::unix::fs::PermissionsExt};

    std::fs::set_permissions(path, Permissions::from_mode(0o600))
}

#[cfg(not(target_family = "unix"))]
pub fn chmod_path_to_600(path: &Path) -> io::Result<()> {
    Ok(())
}
