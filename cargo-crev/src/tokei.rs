use crate::prelude::*;
use std::path::Path;
use tokei::{Config, LanguageType, Languages};

pub fn get_rust_line_count(path: &Path) -> Result<usize> {
    let excluded = &[];
    let config = Config::default();
    let mut languages = Languages::new();
    languages.get_statistics(&[path], excluded, &config);
    let rust = languages
        .get(&LanguageType::Rust)
        .ok_or_else(|| format_err!("Rust should work"))?;
    Ok(rust.code)
}
