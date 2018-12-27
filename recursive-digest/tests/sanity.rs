use crev_recursive_digest::DigestError;
use digest::Digest;
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempdir::TempDir;

#[test]
fn sanity() -> Result<(), DigestError> {
    let tmp_dir = TempDir::new("recursive-digest-test")?;

    let msg = b"foo";

    // Directory "recursive-digest-test/a/"
    let dir_path = tmp_dir.path().join("a");
    fs::create_dir_all(&dir_path)?;

    // File "recursive-digest-test/a/foo"
    let file_in_dir_path = dir_path.join("foo");
    let mut file_in_dir = fs::File::create(&file_in_dir_path)?;
    file_in_dir.write_all(msg)?;
    drop(file_in_dir);

    // File "recursive-digest-test/b"
    let file_path = tmp_dir.path().join("b");
    let mut file = fs::File::create(&file_path)?;
    file.write_all(msg)?;
    drop(file);

    let empty = HashSet::new();

    let dir_digest = crev_recursive_digest::get_recursive_digest_for_dir::<blake2::Blake2b, _>(
        &dir_path, // "recursive-digest-test/a/"
        &empty,    // Exclude no files
    )?;
    let file_digest = crev_recursive_digest::get_recursive_digest_for_dir::<blake2::Blake2b, _>(
        &file_path, // "recursive-digest-test/b"
        &empty,     // Exclude no files
    )?;

    let mut hasher = blake2::Blake2b::new();
    hasher.input(b"F");
    hasher.input(msg);

    let standalone_file_digest = hasher.result().to_vec();

    assert_eq!(&file_digest, &standalone_file_digest);
    assert_ne!(&dir_digest, &standalone_file_digest);
    // captured by `echo  -ne "Ffoo" | b2sum`
    assert_eq!(
        hex::encode(&standalone_file_digest),
        "e41c3b6ac2b512af3a14eb11faed1486f693ce3bd3606afbe458e183ae4e1080a4209f44ada1c186920f541d41a192eaa654fee6792a6ac008f44f783a59176d"
    );

    let mut hasher = blake2::Blake2b::new();
    hasher.input(b"D");
    hasher.input(
        &hex::decode(
            "ca002330e69d3e6b84a46a56a6533fd79d51d97a3bb7cad6c2ff43b354185d6dc1e723fb3db4ae0737e120378424c714bb982d9dc5bbd7a0ab318240ddd18f8d"
        ).unwrap()
    );

    hasher.input(&file_digest);

    let manual_dir_digest = hasher.result().to_vec();
    assert_eq!(&dir_digest, &manual_dir_digest);

    Ok(())
}

#[cfg(target_family = "windows")]
pub fn symlink_file<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(src, dst)
}

#[cfg(target_family = "unix")]
pub fn symlink_file<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

/// Captured by:
///
/// ```
/// mkdir -p /tmp/a/b/c/d/e/f/g
/// ln -sf /tmp/a/b/c/d/e/f/g/h "../../a"
/// rblake2sum /tmp/a
/// ```
#[test]
fn backward_comp() -> Result<(), DigestError> {
    let tmp_dir = TempDir::new("recursive-digest-test2")?;

    let dir_path = tmp_dir.path().join("a");
    let path = dir_path.clone();
    let path = path.join("b");
    let path = path.join("c");
    let path = path.join("d");
    let path = path.join("e");
    let path = path.join("f");
    let path = path.join("g");
    fs::create_dir_all(&path)?;

    symlink_file(std::path::PathBuf::from("../../a"), path.join("h"))?;

    let dir_digest = crev_recursive_digest::get_recursive_digest_for_dir::<blake2::Blake2b, _>(
        &dir_path,
        &HashSet::new(),
    )?;

    assert_eq!(
        hex::encode(&dir_digest),
        "bc97399633e1228a563d57adecf98810364526a8e7bfc24b89985c5607e77605575d10989d5954b762af45c498129854dca603688fd63bd580bbf952c650b735"
    );
    tmp_dir.into_path();
    Ok(())
}

#[test]
fn test_file_digest() -> Result<(), DigestError> {
    let tmp_dir = TempDir::new("recursive-digest-test3")?;
    let foo_content = b"foo_content";
    let file_in_dir_path = tmp_dir.path().join("foo");
    let mut file_in_dir = fs::File::create(&file_in_dir_path)?;
    file_in_dir.write_all(foo_content)?;

    let empty = HashSet::new();

    let expected = {
        let mut hasher = blake2::Blake2b::new();
        hasher.input(b"F");
        hasher.input(foo_content);
        hasher.result().to_vec()
    };

    assert_eq!(
        crev_recursive_digest::get_recursive_digest_for_dir::<blake2::Blake2b, _>(
            &file_in_dir_path,
            &empty
        )?,
        expected
    );

    Ok(())
}

#[test]
// Tests the inclusion and exclusing of paths.
fn test_exclude_include_path() -> Result<(), DigestError> {
    let tmp_dir = TempDir::new("recursive-digest-test3")?;

    let foo_content = b"foo_content";
    let file_in_dir_path = tmp_dir.path().join("foo");
    let mut file_in_dir = fs::File::create(&file_in_dir_path)?;
    file_in_dir.write_all(foo_content)?;

    let bar_content = b"bar_content";
    let file_in_dir_path_2 = tmp_dir.path().join("bar");
    let mut file_in_dir_2 = fs::File::create(&file_in_dir_path_2)?;
    file_in_dir_2.write_all(bar_content)?;

    let expected = {
        let mut hasher = blake2::Blake2b::new();
        hasher.input(b"F");
        hasher.input(bar_content);
        let file_sum = hasher.result().to_vec();

        let mut hasher = blake2::Blake2b::new();
        hasher.input("bar".as_bytes());
        let dir_sum = hasher.result().to_vec();

        let mut hasher = blake2::Blake2b::new();
        hasher.input(b"D");
        hasher.input(dir_sum);
        hasher.input(file_sum);
        hasher.result().to_vec()
    };

    let mut excluded = HashSet::new();
    excluded.insert(Path::new("foo").to_path_buf());
    assert_eq!(
        crev_recursive_digest::get_recursive_digest_for_dir::<blake2::Blake2b, _>(
            &tmp_dir.path(),
            &excluded
        )?,
        expected
    );

    let mut included = HashSet::new();
    included.insert(Path::new("bar").to_path_buf());
    assert_eq!(
        crev_recursive_digest::get_recursive_digest_for_paths::<blake2::Blake2b, _>(
            &tmp_dir.path(),
            included
        )?,
        expected
    );

    Ok(())
}

#[test]
fn ignore_dir() -> Result<(), DigestError> {
    let tmp_dir = TempDir::new("recursive-digest-test-ignore-dir")?;

    let d1 = tmp_dir.path().join("d1");
    let d2 = tmp_dir.path().join("d2");

    fs::create_dir_all(&d1.join("a/b1/c/d"))?;
    fs::create_dir_all(&d1.join("a/b2/c/d"))?;
    fs::create_dir_all(&d2)?;

    let excluded_empty = HashSet::new();
    let mut excluded_a = HashSet::new();
    excluded_a.insert(PathBuf::from("a"));

    assert_eq!(
        crev_recursive_digest::get_recursive_digest_for_dir::<blake2::Blake2b, _>(
            &d1,
            &excluded_a
        )?,
        crev_recursive_digest::get_recursive_digest_for_dir::<blake2::Blake2b, _>(
            &d2,
            &excluded_empty
        )?,
    );
    Ok(())
}
