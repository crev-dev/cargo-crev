use anyhow::{bail, Result};
use crev_common::{read_file_to_string, run_with_shell_cmd};
use crev_data::{proof, proof::content::ContentExt, Id, PublicId};
use crev_lib::{local::Local, util::get_documentation_for, TrustProofType};
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
    let dir = tempdir::TempDir::new("crev")?;
    let file_path = dir.path().join("crev.review");
    std::fs::write(&file_path, text)?;

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
    base_version: Option<&semver::Version>,
) -> Result<C> {
    let mut text = String::new();
    if let Some(date) = previous_date {
        write!(
            &mut text,
            "# Overwriting existing proof created on {}\n",
            date.to_rfc3339()
        )?;
    }
    let draft = content.to_draft();

    write!(&mut text, "# {}\n", draft.title())?;
    if let Some(base_version) = base_version {
        write!(&mut text, "# Diff base version: {}\n", base_version)?;
    }
    text.write_str(&draft.body())?;
    text.write_str("\n\n")?;
    for line in get_documentation_for(content).lines() {
        write!(&mut text, "# {}\n", line)?;
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

/// interactively edit currnent user's yaml config file
pub fn edit_user_config(local: &Local) -> Result<()> {
    let config = local.load_user_config()?;
    let mut text = serde_yaml::to_string(&config)?;
    loop {
        text = edit_text_iteractively(&text)?;
        match serde_yaml::from_str(&text) {
            Err(e) => {
                eprintln!("There was an error parsing content: {}", e);
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

/// Opens editor with a new trust proof for given Ids
///
/// Currently ignores previous proofs
pub fn build_trust_proof_interactively(
    local: &Local,
    from_id: &PublicId,
    ids: Vec<Id>,
    trust_or_distrust: TrustProofType,
) -> Result<proof::trust::Trust> {
    let trust = local.build_trust_proof(from_id, ids, trust_or_distrust)?;

    // TODO: Look up previous trust proof?
    Ok(edit_proof_content_iteractively(&trust, None, None)?)
}
