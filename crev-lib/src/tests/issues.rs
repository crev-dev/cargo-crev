use super::*;

use crev_data::{proof, OwnId};
use semver::Version;
use ifmt::iformat;
use crev_data::review::{Advisory, AdvisoryRange, Issue};
use crev_data::TrustLevel;

const SOURCE: &str = "SOURCE_ID";
const NAME: &str = "name";

fn build_advisory(id: impl Into<String>, range: AdvisoryRange) -> Advisory {
    let id = id.into();
    Advisory::builder()
            .range(range)
            .ids(vec![id.clone()])
            .comment(iformat!("comment for {id}"))
            .build()
}

fn build_issue(id: impl Into<String>) -> Issue {
    let id = id.into();
    Issue::builder()
            .id(id.clone())
            .comment(iformat!("issue {id}"))
            .build()
}
fn build_proof_with_advisories(id: &OwnId, version: Version, advisories: Vec<Advisory>) -> proof::Proof {
    let package_info = proof::PackageInfo {
        id: None,
        source: "SOURCE_ID".to_owned(),
        name: NAME.into(),
        version: version,
        digest: vec![0, 1, 2, 3],
        digest_type: proof::default_digest_type(),
        revision: "".into(),
        revision_type: proof::default_revision_type(),
    };
    let review = proof::review::PackageBuilder::default()
        .from(id.id.to_owned())
        .package(package_info.clone())
        .comment("comment".into())
        .advisories(advisories)
        .build()
        .unwrap();

    let proof = review.sign_by(&id).unwrap();

    proof
}

fn build_proof_with_issues(id: &OwnId, version: Version, issues: Vec<Issue>) -> proof::Proof {
    let package_info = proof::PackageInfo {
        id: None,
        source: "SOURCE_ID".to_owned(),
        name: NAME.into(),
        version: version,
        digest: vec![0, 1, 2, 3],
        digest_type: proof::default_digest_type(),
        revision: "".into(),
        revision_type: proof::default_revision_type(),
    };
    let review = proof::review::PackageBuilder::default()
        .from(id.id.to_owned())
        .package(package_info.clone())
        .comment("comment".into())
        .issues(issues)
        .build()
        .unwrap();

    let proof = review.sign_by(&id).unwrap();

    proof
}
#[test]
fn advisories_sanity() -> Result<()> {
    let id = OwnId::generate_for_git_url("https://a");

/*
    let package_info = proof::PackageInfo {
        id: None,
        source: "SOURCE_ID".to_owned(),
        name: NAME.into(),
        version: Version::parse("1.2.3").unwrap(),
        digest: vec![0, 1, 2, 3],
        digest_type: proof::default_digest_type(),
        revision: "".into(),
        revision_type: proof::default_revision_type(),
    };
    let review = proof::review::PackageBuilder::default()
        .from(id.id.to_owned())
        .package(package_info.clone())
        .comment("comment".into())
        .advisories()
        .build()
        .unwrap();
    let proof = review.sign_by(&id)?;

*/
    let proof = build_proof_with_advisories(&id, Version::parse("1.2.3").unwrap(), vec![build_advisory("someid", AdvisoryRange::Major)]);

    let mut trustdb = ProofDB::new();
    trustdb.import_from_iter(vec![proof].into_iter());

    assert_eq!(
        trustdb
            .get_advisories_for_version(SOURCE, NAME, &Version::parse("1.2.2").unwrap())
            .len(),
        1
    );
    assert_eq!(
        trustdb
            .get_advisories_for_version(SOURCE, NAME, &Version::parse("1.2.3").unwrap())
            .len(),
        0
    );
    assert_eq!(
        trustdb
            .get_advisories_for_version(SOURCE, NAME, &Version::parse("1.3.0").unwrap())
            .len(),
        0
    );
    assert_eq!(
        trustdb
            .get_advisories_for_version(SOURCE, NAME, &Version::parse("0.1.0").unwrap())
            .len(),
        0
    );
/*
    let review = proof::review::PackageBuilder::default()
        .from(id.id.to_owned())
        .package(package_info)
        .comment("comment".into())
        .advisories( vec![ build_advisory("someid", AdvisoryRange::All) ])
        .build()
        .unwrap();

    let proof = review.sign_by(&id)?;
    */
    let proof = build_proof_with_advisories(&id, Version::parse("1.2.3").unwrap(), vec![build_advisory("someid", AdvisoryRange::All)]);


    trustdb.import_from_iter(vec![proof].into_iter());

    assert_eq!(
        trustdb
            .get_advisories_for_version(SOURCE, NAME, &Version::parse("0.1.0").unwrap())
            .len(),
        1
    );
    assert_eq!(
        trustdb
            .get_advisories_for_version(SOURCE, NAME, &Version::parse("1.3.0").unwrap())
            .len(),
        0
    );
    assert_eq!(
        trustdb
            .get_advisories_for_version(SOURCE, NAME, &Version::parse("2.3.0").unwrap())
            .len(),
        0
    );

    Ok(())
}

