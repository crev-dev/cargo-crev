use crate::{
    id::UnlockedId,
    proof::{self, Content, ContentExt, ContentWithDraft, Proof},
    Error, Result, Url,
};
use semver::Version;
use std::{default::Default, path::PathBuf};

#[test]
pub fn signed_parse() -> Result<()> {
    let s = r#"
-----BEGIN CREV PACKAGE REVIEW-----
version: -1
date: "2018-12-18T23:10:21.111854021-08:00"
from:
  id-type: crev
  id: FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE
  url: "https://github.com/dpc/crev-proofs"
package:
  source: "https://crates.io"
  name: log
  version: 0.4.6
  digest: BhDmOOjfESqs8i3z9qsQANH8A39eKklgQKuVtrwN-Tw
review:
  thoroughness: low
  understanding: medium
  rating: positive
-----BEGIN CREV PACKAGE REVIEW SIGNATURE-----
4R2WjtU-avpBznmJYAl44H1lOYgETu3RSNhCDcB4GpqhJbSRkd-eqnUuhHgDUs77OlhUf7BSA0dydxaALwx0Dg
-----END CREV PACKAGE REVIEW-----
"#;

    let proofs = Proof::parse_from(s.as_bytes())?;
    assert_eq!(proofs.len(), 1);
    assert_eq!(
        proofs[0].signature(),
        "4R2WjtU-avpBznmJYAl44H1lOYgETu3RSNhCDcB4GpqhJbSRkd-eqnUuhHgDUs77OlhUf7BSA0dydxaALwx0Dg"
    );
    Ok(())
}

#[test]
pub fn signed_parse_multiple() -> Result<()> {
    let s = r#"

-----BEGIN CREV PACKAGE REVIEW-----
version: -1
date: "2018-12-18T23:10:21.111854021-08:00"
from:
  id-type: crev
  id: FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE
  url: "https://github.com/dpc/crev-proofs"
package:
  source: "https://crates.io"
  name: log
  version: 0.4.6
  digest: BhDmOOjfESqs8i3z9qsQANH8A39eKklgQKuVtrwN-Tw
review:
  thoroughness: low
  understanding: medium
  rating: positive
-----BEGIN CREV PACKAGE REVIEW SIGNATURE-----
4R2WjtU-avpBznmJYAl44H1lOYgETu3RSNhCDcB4GpqhJbSRkd-eqnUuhHgDUs77OlhUf7BSA0dydxaALwx0Dg
-----END CREV PACKAGE REVIEW-----
-----BEGIN CREV PACKAGE REVIEW-----
version: -1
date: "2018-12-18T23:10:21.111854021-08:00"
from:
  id-type: crev
  id: FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE
  url: "https://github.com/dpc/crev-proofs"
package:
  source: "https://crates.io"
  name: log
  version: 0.4.6
  digest: BhDmOOjfESqs8i3z9qsQANH8A39eKklgQKuVtrwN-Tw
review:
  thoroughness: low
  understanding: medium
  rating: positive
-----BEGIN CREV PACKAGE REVIEW SIGNATURE-----
4R2WjtU-avpBznmJYAl44H1lOYgETu3RSNhCDcB4GpqhJbSRkd-eqnUuhHgDUs77OlhUf7BSA0dydxaALwx0Dg
-----END CREV PACKAGE REVIEW-----
"#;

    let proofs = Proof::parse_from(s.as_bytes())?;
    assert_eq!(proofs.len(), 2);
    assert_eq!(
        proofs[0].signature(),
        "4R2WjtU-avpBznmJYAl44H1lOYgETu3RSNhCDcB4GpqhJbSRkd-eqnUuhHgDUs77OlhUf7BSA0dydxaALwx0Dg"
    );
    assert_eq!(
        proofs[1].signature(),
        "4R2WjtU-avpBznmJYAl44H1lOYgETu3RSNhCDcB4GpqhJbSRkd-eqnUuhHgDUs77OlhUf7BSA0dydxaALwx0Dg"
    );
    Ok(())
}

