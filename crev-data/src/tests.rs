use crate::{
    id::OwnId,
    proof::{self, ContentExt, Proof},
    Result, Url,
};
use failure::format_err;
use semver::Version;
use std::{default::Default, path::PathBuf};

#[test]
pub fn signed_parse() -> Result<()> {
    let s = r#"
-----BEGIN CODE REVIEW-----
foo
-----BEGIN CODE REVIEW SIGNATURE-----
sig
-----END CODE REVIEW-----
"#;

    let proofs = Proof::parse(s.as_bytes())?;
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

    let proofs = Proof::parse(s.as_bytes())?;
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

    let proofs = Proof::parse(s.as_bytes())?;
    assert_eq!(proofs.len(), 2);
    assert_eq!(proofs[0].body, "foo1\n");
    assert_eq!(proofs[0].signature, "sig1\n");
    assert_eq!(proofs[1].body, "foo2\n");
    assert_eq!(proofs[1].signature, "sig2\n");
    Ok(())
}

pub fn generate_id_and_proof() -> Result<(OwnId, Proof)> {
    let id = OwnId::generate(Url::new_git("https://mypage.com/trust.git".into()));

    let package_info = proof::PackageInfo {
        id: None,
        source: "SOURCE_ID".to_owned(),
        name: "name".into(),
        version: Version::parse("1.0.0").unwrap(),
        digest: vec![0, 1, 2, 3],
        digest_type: proof::default_digest_type(),
        revision: "".into(),
        revision_type: proof::default_revision_type(),
    };
    let review = proof::review::CodeBuilder::default()
        .from(id.id.to_owned())
        .package(package_info)
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

#[test]
pub fn ensure_serializes_to_valid_proof_works() -> Result<()> {
    let a = OwnId::generate_for_git_url("https://a");
    let digest = vec![0; 32];
    let package = proof::PackageInfo {
        id: None,
        source: "source".into(),
        name: "name".into(),
        version: Version::parse("1.0.0").unwrap(),
        digest: digest.clone(),
        digest_type: proof::default_digest_type(),
        revision: "".into(),
        revision_type: proof::default_revision_type(),
    };

    let mut package = a.as_pubid().create_package_review_proof(
        package.clone(),
        Default::default(),
        "a".into(),
    )?;
    assert!(package.ensure_serializes_to_valid_proof().is_ok());
    package.comment = std::iter::repeat("a").take(32_000).collect::<String>();
    assert!(package.ensure_serializes_to_valid_proof().is_err());
    Ok(())
}