#[test]
fn issues_sanity() -> Result<()> {
    let id = OwnId::generate_for_git_url("https://a");
    let mut trustdb = ProofDB::new();
    let trust_set = trustdb.calculate_trust_set(id.as_ref(), &TrustDistanceParams::new_no_wot());

    /*
    const SOURCE: &str = "SOURCE_ID";
    const NAME: &str = "NAME";

    let package_info = proof::PackageInfo {
        id: None,
        source: "SOURCE_ID".to_owned(),
        name: NAME.into(),
        version: Version::parse("1.2.3").unwrap(),
        digest: vec![0, 1, 2, 3],
        digest_type: proof::default_digest_type(),
        revision: "".into(),
        revision_type: proof::default_revision_type(),
    };
    let review = proof::review::PackageBuilder::default()
        .from(id.id.to_owned())
        .package(package_info.clone())
        .comment("comment".into())
        .advisories(vec![build_advisory("someid", AdvisoryRange::Major)])
        .build()
        .unwrap();

    let proof = review.sign_by(&id)?;
        */


    let proof = build_proof_with_advisories(&id, Version::parse("1.2.3").unwrap(), vec![build_advisory("issueX", AdvisoryRange::Major)]);
    trustdb.import_from_iter(vec![proof].into_iter());

    assert_eq!(
        trustdb
            .get_issues_for_version(SOURCE, NAME, &Version::parse("2.0.0").unwrap(), &trust_set, TrustLevel::Medium)
            .len(),
        0
    );

    assert_eq!(
        trustdb
            .get_issues_for_version(SOURCE, NAME, &Version::parse("1.0.1").unwrap(), &trust_set, TrustLevel::Medium)
            .len(),
        1
    );

    let proof = build_proof_with_advisories(&id, Version::parse("2.0.1").unwrap(), vec![build_advisory("issueY", AdvisoryRange::All)]);
    trustdb.import_from_iter(vec![proof].into_iter());
    assert_eq!(
        trustdb
            .get_issues_for_version(SOURCE, NAME, &Version::parse("0.0.1").unwrap(), &trust_set, TrustLevel::Medium)
            .len(),
        1
    );
    assert_eq!(
        trustdb
            .get_issues_for_version(SOURCE, NAME, &Version::parse("2.0.0").unwrap(), &trust_set, TrustLevel::Medium)
            .len(),
        1
    );

    assert_eq!(
        trustdb
            .get_issues_for_version(SOURCE, NAME, &Version::parse("2.0.1").unwrap(), &trust_set, TrustLevel::Medium)
            .len(),
        0
    );

    let proof = build_proof_with_issues(&id, Version::parse("3.0.5").unwrap(), vec![build_issue("issueX")]);
    trustdb.import_from_iter(vec![proof].into_iter());

    assert_eq!(
        trustdb
            .get_issues_for_version(SOURCE, NAME, &Version::parse("3.0.4").unwrap(), &trust_set, TrustLevel::Medium)
            .len(),
        0
    );

    assert_eq!(
        trustdb
            .get_issues_for_version(SOURCE, NAME, &Version::parse("3.0.5").unwrap(), &trust_set, TrustLevel::Medium)
            .len(),
        1
    );

    assert_eq!(
        trustdb
            .get_issues_for_version(SOURCE, NAME, &Version::parse("3.1.0").unwrap(), &trust_set, TrustLevel::Medium)
            .len(),
        1
    );

    let proof = build_proof_with_advisories(&id, Version::parse("3.1.0").unwrap(), vec![build_advisory("issueX", AdvisoryRange::Major)]);
    trustdb.import_from_iter(vec![proof].into_iter());

    assert_eq!(
        trustdb
            .get_issues_for_version(SOURCE, NAME, &Version::parse("3.1.0").unwrap(), &trust_set, TrustLevel::Medium)
            .len(),
        0
    );
    assert_eq!(
        trustdb
            .get_issues_for_version(SOURCE, NAME, &Version::parse("4.0.0").unwrap(), &trust_set, TrustLevel::Medium)
            .len(),
        0
    );
    assert_eq!(
        trustdb
            .get_issues_for_version(SOURCE, NAME, &Version::parse("3.0.5").unwrap(), &trust_set, TrustLevel::Medium)
            .len(),
        1
    );

    assert_eq!(
        trustdb
            .get_issues_for_version(SOURCE, NAME, &Version::parse("3.0.7").unwrap(), &trust_set, TrustLevel::Medium)
            .len(),
        1
    );
    Ok(())
}
