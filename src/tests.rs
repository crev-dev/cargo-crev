use super::*;
use common_failures::prelude::*;

#[test]
fn sign_proof_review() -> Result<()> {
    let id = id::OwnId::generate("John Doe <doe@john.com>".into());

    let unsigned_review = proof::ReviewProofBuilder::default()
        .revision(Some("foobar".into()))
        .build()
        .map_err(|e| format_err!("{}", e))?;

    let signed_review = unsigned_review.sign(&id);

    println!("{:#?}", signed_review);

    Ok(())
}
