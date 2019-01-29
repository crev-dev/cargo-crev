use crate::prelude::*;
use std::path::Path;
use tokei::{LanguageType, Languages};

pub fn get_rust_line_count(path: &Path) -> Result<usize> {
    let path = format!("{}", path.display());
    let paths = &[path.as_str()];
    let excluded = vec![];
    let mut languages = Languages::new();
    languages.get_statistics(paths, excluded, None);
    let language_map = languages.remove_empty();
    let rust = language_map
        .get(&LanguageType::Rust)
        .ok_or_else(|| format_err!("Rust should work"))?;
    Ok(rust.code)
}
