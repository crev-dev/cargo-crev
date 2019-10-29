use crate::prelude::*;
use std::path::Path;
use tokei::{Config, LanguageType, Languages};

pub fn get_rust_line_count(path: &Path) -> Result<usize> {
    let excluded = &["tests/", "examples/"];
    let mut config = Config::default();
    config.treat_doc_strings_as_comments = Some(true);
    config.no_ignore_vcs = Some(true);
    config.hidden = Some(true);
    let mut languages = Languages::new();
    languages.get_statistics(&[path], excluded, &config);
    let rust = languages
        .get(&LanguageType::Rust)
        .ok_or_else(|| format_err!("Rust should work"))?;
    Ok(rust.code)
}
