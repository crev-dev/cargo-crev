use crate::{Error, Result};
pub use crev_common::{
    read_file_to_string, run_with_shell_cmd, store_str_to_file, store_to_file_with,
};
use crev_data::proof::{self, ContentExt};
use std::{
    self, env, ffi,
    fmt::Write as FmtWrite,
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};

pub mod git;

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
        Error::EditorLaunch(status.code().unwrap_or(-1));
    }
    Ok(())
}

pub fn get_documentation_for(content: &impl proof::Content) -> &'static str {
    match content.kind() {
        proof::Trust::KIND => include_str!("../../rc/doc/editing-trust.md"),
        proof::CodeReview::KIND => include_str!("../../rc/doc/editing-code-review.md"),
        proof::PackageReview::KIND => include_str!("../../rc/doc/editing-package-review.md"),
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
        write!(
            &mut text,
            "# Overwriting existing proof created on {}\n",
            date.to_rfc3339()
        )
        .map_err(|_| Error::FmtIO)?;
    }
    let draft = content.to_draft();

    write!(&mut text, "# {}\n", draft.title()).map_err(|_| Error::FmtIO)?;
    if let Some(base_version) = base_version {
        write!(&mut text, "# Diff base version: {}\n", base_version).map_err(|_| Error::FmtIO)?;
    }
    text.write_str(&draft.body()).map_err(|_| Error::FmtIO)?;
    text.write_str("\n\n").map_err(|_| Error::FmtIO)?;
    for line in get_documentation_for(content).lines() {
        write!(&mut text, "# {}\n", line).map_err(|_| Error::FmtIO)?;
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

pub fn get_recursive_digest_for_paths(
    root_path: &Path,
    paths: fnv::FnvHashSet<PathBuf>,
) -> std::result::Result<Vec<u8>, crev_recursive_digest::DigestError> {
    let h = crev_recursive_digest::RecursiveDigest::<crev_common::Blake2b256, _, _>::new()
        .filter(|entry| {
            let rel_path = entry
                .path()
                .strip_prefix(&root_path)
                .expect("must be prefix");
            paths.contains(rel_path)
        })
        .build();

    h.get_digest_of(root_path)
}

pub fn get_recursive_digest_for_dir(
    root_path: &Path,
    rel_path_ignore_list: &fnv::FnvHashSet<PathBuf>,
) -> std::result::Result<Vec<u8>, crev_recursive_digest::DigestError> {
    let h = crev_recursive_digest::RecursiveDigest::<crev_common::Blake2b256, _, _>::new()
        .filter(|entry| {
            let rel_path = entry
                .path()
                .strip_prefix(&root_path)
                .expect("must be prefix");
            !rel_path_ignore_list.contains(rel_path)
        })
        .build();

    h.get_digest_of(root_path)
}
