use super::*;
use crev_data::{
    proof::{self, trust::TrustLevel, ContentExt, OverrideItem},
    Digest, UnlockedId, Url, Version,
};
use default::default;
use std::sync::Arc;

mod issues;

// Exact distance of flooding the web of trust graph is configurable,
// with the edges distance corresponding to the trust level.
#[test]
fn proofdb_distance() -> Result<()> {
    let url = FetchSource::Url(Arc::new(Url::new_git("https://example.com")));

    let a = UnlockedId::generate_for_git_url("https://a");
    let b = UnlockedId::generate_for_git_url("https://b");
    let c = UnlockedId::generate_for_git_url("https://c");
    let d = UnlockedId::generate_for_git_url("https://d");
    let e = UnlockedId::generate_for_git_url("https://e");

    let distance_params = TrustDistanceParams {
        high_trust_distance: 1,
        medium_trust_distance: 10,
        low_trust_distance: 100,
        max_distance: 111,
    };

    let a_to_b = a.create_signed_trust_proof(vec![b.as_public_id()], TrustLevel::High, vec![])?;
    let b_to_c = b.create_signed_trust_proof(vec![c.as_public_id()], TrustLevel::Medium, vec![])?;
    let c_to_d = c.create_signed_trust_proof(vec![d.as_public_id()], TrustLevel::Low, vec![])?;
    let d_to_e = d.create_signed_trust_proof(vec![e.as_public_id()], TrustLevel::High, vec![])?;

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

    let b_to_d = b.create_signed_trust_proof(vec![d.as_public_id()], TrustLevel::Medium, vec![])?;

    trustdb.import_from_iter(vec![(b_to_d, url)].into_iter());

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
    let a = UnlockedId::generate_for_git_url("https://a");
    let digest = [0; 32];
    let package = crev_data::proof::PackageInfo {
        id: proof::PackageVersionId::new(
            "source".into(),
            "name".into(),
            Version::parse("1.0.0").unwrap(),
        ),
        digest: digest.to_vec(),
        digest_type: crev_data::proof::default_digest_type(),
        revision: "".into(),
        revision_type: crev_data::proof::default_revision_type(),
    };

    let proof1 = a
        .as_public_id()
        .create_package_review_proof(package.clone(), default(), vec![], "a".into())?
        .sign_by(&a)?;
    // it's lame, but oh well... ; we need to make sure there's a time delay between
    // the two proofs
    #[allow(deprecated)]
    std::thread::sleep_ms(1);
    let proof2 = a
        .as_public_id()
        .create_package_review_proof(package.clone(), default(), vec![], "b".into())?
        .sign_by(&a)?;

    for order in vec![vec![proof1.clone(), proof2.clone()], vec![proof2, proof1]] {
        let mut trustdb = ProofDB::new();
        trustdb.import_from_iter(order.into_iter().map(|x| (x, url.clone())));
        assert_eq!(
            trustdb
                .get_package_reviews_by_digest(&Digest::from(digest))
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
fn proofdb_distrust() -> Result<()> {
    let url = FetchSource::Url(Arc::new(Url::new_git("https://a")));
    let a = UnlockedId::generate_for_git_url("https://a");
    let b = UnlockedId::generate_for_git_url("https://b");
    let c = UnlockedId::generate_for_git_url("https://c");
    let d = UnlockedId::generate_for_git_url("https://d");
    let e = UnlockedId::generate_for_git_url("https://e");

    let distance_params = TrustDistanceParams {
        high_trust_distance: 1,
        medium_trust_distance: 10,
        low_trust_distance: 100,
        max_distance: 10000,
    };

    let a_to_bc = a.create_signed_trust_proof(
        vec![b.as_public_id(), c.as_public_id()],
        TrustLevel::High,
        vec![],
    )?;
    let b_to_d = b.create_signed_trust_proof(vec![d.as_public_id()], TrustLevel::Low, vec![])?;
    let d_to_c =
        d.create_signed_trust_proof(vec![c.as_public_id()], TrustLevel::Distrust, vec![])?;
    let c_to_e = c.create_signed_trust_proof(vec![e.as_public_id()], TrustLevel::Low, vec![])?;

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

    // This introduces a tie between nodes banning each other.
    // Both should be removed from the trust_set.
    let e_to_d =
        e.create_signed_trust_proof(vec![d.as_public_id()], TrustLevel::Distrust, vec![])?;

    trustdb.import_from_iter(vec![(e_to_d, url)].into_iter());

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

#[test]
fn proofdb_trust_ignore_override() -> Result<()> {
    let url = FetchSource::Url(Arc::new(Url::new_git("https://a")));
    let a = UnlockedId::generate_for_git_url("https://a");
    let b = UnlockedId::generate_for_git_url("https://b");
    let c = UnlockedId::generate_for_git_url("https://c");
    let d = UnlockedId::generate_for_git_url("https://d");

    let distance_params = TrustDistanceParams {
        high_trust_distance: 1,
        medium_trust_distance: 10,
        low_trust_distance: 100,
        max_distance: 10000,
    };

    // a trust b and c, but c more, c overrides (ignores) trust of b in d
    let a_to_b = a.create_signed_trust_proof(vec![b.as_public_id()], TrustLevel::Medium, vec![])?;
    let a_to_c = a.create_signed_trust_proof(vec![c.as_public_id()], TrustLevel::High, vec![])?;

    let b_to_d = b.create_signed_trust_proof(vec![d.as_public_id()], TrustLevel::High, vec![])?;

    let c_to_d = {
        let mut c_to_d_unsigned =
            c.id.create_trust_proof(vec![d.as_public_id()], TrustLevel::None, vec![])?;
        c_to_d_unsigned.override_.push(OverrideItem {
            id: b.as_public_id().clone(),
            comment: "".into(),
        });
        c_to_d_unsigned.sign_by(&c)?
    };

    let mut trustdb = ProofDB::new();

    trustdb.import_from_iter(
        vec![a_to_b, a_to_c.clone(), b_to_d.clone(), c_to_d.clone()]
            .into_iter()
            .map(|x| (x, url.clone())),
    );

    let trust_set: TrustSet = trustdb.calculate_trust_set(a.as_ref(), &distance_params);
    let trusted_ids: HashSet<_> = trust_set.trusted_ids().cloned().collect();

    assert!(trust_set
        .trust_ignore_overrides
        .contains_key(&(b.id.id.clone(), d.id.id.clone())));
    assert!(trusted_ids.contains(a.as_ref()));
    assert!(trusted_ids.contains(b.as_ref()));
    assert!(trusted_ids.contains(c.as_ref()));
    assert!(!trusted_ids.contains(d.as_ref()));

    // had the `a` trusted `b` at the same level as `c`, the override to ignore `b`'s trust in `d`
    // would not have effect from PoV of `a`
    {
        let a_to_b =
            a.create_signed_trust_proof(vec![b.as_public_id()], TrustLevel::High, vec![])?;

        let mut trustdb = ProofDB::new();

        trustdb.import_from_iter(
            vec![a_to_b, a_to_c, b_to_d, c_to_d]
                .into_iter()
                .map(|x| (x, url.clone())),
        );

        let trust_set: TrustSet = trustdb.calculate_trust_set(a.as_ref(), &distance_params);
        let trusted_ids: HashSet<_> = trust_set.trusted_ids().cloned().collect();

        assert!(trust_set
            .trust_ignore_overrides
            .contains_key(&(b.id.id.clone(), d.id.id.clone())));
        assert!(trusted_ids.contains(a.as_ref()));
        assert!(trusted_ids.contains(b.as_ref()));
        assert!(trusted_ids.contains(c.as_ref()));
        assert!(trusted_ids.contains(d.as_ref()));
    }
    Ok(())
}
