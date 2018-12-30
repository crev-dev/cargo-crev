use super::*;

use crate::trustdb::{self, ProofDB};
use crev_data::proof::trust::TrustLevel;
use crev_data::OwnId;

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
fn trustdb_distance() -> Result<()> {
    let a = OwnId::generate_for_git_url("https://a");
    let b = OwnId::generate_for_git_url("https://b");
    let c = OwnId::generate_for_git_url("https://c");
    let d = OwnId::generate_for_git_url("https://d");
    let e = OwnId::generate_for_git_url("https://e");

    let distance_params = trustdb::TrustDistanceParams {
        high_trust_distance: 1,
        medium_trust_distance: 10,
        low_trust_distance: 100,
        max_distance: 111,
    };

    let a_to_b = a
        .create_trust_proof(vec![b.as_pubid().to_owned()], TrustLevel::High)?
        .sign_by(&a)?;
    let b_to_c = b
        .create_trust_proof(vec![c.as_pubid().to_owned()], TrustLevel::Medium)?
        .sign_by(&b)?;
    let c_to_d = c
        .create_trust_proof(vec![d.as_pubid().to_owned()], TrustLevel::Low)?
        .sign_by(&c)?;
    let d_to_e = d
        .create_trust_proof(vec![e.as_pubid().to_owned()], TrustLevel::High)?
        .sign_by(&d)?;

    let mut trustdb = ProofDB::new();

    trustdb.import_from_iter(vec![a_to_b, b_to_c, c_to_d, d_to_e].into_iter());

    let trust_set = trustdb.calculate_trust_set(a.as_ref(), &distance_params);

    assert!(trust_set.contains(a.as_ref()));
    assert!(trust_set.contains(b.as_ref()));
    assert!(trust_set.contains(c.as_ref()));
    assert!(trust_set.contains(d.as_ref()));
    assert!(!trust_set.contains(e.as_ref()));

    let b_to_d = b
        .create_trust_proof(vec![d.as_pubid().to_owned()], TrustLevel::Medium)?
        .sign_by(&b)?;

    trustdb.import_from_iter(vec![b_to_d].into_iter());
    let trust_set = trustdb.calculate_trust_set(a.as_ref(), &distance_params);

    assert!(trust_set.contains(a.as_ref()));
    assert!(trust_set.contains(b.as_ref()));
    assert!(trust_set.contains(c.as_ref()));
    assert!(trust_set.contains(d.as_ref()));
    assert!(trust_set.contains(e.as_ref()));
    Ok(())
}
