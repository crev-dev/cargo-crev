use super::*;
use crate::proofdb::FetchSource;
use crev_data::{
    proof::{self, trust::TrustLevel, ContentExt},
    Digest, Level, OwnId, Url,
};
use default::default;
use semver::Version;
use std::{str::FromStr, sync::Arc};

mod issues;

// Basic liftime of an `LockedId`:
//
// * generate
// * lock with a passphrase
// * unlock
// * compare
#[test]
fn lock_and_unlock() -> Result<()> {
    let id = OwnId::generate_for_git_url("https://example.com/crev-proofs");

    let id_relocked = id::LockedId::from_own_id(&id, "password")?.to_unlocked("password")?;
    assert_eq!(id.id.id, id_relocked.id.id);

    assert!(id::LockedId::from_own_id(&id, "password")?
        .to_unlocked("wrongpassword")
        .is_err());

    let id_stored = serde_yaml::to_string(&id::LockedId::from_own_id(&id, "pass")?)?;
    let id_restored: OwnId =
        serde_yaml::from_str::<id::LockedId>(&id_stored)?.to_unlocked("pass")?;

    println!("{}", id_stored);

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

    let _trust_proof =
        unlocked.create_signed_trust_proof(vec![unlocked.as_pubid()], TrustLevel::High)?;

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

// Exact distance of flooding the web of trust graph is configurable,
// with the edges distance corresponding to the trust level.
#[test]
fn proofdb_distance() -> Result<()> {
    let url = FetchSource::Url(Arc::new(Url::new_git("https://example.com")));

    let a = OwnId::generate_for_git_url("https://a");
    let b = OwnId::generate_for_git_url("https://b");
    let c = OwnId::generate_for_git_url("https://c");
    let d = OwnId::generate_for_git_url("https://d");
    let e = OwnId::generate_for_git_url("https://e");

    let distance_params = TrustDistanceParams {
        high_trust_distance: 1,
        medium_trust_distance: 10,
        low_trust_distance: 100,
        max_distance: 111,
    };

    let a_to_b = a.create_signed_trust_proof(vec![b.as_pubid()], TrustLevel::High)?;
    let b_to_c = b.create_signed_trust_proof(vec![c.as_pubid()], TrustLevel::Medium)?;
    let c_to_d = c.create_signed_trust_proof(vec![d.as_pubid()], TrustLevel::Low)?;
    let d_to_e = d.create_signed_trust_proof(vec![e.as_pubid()], TrustLevel::High)?;

    let mut trustdb = ProofDB::new();

    trustdb.import_from_iter(
        vec![a_to_b, b_to_c, c_to_d, d_to_e]
            .into_iter()
            .map(|x| (x, url.clone())),
    );

    let trust_set: HashSet<crev_data::Id> = trustdb
        .calculate_trust_set(a.as_ref(), &distance_params)
        .trusted_ids()
        .cloned()
        .collect();

    assert!(trust_set.contains(a.as_ref()));
    assert!(trust_set.contains(b.as_ref()));
    assert!(trust_set.contains(c.as_ref()));
    assert!(trust_set.contains(d.as_ref()));
    assert!(!trust_set.contains(e.as_ref()));

    let b_to_d = b.create_signed_trust_proof(vec![d.as_pubid()], TrustLevel::Medium)?;

    trustdb.import_from_iter(vec![(b_to_d, url.clone())].into_iter());

    let trust_set: HashSet<_> = trustdb
        .calculate_trust_set(a.as_ref(), &distance_params)
        .trusted_ids()
        .cloned()
        .collect();

    assert!(trust_set.contains(a.as_ref()));
    assert!(trust_set.contains(b.as_ref()));
    assert!(trust_set.contains(c.as_ref()));
    assert!(trust_set.contains(d.as_ref()));
    assert!(trust_set.contains(e.as_ref()));
    Ok(())
}

// A subsequent review of exactly same package version
// is supposed to overwrite the previous one, and it
// should be visible in all the user-facing stats, listings
// and counts.
#[test]
fn overwritting_reviews() -> Result<()> {
    let url = FetchSource::Url(Arc::new(Url::new_git("https://a")));
    let a = OwnId::generate_for_git_url("https://a");
    let digest = vec![0; 32];
    let package = crev_data::proof::PackageInfo {
        id: proof::PackageVersionId::new(
            "source".into(),
            "name".into(),
            Version::parse("1.0.0").unwrap(),
        ),
        digest: digest.clone(),
        digest_type: crev_data::proof::default_digest_type(),
        revision: "".into(),
        revision_type: crev_data::proof::default_revision_type(),
    };

    let proof1 = a
        .as_pubid()
        .create_package_review_proof(package.clone(), default(), "a".into())?
        .sign_by(&a)?;
    // it's lame, but oh well... ; we need to make sure there's a time delay between
    // the two proofs
    #[allow(deprecated)]
    std::thread::sleep_ms(1);
    let proof2 = a
        .as_pubid()
        .create_package_review_proof(package.clone(), default(), "b".into())?
        .sign_by(&a)?;

    for order in vec![
        vec![proof1.clone(), proof2.clone()],
        vec![proof2.clone(), proof1.clone()],
    ] {
        let mut trustdb = ProofDB::new();
        trustdb.import_from_iter(order.into_iter().map(|x| (x, url.clone())));
        assert_eq!(
            trustdb
                .get_package_reviews_by_digest(&Digest::from_vec(digest.clone()))
                .map(|r| r.comment)
                .collect::<Vec<_>>(),
            vec!["b".to_string()]
        );
        assert_eq!(
            trustdb
                .get_package_reviews_for_package(
                    &package.id.id.source,
                    Some(&package.id.id.name),
                    Some(&package.id.version)
                )
                .count(),
            1
        );
        assert_eq!(
            trustdb
                .get_package_reviews_for_package(
                    &package.id.id.source,
                    Some(&package.id.id.name),
                    None
                )
                .count(),
            1
        );
        assert_eq!(
            trustdb
                .get_package_reviews_for_package(&package.id.id.source, None, None)
                .count(),
            1
        );
    }

    Ok(())
}

#[test]
fn dont_consider_an_empty_review_as_valid() -> Result<()> {
    let url = FetchSource::Url(Arc::new(Url::new_git("https://a")));
    let a = OwnId::generate_for_git_url("https://a");
    let digest = vec![0; 32];
    let package = crev_data::proof::PackageInfo {
        id: proof::PackageVersionId::new(
            "source".into(),
            "name".into(),
            Version::parse("1.0.0").unwrap(),
        ),
        digest: digest.clone(),
        digest_type: crev_data::proof::default_digest_type(),
        revision: "".into(),
        revision_type: crev_data::proof::default_revision_type(),
    };

    let review = crev_data::proof::review::Review::new_none();

    let proof1 = a
        .as_pubid()
        .create_package_review_proof(package.clone(), review, "a".into())?
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
        &Digest::from_vec(digest),
        &trust_set,
        &verification_reqs,
        &trustdb
    )
    .is_verified());

    Ok(())
}

