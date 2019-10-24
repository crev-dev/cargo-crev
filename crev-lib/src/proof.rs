use chrono::prelude::*;
use crev_data::proof::Content;
use std::path::PathBuf;

/// The path to use under package `.crev/`
pub(crate) fn rel_package_path(content: &Content, host_salt: &[u8]) -> PathBuf {
    rel_store_path(content, host_salt)
}

/// The path to use under user store
pub(crate) fn rel_store_path(content: &Content, host_salt: &[u8]) -> PathBuf {
    let (type_name, type_subname) = type_name(content);
    let date = content
        .date()
        .with_timezone(&Utc)
        .format("%Y-%m")
        .to_string();
    let path = PathBuf::from(content.author_id().to_string()).join(type_name);
    let mut host_full_id = host_salt.to_vec();
    host_full_id.append(&mut content.author_id().to_bytes());
    let host_plus_id_digest = crev_common::blake2b256sum(&host_full_id);

    path.join(if let Some(type_subname) = type_subname {
        format!(
            "{}-{}-{}",
            date,
            type_subname,
            crev_common::base64_encode(&host_plus_id_digest[..4])
        )
    } else {
        format!(
            "{}-{}",
            date,
            crev_common::base64_encode(&host_plus_id_digest[..4])
        )
    })
    .with_extension("proof.crev")
}
