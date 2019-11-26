use crev_data::proof::{self, CommonOps};
use std::path::PathBuf;

fn proof_store_names(proof: &proof::Proof) -> (&str, Option<&str>) {
    match proof.kind() {
        proof::CodeReview::KIND => ("reviews", Some("code")),
        proof::PackageReview::KIND => ("reviews", Some("package")),
        proof::Trust::KIND => ("trust", None),
        _ => ("other", None),
    }
}

/// The path to use under user store
pub(crate) fn rel_store_path(proof: &proof::Proof, host_salt: &[u8]) -> PathBuf {
    let (type_name, type_subname) = proof_store_names(proof);
    let date = proof.date_utc().format("%Y-%m").to_string();
    let path = PathBuf::from(proof.author_id().to_string()).join(type_name);
    let mut host_full_id = host_salt.to_vec();
    host_full_id.append(&mut proof.author_id().to_bytes());
    let host_plus_id_digest = crev_common::blake2b256sum(&host_full_id);

    path.join(if let Some(type_subname) = type_subname {
        format!(
            "{}-{}-{}",
            date,
            type_subname,
            // this used to be `[..4]`, but temporarily change it
            // to accomodate a new proof format. old clients will
            // fail to parse a whole file if it contains a new proof
            // format, so this makes sure new proofs are only
            // in separate files; this can be reverted back after
            // some time
            crev_common::base64_encode(&host_plus_id_digest[1..5])
        )
    } else {
        format!(
            "{}-{}",
            date,
            crev_common::base64_encode(&host_plus_id_digest[1..5])
        )
    })
    .with_extension("proof.crev")
}
