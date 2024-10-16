use super::*;
use crev_data::{
    proof::{ContentExt, PackageVersionId},
    Level, UnlockedId, Url,
};
use crev_wot::{FetchSource, ProofDB};
use default::default;
use std::{str::FromStr, sync::Arc};

// Basic lifetime of an `LockedId`:
//
// * generate
// * lock with a passphrase
// * unlock
// * compare
#[test]
fn lock_and_unlock() -> Result<()> {
    let id = UnlockedId::generate_for_git_url("https://example.com/crev-proofs");

    let id_relocked = id::LockedId::from_unlocked_id(&id, "password")?.to_unlocked("password")?;
    assert_eq!(id.id.id, id_relocked.id.id);

    assert!(id::LockedId::from_unlocked_id(&id, "password")?
        .to_unlocked("wrongpassword")
        .is_err());

    let id_stored = serde_yaml::to_string(&id::LockedId::from_unlocked_id(&id, "pass")?)?;
    let id_restored: UnlockedId =
        serde_yaml::from_str::<id::LockedId>(&id_stored)?.to_unlocked("pass")?;

    println!("{id_stored}");

    assert_eq!(id.id.id, id_restored.id.id);
    Ok(())
}

#[test]
fn use_id_generated_by_previous_versions() -> Result<()> {
    let yaml = r#"
---
version: -1
url: "https://github.com/dpc/crev-proofs-test"
public-key: mScrJLNL5NV4DH9mSPsqcvU8wu0P_W6bvXhjViZP4aE
sealed-secret-key: ukQvCTnTX6LmnUaBkoB4IGhIvnMxSNb5T8HoEn6DbFnI1IWzMqsGhkzxVzzc-zDs
seal-nonce: gUu4izYVvDgZjHFGpcunWmNV3nTgmswvSZsCr3lKboQ
pass:
  version: 19
  variant: argon2id
  iterations: 192
  memory-size: 4096
  lanes: 8
  salt: 9jeCQhM2dMZErCErRQ_RmZ08X68xpta1tIhTbCHOTs0
"#;

    let locked = id::LockedId::from_str(yaml)?;
    let unlocked = locked.to_unlocked("a")?;

    let _trust_proof = unlocked.create_signed_trust_proof(
        vec![unlocked.as_public_id()],
        TrustLevel::High,
        vec![],
    )?;

    Ok(())
}

#[test]
fn validate_proof_generated_by_previous_version() -> Result<()> {
    let yaml = r#"
-----BEGIN CREV PACKAGE REVIEW-----
version: -1
date: "2019-04-13T00:04:16.625524407-07:00"
from:
  id-type: crev
  id: mScrJLNL5NV4DH9mSPsqcvU8wu0P_W6bvXhjViZP4aE
  url: "https://github.com/dpc/crev-proofs-test"
package:
  source: "https://crates.io"
  name: hex
  version: 0.3.2
  digest: 6FtxZesHD7pnSlbpp--CF_MPAnJATZI4ZR-Vdwb6Fes
review:
  thoroughness: none
  understanding: medium
  rating: positive
comment: THIS IS JUST FOR TEST
-----BEGIN CREV PACKAGE REVIEW SIGNATURE-----
NtGu3z1Jtnj6wx8INBrVujcOPz61BiGmJS-UoAOe0XQutatFsEbgAcAo7rBvZz4Q-ccNXIFZtKnXhBDMjVm0Aw
-----END CREV PACKAGE REVIEW-----
"#;

    let proofs = crev_data::proof::Proof::parse_from(yaml.as_bytes())?;
    assert_eq!(proofs.len(), 1);

    proofs[0].verify()?;

    Ok(())
}

#[test]
fn dont_consider_an_empty_review_as_valid() -> Result<()> {
    let url = FetchSource::Url(Arc::new(Url::new_git("https://a")));
    let a = UnlockedId::generate_for_git_url("https://a");
    let digest = [13; 32];
    let package = crev_data::proof::PackageInfo {
        id: PackageVersionId::new(
            "source".into(),
            "name".into(),
            Version::parse("1.0.0").unwrap(),
        ),
        revision: String::new(),
        revision_type: crev_data::proof::default_revision_type(),
        digest: digest.to_vec(),
        digest_type: crev_data::proof::default_digest_type(),
    };

    let review = crev_data::proof::review::Review::new_none();

    let proof1 = a
        .as_public_id()
        .create_package_review_proof(package, review, vec![], "a".into())?
        .sign_by(&a)?;

    let mut trustdb = ProofDB::new();
    let trust_set = trustdb.calculate_trust_set(&a.id.id, &default());
    trustdb.import_from_iter(vec![proof1].into_iter().map(|x| (x, url.clone())));
    let verification_reqs = VerificationRequirements {
        thoroughness: Level::None,
        understanding: Level::None,
        trust_level: Level::None,
        redundancy: 1,
    };
    assert!(!verify_package_digest(
        &Digest::from(digest),
        &trust_set,
        &verification_reqs,
        &trustdb
    )
    .is_verified());

    Ok(())
}
