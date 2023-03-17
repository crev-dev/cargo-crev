use anyhow::{bail, Result};
use crev_common::{run_with_shell_cmd, CancelledError};
use crev_data::{proof, proof::content::ContentExt};
use crev_lib::{local::Local, util::get_documentation_for};
use std::{
    env, ffi,
    fmt::Write,
    path::{Path, PathBuf},
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
    let dir = tempfile::tempdir()?;
    let file_path = dir.path().join("crev.review.yaml");
    std::fs::write(&file_path, text)?;

    let starting_ts = std::fs::metadata(&file_path)?
        .modified()
        .unwrap_or_else(|_| std::time::SystemTime::now());

    edit_file(&file_path)?;

    let modified_ts = std::fs::metadata(&file_path)?
        .modified()
        .unwrap_or_else(|_| std::time::SystemTime::now());

    Ok((
        std::fs::read_to_string(&file_path)?,
        starting_ts != modified_ts,
    ))
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
            let reply = rprompt::prompt_reply_from_bufread(
                &mut std::io::stdin().lock(),
                &mut std::io::stderr(),
                "Commit anyway? (y/N/q) ",
            )?;

            match reply.as_str() {
                "y" | "Y" => return Ok(text),
                "q" | "Q" => return Err(CancelledError::ByUser.into()),
                "n" | "N" | "" | _ => continue,
            }
        }
        return Ok(text);
    }
}

pub fn edit_file(path: &Path) -> Result<()> {
    let editor = get_editor_to_use()?;

    let status = run_with_shell_cmd(&editor, Some(path))?;

    if !status.success() {
        bail!(
            "Can't launch editor {}: {}",
            editor.to_str().unwrap_or("?"),
            status
        );
    }
    Ok(())
}

pub fn edit_proof_content_iteractively<C: proof::ContentWithDraft>(
    content: &C,
    previous_date: Option<&proof::Date>,
    base_version: Option<&crev_data::Version>,
    extra_leading_comment: Option<&str>,
    extra_follow_content_fn: impl FnOnce(&mut String) -> Result<()>,
) -> Result<C> {
    let mut text = String::new();
    if let Some(date) = previous_date {
        writeln!(
            &mut text,
            "# Overwriting existing proof created on {}",
            date.to_rfc3339()
        )?;
    }
    let draft = content.to_draft();

    writeln!(&mut text, "# {}", draft.title())?;
    if let Some(extra_comment) = extra_leading_comment {
        writeln!(&mut text, "# {extra_comment}")?;
    }
    if let Some(base_version) = base_version {
        writeln!(&mut text, "# Diff base version: {base_version}")?;
    }
    text.write_str(draft.body())?;
    (extra_follow_content_fn)(&mut text)?;
    text.write_str("\n\n")?;
    for line in get_documentation_for(content).lines() {
        writeln!(&mut text, "# {line}")?;
    }
    loop {
        text = edit_text_iteractively_until_writen_to(&text)?;
        match content.apply_draft(&text) {
            Err(e) => {
                eprintln!("There was an error parsing content: {e}");
                crev_common::try_again_or_cancel()?;
            }
            Ok(content) => {
                if let Err(e) = content.ensure_serializes_to_valid_proof() {
                    eprintln!("There was an error validating serialized proof: {e}");
                    crev_common::try_again_or_cancel()?;
                } else {
                    return Ok(content);
                }
            }
        }
    }
}

/// interactively edit currnent user's yaml config file
pub fn edit_user_config(local: &Local) -> Result<()> {
    let config = local.load_user_config()?;
    let mut text = serde_yaml::to_string(&config)?;
    loop {
        text = edit_text_iteractively(&text)?;
        match serde_yaml::from_str(&text) {
            Err(e) => {
                eprintln!("There was an error parsing content: {e}");
                crev_common::try_again_or_cancel()?;
            }
            Ok(edited_config) => {
                return Ok(local.store_user_config(&edited_config)?);
            }
        }
    }
}

/// interactively edit readme file of the current user's proof repo
pub fn edit_readme(local: &Local) -> Result<()> {
    edit_file(&local.get_proofs_dir_path()?.join("README.md"))?;
    local.proof_dir_git_add_path(&PathBuf::from("README.md"))?;
    Ok(())
}
