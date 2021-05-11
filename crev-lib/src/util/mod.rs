use crev_common::sanitize_name_for_fs;
pub use crev_common::{run_with_shell_cmd, store_str_to_file, store_to_file_with};
use crev_data::proof;
use std::{
    self,
    ffi::OsStr,
    io,
    path::{Path, PathBuf},
};

pub mod git;

pub fn get_documentation_for(content: &impl proof::Content) -> &'static str {
    match content.kind() {
        proof::Trust::KIND => include_str!("../../rc/doc/editing-trust.md"),
        proof::CodeReview::KIND => include_str!("../../rc/doc/editing-code-review.md"),
        proof::PackageReview::KIND => include_str!("../../rc/doc/editing-package-review.md"),
        _ => "unknown proof type",
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

fn mark_dangerous_name(
    orig_name: &OsStr,
    parent: &Path,
    idx: usize,
    changes: &mut Vec<String>,
) -> PathBuf {
    let orig_name = match orig_name.to_str() {
        Some(s) => s,
        None => {
            let name = Path::new(orig_name);
            let alt =
                sanitize_name_for_fs(&format!("{} {}", name.display(), idx)).with_extension("CREV");
            changes.push(format!(
                "Non-Unicode filename '{}' renamed to '{}'",
                name.display(),
                alt.display()
            ));
            return alt;
        }
    };

    // You don't get to spoof anti-spoofing measure
    if orig_name.contains(".CREV") || orig_name.contains("-CREV") || orig_name.contains("CREV.") {
        let alt = sanitize_name_for_fs(orig_name).with_extension("CREV");
        changes.push(format!(
            "File '{}' is not from cargo-crev. Renamed to '{}'",
            orig_name,
            alt.display()
        ));
        return alt;
    }

    // file-systems may be case-insensitive
    match orig_name.to_ascii_lowercase().as_str() {
        "cargo.toml" => {
            changes
                .push("Cargo.toml could cause IDEs automatically build dependencies".to_string());
            return PathBuf::from("Cargo.toml.CREV");
        }
        ".cargo" => {
            changes.push(".cargo config can replace linkers, source of dependencies".to_string());
            return PathBuf::from("CREV.cargo");
        }
        "config" | "config.toml" if parent.file_name().unwrap() == "cargo" => {
            changes.push("cargo/config can replace linkers, source of dependencies".to_string());
            return PathBuf::from("config.CREV");
        }
        "rust-toolchain" | "rust-toolchain.toml" => {
            changes
                .push("rust-toolchain file could unexpectedly replace your compiler".to_string());
            return PathBuf::from(format!("{}.CREV", orig_name));
        }
        n if n.starts_with(".") => {
            changes.push(format!("Hidden file: '{}'", orig_name));
            return PathBuf::from(format!("CREV{}", orig_name));
        }
        n if n.len() > 250 => {
            let alt = sanitize_name_for_fs(orig_name).with_extension("CREV");
            changes.push(format!(
                "Long file name: '{}' renamed to '{}'",
                orig_name,
                alt.display()
            ));
            alt
        }
        // Are there legit use-cases for Unicode names? Killing it avoids risk of homograph or BIDI spoofing
        n if n.as_bytes().iter().any(|&c| {
            c < b' '
                || c >= 0x7F
                || matches!(
                    c,
                    b'\"' | b'`' | b'$' | b'<' | b'\\' | b'*' | b'?' | b'{' | b'['
                )
        }) =>
        {
            let alt = sanitize_name_for_fs(orig_name).with_extension("CREV");
            changes.push(format!(
                "Name contains metacharacters, unprintables, or non-ASCII: '{}' renamed to '{}'",
                orig_name,
                alt.display()
            ));
            alt
        }
        _ => PathBuf::from(orig_name),
    }
}

/// Make a copy of the directory, but skip or rename all files that are potentially dangerous in Cargo projects
pub fn copy_dir_sanitized(
    src_dir: &Path,
    dest_dir: &Path,
    changes: &mut Vec<String>,
) -> std::io::Result<()> {
    for (n, entry) in std::fs::read_dir(src_dir)?.enumerate() {
        let entry = entry?;
        let src_path = entry.path();
        let safe_file_name = mark_dangerous_name(&entry.file_name(), src_dir, n, changes);
        let dest_path = dest_dir.join(safe_file_name);
        let ft = entry.file_type()?;
        if ft.is_symlink() {
            changes.push(format!(
                "Symlink not copied. The symlink is in '{}'",
                src_path.display()
            ));
        } else if ft.is_file() {
            std::fs::copy(entry.path(), &dest_path)?;
        } else {
            assert!(ft.is_dir());
            let _ = std::fs::create_dir(&dest_path);
            copy_dir_sanitized(&entry.path(), &dest_path, changes)?;
        }
    }
    Ok(())
}