#[test]
pub fn signed_parse_multiple_newlines() -> Result<()> {
    let s = r#"

-----BEGIN CREV PACKAGE REVIEW-----
version: -1
date: "2018-12-18T23:10:21.111854021-08:00"
from:
  id-type: crev
  id: FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE
  url: "https://github.com/dpc/crev-proofs"
package:
  source: "https://crates.io"
  name: log
  version: 0.4.6
  digest: BhDmOOjfESqs8i3z9qsQANH8A39eKklgQKuVtrwN-Tw
review:
  thoroughness: low
  understanding: medium
  rating: positive
-----BEGIN CREV PACKAGE REVIEW SIGNATURE-----
4R2WjtU-avpBznmJYAl44H1lOYgETu3RSNhCDcB4GpqhJbSRkd-eqnUuhHgDUs77OlhUf7BSA0dydxaALwx0Dg
-----END CREV PACKAGE REVIEW-----



-----BEGIN CREV PACKAGE REVIEW-----
version: -1
date: "2018-12-18T23:10:21.111854021-08:00"
from:
  id-type: crev
  id: FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE
  url: "https://github.com/dpc/crev-proofs"
package:
  source: "https://crates.io"
  name: log
  version: 0.4.6
  digest: BhDmOOjfESqs8i3z9qsQANH8A39eKklgQKuVtrwN-Tw
review:
  thoroughness: low
  understanding: medium
  rating: positive
-----BEGIN CREV PACKAGE REVIEW SIGNATURE-----
4R2WjtU-avpBznmJYAl44H1lOYgETu3RSNhCDcB4GpqhJbSRkd-eqnUuhHgDUs77OlhUf7BSA0dydxaALwx0Dg
-----END CREV PACKAGE REVIEW-----

"#;

    let proofs = Proof::parse_from(s.as_bytes())?;
    assert_eq!(proofs.len(), 2);
    assert_eq!(
        proofs[0].signature(),
        "4R2WjtU-avpBznmJYAl44H1lOYgETu3RSNhCDcB4GpqhJbSRkd-eqnUuhHgDUs77OlhUf7BSA0dydxaALwx0Dg"
    );
    assert_eq!(
        proofs[1].signature(),
        "4R2WjtU-avpBznmJYAl44H1lOYgETu3RSNhCDcB4GpqhJbSRkd-eqnUuhHgDUs77OlhUf7BSA0dydxaALwx0Dg"
    );
    Ok(())
}

pub fn generate_id_and_proof() -> Result<(UnlockedId, Proof)> {
    let id = UnlockedId::generate(Some(Url::new_git("https://mypage.com/trust.git")));

    let package_info = proof::PackageInfo {
        id: proof::PackageVersionId::new(
            "SOURCE_ID".to_owned(),
            "name".into(),
            Version::parse("1.0.0").unwrap(),
        ),
        digest: vec![0, 1, 2, 3],
        digest_type: proof::default_digest_type(),
        revision: String::new(),
        revision_type: proof::default_revision_type(),
    };
    let review = proof::review::CodeBuilder::default()
        .from(id.id.clone())
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
        .map_err(|e| Error::BuildingReview(e.to_string().into()))?;

    let proof = review.sign_by(&id)?;

    Ok((id, proof))
}

#[test]
pub fn sign_proof_review() -> Result<()> {
    let (_id, proof) = generate_id_and_proof()?;

    proof.verify()?;
    println!("{proof}");

    Ok(())
}

#[test]
pub fn verify_works() -> Result<()> {
    let (_id, proof) = generate_id_and_proof()?;

    let proof = Proof::from_parts(proof.body().to_owned() + "\n", proof.signature().to_owned())?;

    assert!(proof.verify().is_err());

    Ok(())
}

#[test]
pub fn ensure_serializes_to_valid_proof_works() -> Result<()> {
    let a = UnlockedId::generate_for_git_url("https://a");
    let digest = vec![0; 32];
    let package = proof::PackageInfo {
        id: proof::PackageVersionId::new(
            "source".into(),
            "name".into(),
            Version::parse("1.0.0").unwrap(),
        ),
        digest,
        digest_type: proof::default_digest_type(),
        revision: String::new(),
        revision_type: proof::default_revision_type(),
    };

    let mut package = a.as_public_id().create_package_review_proof(
        package,
        Default::default(),
        vec![],
        "a".into(),
    )?;
    assert!(package.ensure_serializes_to_valid_proof().is_ok());
    package.comment = "a".repeat(32_000);
    assert!(package.ensure_serializes_to_valid_proof().is_err());
    Ok(())
}

#[test]
pub fn parse_package_overrides() -> Result<()> {
    let s = r#"
version: -1
date: "2018-12-18T23:10:21.111854021-08:00"
from:
  id-type: crev
  id: FYlr8YoYGVvDwHQxqEIs89reKKDy-oWisoO0qXXEfHE
  url: "https://github.com/dpc/crev-proofs"
package:
  source: "https://crates.io"
  name: log
  version: 0.4.6
  digest: BhDmOOjfESqs8i3z9qsQANH8A39eKklgQKuVtrwN-Tw
review:
  thoroughness: low
  understanding: medium
  rating: positive
override:
  - id-type: crev
    id: "-sApEowWcAS9J0R7aO18cghvhLBpuMhyeUuWQq_fits"
    url: "https://github.com/foo/bar"
    comment: TEST
"#;

    let proof: proof::package::Package = serde_yaml::from_str(s).expect("deserialization failed");

    proof.validate_data()?;

    let draft = proof.to_draft();

    assert_eq!(proof.override_.len(), 1);
    assert!(draft.body.contains("override:"));
    assert!(draft
        .body
        .contains("-sApEowWcAS9J0R7aO18cghvhLBpuMhyeUuWQq_fits"));

    let new_proof = proof.apply_draft(&draft.body)?;

    assert_eq!(proof.override_.len(), new_proof.override_.len());

    Ok(())
}
