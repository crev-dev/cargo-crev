use digest::Digest;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::io::Write;
use tempdir::TempDir;

#[test]
fn sanity() -> io::Result<()> {
    let tmp_dir = TempDir::new("recursive-digest-test")?;

    let msg = b"foo";
    let dir_path = tmp_dir.path().join("a");
    let file_path = tmp_dir.path().join("b");

    fs::create_dir_all(&dir_path)?;

    let file_in_dir_path = dir_path.join("foo");
    let mut file_in_dir = fs::File::create(&file_in_dir_path)?;
    file_in_dir.write_all(msg)?;
    drop(file_in_dir);

    let mut file = fs::File::create(&file_path)?;

    file.write_all(msg)?;
    drop(file);

    let empty = HashSet::new();

    let dir_digest = crev_recursive_digest::get_recursive_digest_for_dir::<blake2::Blake2b, _>(
        &dir_path,
        &empty.clone(),
    )?;
    let file_digest = crev_recursive_digest::get_recursive_digest_for_dir::<blake2::Blake2b, _>(
        &file_path, &empty,
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

/// Captured by:
///
/// ```
/// mkdir -p /tmp/a/b/c/d/e/f/g
/// ln -sf /tmp/a/b/c/d/e/f/g/h "../../a"
/// rblake2sum /tmp/a
/// ```
#[test]
fn backward_comp() -> io::Result<()> {
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

    std::os::unix::fs::symlink(std::path::PathBuf::from("../../a"), path.join("h"))?;

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
