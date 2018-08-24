use super::*;
use common_failures::prelude::*;

use std::path::PathBuf;

#[test]
fn sign_proof_review() -> Result<()> {
    let id = id::OwnId::generate("John Doe <doe@john.com>".into());

    let unsigned_review = proof::ReviewProofBuilder::default()
        .from("Me <me@me.com>".into())
        .from_id("abcdf".into())
        .from_id_type("crev".into())
        .revision(Some("foobar".into()))
        .revision_type("git".into())
        .project_urls(vec!["https://github.com/someone/somelib".into()])
        .comment(Some("comment".into()))
        .thoroughness(proof::Level::Some)
        .understanding(proof::Level::Some)
        .trust(proof::Level::Some)
        .files(vec![
            proof::ReviewProofFile {
                path: PathBuf::from("foo.x"),
                digest: vec![1, 2, 3, 4],
                digest_type: "sha256".into(),
            },
            proof::ReviewProofFile {
                path: PathBuf::from("foo.x"),
                digest: vec![1, 2, 3, 4],
                digest_type: "sha256".into(),
            },
        ]).build()
        .map_err(|e| format_err!("{}", e))?;

    println!("{}", unsigned_review);
    let signed_review = unsigned_review.sign(&id)?;
    println!("{}", signed_review);

    let parsed_unsigned = signed_review.parse_review()?;
    println!("{}", parsed_unsigned);

    Ok(())
}
