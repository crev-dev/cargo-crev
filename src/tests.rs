use super::*;
use common_failures::prelude::*;

use std::path::PathBuf;

#[test]
fn sign_proof_review() -> Result<()> {
    let id = id::OwnId::generate("John Doe <doe@john.com>".into());

    let review = proof::ReviewBuilder::default()
        .from("Me <me@me.com>".into())
        .from_id("abcdf".into())
        .from_id_type("crev".into())
        .revision(Some("foobar".into()))
        .revision_type("git".into())
        .project_urls(vec!["https://github.com/someone/somelib".into()])
        .comment(Some("comment".into()))
        .thoroughness(proof::Level::Low)
        .understanding(proof::Level::Low)
        .trust(proof::Level::Low)
        .files(vec![
            proof::ReviewFile {
                path: PathBuf::from("foo.x"),
                digest: vec![1, 2, 3, 4],
                digest_type: "sha256".into(),
            },
            proof::ReviewFile {
                path: PathBuf::from("foo.x"),
                digest: vec![1, 2, 3, 4],
                digest_type: "sha256".into(),
            },
        ]).build()
        .map_err(|e| format_err!("{}", e))?;

    println!("{}", review);
    let proof = review.sign(&id)?;
    println!("{}", proof);

    let parsed_review = proof.parse_review()?;
    println!("{}", parsed_review);

    Ok(())
}
