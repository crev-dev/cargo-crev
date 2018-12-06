use crate::{
    id::OwnId,
    proof::{self, Proof, Serialized},
    Result,
};
use std::path::PathBuf;

#[test]
pub fn signed_parse() -> Result<()> {
    let s = r#"
-----BEGIN CODE REVIEW-----
foo
-----BEGIN CODE REVIEW SIGNATURE-----
sig
-----END CODE REVIEW-----
"#;

    let proofs = Serialized::parse(s.as_bytes())?;
    assert_eq!(proofs.len(), 1);
    assert_eq!(proofs[0].body, "foo\n");
    assert_eq!(proofs[0].signature, "sig\n");
    Ok(())
}

#[test]
pub fn signed_parse_multiple() -> Result<()> {
    let s = r#"
-----BEGIN CODE REVIEW-----
foo1
-----BEGIN CODE REVIEW SIGNATURE-----
sig1
-----END CODE REVIEW-----
-----BEGIN CODE REVIEW-----
foo2
-----BEGIN CODE REVIEW SIGNATURE-----
sig2
-----END CODE REVIEW-----
"#;

    let proofs = Serialized::parse(s.as_bytes())?;
    assert_eq!(proofs.len(), 2);
    assert_eq!(proofs[0].body, "foo1\n");
    assert_eq!(proofs[0].signature, "sig1\n");
    assert_eq!(proofs[1].body, "foo2\n");
    assert_eq!(proofs[1].signature, "sig2\n");
    Ok(())
}

#[test]
pub fn signed_parse_multiple_newlines() -> Result<()> {
    let s = r#"

-----BEGIN CODE REVIEW-----
foo1
-----BEGIN CODE REVIEW SIGNATURE-----
sig1
-----END CODE REVIEW-----


-----BEGIN CODE REVIEW-----
foo2
-----BEGIN CODE REVIEW SIGNATURE-----
sig2
-----END CODE REVIEW-----"#;

    let proofs = Serialized::parse(s.as_bytes())?;
    assert_eq!(proofs.len(), 2);
    assert_eq!(proofs[0].body, "foo1\n");
    assert_eq!(proofs[0].signature, "sig1\n");
    assert_eq!(proofs[1].body, "foo2\n");
    assert_eq!(proofs[1].signature, "sig2\n");
    Ok(())
}

pub fn generate_id_and_proof() -> Result<(OwnId, Proof)> {
    let id = OwnId::generate("https://mypage.com/trust.git".into());

    //let mut from = crate::PubId::new(&id.id, "https://github.com/someone/crev-trust".into());

    let project_id = proof::Project::generate();
    let review = proof::review::CodeBuilder::default()
        .from(id.id.to_owned())
        .revision("foobar".into())
        .revision_type("git".into())
        .project(project_id)
        .comment("comment".into())
        .files(vec![
            proof::review::code::File {
                path: PathBuf::from("foo.x"),
                digest: vec![1, 2, 3, 4],
                digest_type: "sha256".into(),
            },
            proof::review::code::File {
                path: PathBuf::from("foo.x"),
                digest: vec![1, 2, 3, 4],
                digest_type: "sha256".into(),
            },
        ])
        .build()
        .map_err(|e| format_err!("{}", e))?;

    let proof = review.sign_by(&id)?;

    Ok((id, proof))
}

#[test]
pub fn sign_proof_review() -> Result<()> {
    let (_id, proof) = generate_id_and_proof()?;

    proof.verify()?;
    println!("{}", proof);

    Ok(())
}

#[test]
pub fn verify_works() -> Result<()> {
    let (_id, mut proof) = generate_id_and_proof()?;

    proof.body += "\n";

    assert!(proof.verify().is_err());

    Ok(())
}
