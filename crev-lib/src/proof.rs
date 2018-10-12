use chrono::prelude::*;
use crev_data::proof::Content;
use std::path::PathBuf;

fn type_name(content: &Content) -> &str {
    match content {
        Content::Trust(_) => "trust",
        Content::Code(_) => "code-review",
        Content::Project(_) => "project-review",
    }
}

/// The path to use under project `.crev/`
pub(crate) fn rel_project_path(content: &Content) -> PathBuf {
    let type_name = type_name(content);

    PathBuf::from(content.author_id().to_string())
        .join(type_name)
        .join(
            content
                .date()
                .with_timezone(&Utc)
                .format("%Y-%m")
                .to_string(),
        )
        .with_extension(format!("{}.crev", type_name))
}

/// The path to use under user store
pub(crate) fn rel_store_path(content: &Content) -> PathBuf {
    let type_name = type_name(content);
    let mut path = PathBuf::from(content.author_id().to_string()).join(type_name);

    if let Some(project_id) = content.project_id() {
        path = path.join(project_id)
    }

    path.join(
        content
            .date()
            .with_timezone(&Utc)
            .format("%Y-%m")
            .to_string(),
    )
    .with_extension(format!("{}.crev", type_name))
}
