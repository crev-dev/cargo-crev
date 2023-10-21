use bstr::ByteSlice;
use crev_common::sanitize_name_for_fs;
pub use crev_common::{run_with_shell_cmd, store_str_to_file, store_to_file_with};
use crev_data::proof;
use std::{
    self,
    borrow::Cow,
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
) -> std::result::Result<crev_data::Digest, crev_recursive_digest::DigestError> {
    let h = crev_recursive_digest::RecursiveDigest::<crev_common::Blake2b256, _, _>::new()
        .filter(|entry| {
            let rel_path = entry
                .path()
                .strip_prefix(root_path)
                .expect("must be prefix");
            paths.contains(rel_path)
        })
        .build();

    let digest_vec = h.get_digest_of(root_path)?;
    Ok(crev_data::Digest::from_bytes(&digest_vec).unwrap())
}

pub fn get_recursive_digest_for_dir(
    root_path: &Path,
    rel_path_ignore_list: &fnv::FnvHashSet<PathBuf>,
) -> std::result::Result<Vec<u8>, crev_recursive_digest::DigestError> {
    let h = crev_recursive_digest::RecursiveDigest::<crev_common::Blake2b256, _, _>::new()
        .filter(|entry| {
            let rel_path = entry
                .path()
                .strip_prefix(root_path)
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
            PathBuf::from("Cargo.CREV.toml")
        }
        ".cargo" => {
            changes.push(".cargo config can replace linkers, source of dependencies".to_string());
            PathBuf::from("CREV.cargo")
        }
        "config" | "config.toml" if parent.file_name().unwrap() == "cargo" => {
            changes.push("cargo/config can replace linkers, source of dependencies".to_string());
            PathBuf::from("config.CREV")
        }
        "rust-toolchain" | "rust-toolchain.toml" => {
            changes
                .push("rust-toolchain file could unexpectedly replace your compiler".to_string());
            PathBuf::from(format!("{orig_name}.CREV"))
        }
        ".cargo-ok" | ".cargo_vcs_info.json" | ".gitignore" => {
            // they're safe
            PathBuf::from(orig_name)
        }
        n if n.starts_with('.') => {
            changes.push(format!("Hidden file: '{orig_name}'"));
            PathBuf::from(format!("CREV{orig_name}"))
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
            // only obviously non-text files get a pass
            if is_binary_file_extension(&dest_path) {
                std::fs::copy(&src_path, &dest_path)?;
            } else {
                let input = std::fs::read(&src_path)?;
                let output = escape_tricky_unicode(&input);
                if output != input {
                    changes.push(format!(
                        "Escaped potentially confusing UTF-8 in '{}'",
                        src_path.display()
                    ));
                }
                std::fs::write(&dest_path, output)?;
            }
        } else {
            assert!(ft.is_dir());
            let _ = std::fs::create_dir(&dest_path);
            copy_dir_sanitized(&src_path, &dest_path, changes)?;
        }
    }
    Ok(())
}

fn is_binary_file_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map_or(false, |e| {
            matches!(
                e.to_lowercase().as_str(),
                "bin"
                    | "zip"
                    | "gz"
                    | "xz"
                    | "bz2"
                    | "jpg"
                    | "jpeg"
                    | "png"
                    | "gif"
                    | "exe"
                    | "dll"
            )
        })
}

fn escape_tricky_unicode(input: &[u8]) -> Cow<[u8]> {
    if input.is_ascii() {
        return input.into();
    }

    let mut output = Vec::with_capacity(input.len());
    for ch in input.utf8_chunks() {
        output.extend_from_slice(escape_tricky_unicode_str(ch.valid()).as_bytes());
        output.extend_from_slice(ch.invalid());
    }
    output.into()
}

fn escape_tricky_unicode_str(input: &str) -> Cow<str> {
    if input.is_ascii() {
        return input.into();
    }

    use std::fmt::Write;
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            // https://blog.rust-lang.org/2021/11/01/cve-2021-42574.html
            // https://www.unicode.org/L2/L2022/22007r2-avoiding-spoof.pdf
            '\u{115F}' | '\u{1160}' | '\u{13437}' | '\u{13438}' | '\u{1D173}' | '\u{1D174}'
            | '\u{1D175}' | '\u{1D176}' | '\u{1D177}' | '\u{1D178}' | '\u{1D179}' | '\u{1D17A}'
            | '\u{202A}' | '\u{202B}' | '\u{202C}' | '\u{202D}' | '\u{202E}' | '\u{2066}'
            | '\u{2067}' | '\u{2068}' | '\u{2069}' | '\u{206A}' | '\u{206B}' | '\u{206C}'
            | '\u{206D}' | '\u{206E}' | '\u{206F}' | '\u{3164}' | '\u{FFA0}' | '\u{FFF9}'
            | '\u{FFFA}' | '\u{FFFB}' => {
                let _ = write!(&mut out, "\\u{{{:04x}}}", ch as u32);
            }
            _ => out.push(ch),
        }
    }
    out.into()
}

#[test]
fn escapes_unicode_bidi() {
    let bidi_test = "\u{202A}\u{202B}\u{202C}\u{202D}\u{202E} | \u{2066} | \x00\u{2067} | \u{2068}\u{FFFF} | \u{2069}";
    assert_eq!(
        "\\u{202a}\\u{202b}\\u{202c}\\u{202d}\\u{202e} | \\u{2066} | \u{0}\\u{2067} | \\u{2068}\u{ffff} | \\u{2069}".as_bytes(),
        &*escape_tricky_unicode(bidi_test.as_bytes()),
    );

    let binary_test = &b"ABC\0\0\0\x11\xff \xc0\xfa\xda"[..];
    assert_eq!(binary_test, &*escape_tricky_unicode(binary_test));
}
