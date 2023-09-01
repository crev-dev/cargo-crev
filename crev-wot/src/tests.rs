use super::*;
use crev_data::{
    proof::{self, trust::TrustLevel, ContentExt, OverrideItem},
    Digest, UnlockedId, Url, Version,
};
use default::default;
use std::sync::Arc;

mod issues;

fn trust_proof(from: &UnlockedId, to: &UnlockedId, level: TrustLevel) -> Result<proof::Proof> {
    Ok(from.create_signed_trust_proof(vec![to.as_public_id()], level, vec![])?)
}

fn trust_high(from: &UnlockedId, to: &UnlockedId) -> Result<proof::Proof> {
    trust_proof(from, to, TrustLevel::High)
}

fn trust_medium(from: &UnlockedId, to: &UnlockedId) -> Result<proof::Proof> {
    trust_proof(from, to, TrustLevel::Medium)
}

fn trust_low(from: &UnlockedId, to: &UnlockedId) -> Result<proof::Proof> {
    trust_proof(from, to, TrustLevel::Low)
}

fn trust_distrust(from: &UnlockedId, to: &UnlockedId) -> Result<proof::Proof> {
    trust_proof(from, to, TrustLevel::Distrust)
}

// https://stackoverflow.com/a/27582993
macro_rules! collection {
    // map-like
    ($($k:expr => $v:expr),* $(,)?) => {{
        use std::iter::{Iterator, IntoIterator};
        Iterator::collect(IntoIterator::into_iter([$(($k, $v),)*]))
    }};
    // set-like
    ($($v:expr),* $(,)?) => {{
        use std::iter::{Iterator, IntoIterator};
        Iterator::collect(IntoIterator::into_iter([$($v,)*]))
    }};
}

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
        none_trust_distance: 112,
        distrust_distance: 112,
        max_distance: 111,
    };
    let mut trustdb = ProofDB::new();

    trustdb.import_from_iter(
        vec![
            trust_high(&a, &b)?,
            trust_medium(&b, &c)?,
            trust_low(&c, &d)?,
            trust_high(&d, &e)?,
        ]
        .into_iter()
        .map(|x| (x, url.clone())),
    );

    let trust_set = trustdb.calculate_trust_set(a.as_ref(), &distance_params);

    assert_eq!(
        trust_set.get_trusted_ids_refs(),
        collection![a.as_ref(), b.as_ref(), c.as_ref(), d.as_ref()]
    );

    trustdb.import_from_iter(vec![(trust_medium(&b, &d)?, url)].into_iter());

    let trust_set = trustdb.calculate_trust_set(a.as_ref(), &distance_params);

    assert_eq!(
        trust_set.get_trusted_ids_refs(),
        collection![a.as_ref(), b.as_ref(), c.as_ref(), d.as_ref(), e.as_ref()]
    );

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
        revision: String::new(),
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

    for order in [vec![proof1.clone(), proof2.clone()], vec![proof2, proof1]] {
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
        none_trust_distance: 10001,
        distrust_distance: 10001,
        max_distance: 10000,
    };
    let mut trustdb = ProofDB::new();

    trustdb.import_from_iter(
        vec![
            trust_high(&a, &b)?,
            trust_high(&a, &c)?,
            trust_low(&b, &d)?,
            trust_distrust(&d, &c)?,
            trust_low(&c, &e)?,
        ]
        .into_iter()
        .map(|x| (x, url.clone())),
    );

    let trust_set = trustdb.calculate_trust_set(a.as_ref(), &distance_params);

    assert_eq!(
        trust_set.get_trusted_ids_refs(),
        collection![a.as_ref(), b.as_ref(), d.as_ref()]
    );

    // This introduces a tie between nodes banning each other.
    // Both should be removed from the trust_set.
    trustdb.import_from_iter(vec![(trust_distrust(&e, &d)?, url)].into_iter());

    let trust_set = trustdb.calculate_trust_set(a.as_ref(), &distance_params);

    assert_eq!(
        trust_set.get_trusted_ids_refs(),
        collection![a.as_ref(), b.as_ref()]
    );
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
        none_trust_distance: 10001,
        distrust_distance: 10001,
        max_distance: 10000,
    };

    let mut trustdb = ProofDB::new();

    trustdb.import_from_iter(
        vec![
            // a trust b and c, but c more, c overrides (ignores) trust of b in d
            trust_medium(&a, &b)?,
            trust_high(&a, &c)?,
            trust_high(&b, &d)?,
            {
                let mut c_to_d_unsigned =
                    c.id.create_trust_proof(vec![d.as_public_id()], TrustLevel::None, vec![])?;
                c_to_d_unsigned.override_.push(OverrideItem {
                    id: b.as_public_id().clone(),
                    comment: String::new(),
                });
                c_to_d_unsigned.sign_by(&c)?
            },
        ]
        .into_iter()
        .map(|x| (x, url.clone())),
    );

    let trust_set: TrustSet = trustdb.calculate_trust_set(a.as_ref(), &distance_params);

    assert!(trust_set
        .trust_ignore_overrides
        .contains_key(&(b.id.id.clone(), d.id.id.clone())));

    assert_eq!(
        trust_set.get_trusted_ids_refs(),
        collection![a.as_ref(), b.as_ref(), c.as_ref()]
    );

    // had the `a` trusted `b` at the same level as `c`, the override to ignore `b`'s trust in `d`
    // would not have effect from PoV of `a`
    {
        let mut trustdb = ProofDB::new();

        trustdb.import_from_iter(
            vec![
                // a trust b and c, but c more, c overrides (ignores) trust of b in d
                trust_high(&a, &b)?,
                trust_high(&a, &c)?,
                trust_high(&b, &d)?,
                {
                    let mut c_to_d_unsigned =
                        c.id.create_trust_proof(vec![d.as_public_id()], TrustLevel::None, vec![])?;
                    c_to_d_unsigned.override_.push(OverrideItem {
                        id: b.as_public_id().clone(),
                        comment: String::new(),
                    });
                    c_to_d_unsigned.sign_by(&c)?
                },
            ]
            .into_iter()
            .map(|x| (x, url.clone())),
        );

        let trust_set: TrustSet = trustdb.calculate_trust_set(a.as_ref(), &distance_params);

        assert!(trust_set
            .trust_ignore_overrides
            .contains_key(&(b.id.id.clone(), d.id.id.clone())));
        assert_eq!(
            trust_set.get_trusted_ids_refs(),
            collection![a.as_ref(), b.as_ref(), c.as_ref(), d.as_ref()]
        );
    }
    Ok(())
}
