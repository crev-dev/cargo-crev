pub mod git;

use crate::prelude::*;
use crev_common;
use crev_data::proof;
use failure::{bail, format_err};
use git2;
use std::{
    self, env,
    ffi::{self, OsString},
    fmt::Write as FmtWrite,
    fs,
    io::Write,
    path::Path,
    process,
};
use tempdir;

pub use crev_common::{read_file_to_string, store_str_to_file, store_to_file_with};

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

pub fn edit_text_iteractively(text: &str) -> Result<String> {
    let dir = tempdir::TempDir::new("crev")?;
    let file_path = dir.path().join("crev.review");
    let mut file = fs::File::create(&file_path)?;
    file.write_all(text.as_bytes())?;
    file.flush()?;
    drop(file);

    edit_file(&file_path)?;

    Ok(read_file_to_string(&file_path)?)
}

pub fn run_with_shell_cmd(cmd: OsString, path: &Path) -> Result<std::process::ExitStatus> {
    Ok(if cfg!(windows) {
        let mut proc = process::Command::new(cmd.clone());
        proc.arg(path);
        proc
    } else if cfg!(unix) {
        let mut proc = process::Command::new("/bin/sh");
        proc.arg("-c").arg(format!(
            "{} {}",
            cmd.clone().into_string().map_err(|_| format_err!(
                "command not a valid Unicode: {}",
                cmd.to_string_lossy()
            ))?,
            shell_escape::escape(path.display().to_string().into())
        ));
        proc
    } else {
        panic!("What platform are you running this on? Please submit a PR!");
    }
    .status()
    .with_context(|_e| format_err!("Couldn't start the command: {}", cmd.to_string_lossy()))?)
}

pub fn edit_file(path: &Path) -> Result<()> {
    let editor = get_editor_to_use()?;

    let status = run_with_shell_cmd(editor, path)?;

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

pub fn edit_proof_content_iteractively(
    content: &proof::Content,
    previous_date: Option<proof::Date>,
) -> Result<proof::Content> {
    let mut text = String::new();
    if let Some(date) = previous_date {
        text.write_str(&format!(
            "# Overwriting existing proof created on {}\n",
            date.to_rfc3339()
        ))?;
    }

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

pub fn err_eprint_and_ignore<O, E: std::error::Error>(res: std::result::Result<O, E>) -> Option<O> {
    match res {
        Err(e) => {
            eprintln!("{}", e);
            None
        }
        Ok(o) => Some(o),
    }
}
