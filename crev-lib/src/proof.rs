use chrono::prelude::*;
use crev_data::proof::Content;
use std::path::PathBuf;

fn type_name(content: &Content) -> (&str, Option<&str>) {
    match content {
        Content::Trust(_) => ("trust", None),
        Content::Code(_) => ("reviews", Some("code")),
        Content::Project(_) => ("reviews", Some("projects")),
    }
}

/// The path to use under project `.crev/`
pub(crate) fn rel_project_path(content: &Content) -> PathBuf {
    rel_store_path(content)
}

/// The path to use under user store
pub(crate) fn rel_store_path(content: &Content) -> PathBuf {
    let (type_name, type_subname) = type_name(content);
    let date = content
        .date()
        .with_timezone(&Utc)
        .format("%Y-%m")
        .to_string();
    let path = PathBuf::from(content.author_id().to_string()).join(type_name);

    path.join(if let Some(type_subname) = type_subname {
        format!("{}-{}", date, type_subname)
    } else {
        date
    })
    .with_extension("proof.crev")
}
