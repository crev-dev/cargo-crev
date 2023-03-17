use std::{
    fs,
    path::{Path, PathBuf},
};

/// Move dir content from `from` dir to `to` dir
pub fn move_dir_content(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;

    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        let path = path
            .strip_prefix(from)
            .expect("Strip prefix should have worked");
        fs::rename(from.join(path), to.join(path))?;
    }

    Ok(())
}

#[must_use]
pub fn append_to_path(path: PathBuf, ext: &str) -> PathBuf {
    let mut path = path.into_os_string();
    path.push(ext);
    path.into()
}