#[test]
fn proofdb_distrust() -> Result<()> {
    let url = FetchSource::Url(Arc::new(Url::new_git("https://a")));
    let a = OwnId::generate_for_git_url("https://a");
    let b = OwnId::generate_for_git_url("https://b");
    let c = OwnId::generate_for_git_url("https://c");
    let d = OwnId::generate_for_git_url("https://d");
    let e = OwnId::generate_for_git_url("https://e");

    let distance_params = TrustDistanceParams {
        high_trust_distance: 1,
        medium_trust_distance: 10,
        low_trust_distance: 100,
        max_distance: 10000,
    };

    let a_to_bc =
        a.create_signed_trust_proof(vec![b.as_pubid(), c.as_pubid()], TrustLevel::High)?;
    let b_to_d = b.create_signed_trust_proof(vec![d.as_pubid()], TrustLevel::Low)?;
    let d_to_c = d.create_signed_trust_proof(vec![c.as_pubid()], TrustLevel::Distrust)?;
    let c_to_e = c.create_signed_trust_proof(vec![e.as_pubid()], TrustLevel::High)?;

    let mut trustdb = ProofDB::new();

    trustdb.import_from_iter(
        vec![a_to_bc, b_to_d, d_to_c, c_to_e]
            .into_iter()
            .map(|x| (x, url.clone())),
    );

    let trust_set: HashSet<_> = trustdb
        .calculate_trust_set(a.as_ref(), &distance_params)
        .trusted_ids()
        .cloned()
        .collect();

    assert!(trust_set.contains(a.as_ref()));
    assert!(trust_set.contains(b.as_ref()));
    assert!(!trust_set.contains(c.as_ref()));
    assert!(trust_set.contains(d.as_ref()));
    assert!(!trust_set.contains(e.as_ref()));

    let e_to_d = e.create_signed_trust_proof(vec![d.as_pubid()], TrustLevel::Distrust)?;

    trustdb.import_from_iter(vec![(e_to_d, url.clone())].into_iter());

    let trust_set: HashSet<_> = trustdb
        .calculate_trust_set(a.as_ref(), &distance_params)
        .trusted_ids()
        .cloned()
        .collect();

    assert!(trust_set.contains(a.as_ref()));
    assert!(trust_set.contains(b.as_ref()));
    assert!(!trust_set.contains(c.as_ref()));
    assert!(!trust_set.contains(d.as_ref()));
    assert!(!trust_set.contains(e.as_ref()));

    Ok(())
}
