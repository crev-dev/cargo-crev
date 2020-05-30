pub use crev_common::{run_with_shell_cmd, store_str_to_file, store_to_file_with};
use crev_data::proof;
use std::{
    self, io,
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
