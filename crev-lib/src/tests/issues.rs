use super::*;

use crev_data::{
    proof,
    review::{Advisory, Issue, VersionRange},
    TrustLevel, UnlockedId,
};
use crev_wot::FetchSource;
use semver::Version;

const SOURCE: &str = "SOURCE_ID";
const NAME: &str = "name";

fn build_advisory(id: impl Into<String>, range: VersionRange) -> Advisory {
    let id = id.into();
    Advisory::builder()
        .range(range)
        .ids(vec![id.clone()])
        .comment(format!("comment for {}", id))
        .build()
}

fn build_issue(id: impl Into<String>) -> Issue {
    let id = id.into();
    Issue::builder()
        .id(id.clone())
        .comment(format!("issue {}", id))
        .build()
}

fn build_proof_with_advisories(
    id: &UnlockedId,
    version: Version,
    advisories: Vec<Advisory>,
) -> proof::Proof {
    let package_info = proof::PackageInfo {
        id: proof::PackageVersionId::new(SOURCE.into(), NAME.into(), version),
        digest: vec![0, 1, 2, 3],
        digest_type: proof::default_digest_type(),
        revision: "".into(),
        revision_type: proof::default_revision_type(),
    };
    let review = proof::review::PackageBuilder::default()
        .from(id.id.to_owned())
        .package(package_info)
        .comment("comment".into())
        .advisories(advisories)
        .build()
        .unwrap();

    review.sign_by(&id).unwrap()
}

fn build_proof_with_issues(id: &UnlockedId, version: Version, issues: Vec<Issue>) -> proof::Proof {
    let package_info = proof::PackageInfo {
        id: proof::PackageVersionId::new("SOURCE_ID".to_owned(), NAME.into(), version),
        digest: vec![0, 1, 2, 3],
        digest_type: proof::default_digest_type(),
        revision: "".into(),
        revision_type: proof::default_revision_type(),
    };
    let review = proof::review::PackageBuilder::default()
        .from(id.id.to_owned())
        .package(package_info)
        .comment("comment".into())
        .issues(issues)
        .build()
        .unwrap();

    review.sign_by(&id).unwrap()
}

#[test]
fn advisories_sanity() -> Result<()> {
    let url = FetchSource::LocalUser;
    let id = UnlockedId::generate_for_git_url("https://a");

    let proof = build_proof_with_advisories(
        &id,
        Version::parse("1.2.3").unwrap(),
        vec![build_advisory("someid", VersionRange::Major)],
    );

    let mut proofdb = ProofDB::new();
    proofdb.import_from_iter(vec![(proof, url.clone())].into_iter());

    assert_eq!(proofdb.get_pkg_reviews_for_source(SOURCE).count(), 1);

    assert_eq!(
        proofdb
            .get_advisories_for_version(SOURCE, NAME, &Version::parse("1.2.2").unwrap())
            .count(),
        1
    );
    assert_eq!(
        proofdb
            .get_advisories_for_version(SOURCE, NAME, &Version::parse("1.2.3").unwrap())
            .count(),
        0
    );
    assert_eq!(
        proofdb
            .get_advisories_for_version(SOURCE, NAME, &Version::parse("1.3.0").unwrap())
            .count(),
        0
    );
    assert_eq!(
        proofdb
            .get_advisories_for_version(SOURCE, NAME, &Version::parse("0.1.0").unwrap())
            .count(),
        0
    );
    let proof = build_proof_with_advisories(
        &id,
        Version::parse("1.2.3").unwrap(),
        vec![build_advisory("someid", VersionRange::All)],
    );

    proofdb.import_from_iter(vec![(proof, url)].into_iter());

    assert_eq!(
        proofdb
            .get_advisories_for_version(SOURCE, NAME, &Version::parse("0.1.0").unwrap())
            .count(),
        1
    );
    assert_eq!(
        proofdb
            .get_advisories_for_version(SOURCE, NAME, &Version::parse("1.3.0").unwrap())
            .count(),
        0
    );
    assert_eq!(
        proofdb
            .get_advisories_for_version(SOURCE, NAME, &Version::parse("2.3.0").unwrap())
            .count(),
        0
    );

    Ok(())
}

#[test]
fn issues_sanity() -> Result<()> {
    let url = FetchSource::LocalUser;
    let id = UnlockedId::generate_for_git_url("https://a");
    let mut trustdb = ProofDB::new();
    let trust_set = trustdb.calculate_trust_set(id.as_ref(), &TrustDistanceParams::new_no_wot());

    let proof = build_proof_with_advisories(
        &id,
        Version::parse("1.2.3").unwrap(),
        vec![build_advisory("issueX", VersionRange::Major)],
    );
    trustdb.import_from_iter(vec![(proof, url.clone())].into_iter());

    assert_eq!(
        trustdb
            .get_open_issues_for_version(
                SOURCE,
                NAME,
                &Version::parse("2.0.0").unwrap(),
                &trust_set,
                TrustLevel::Medium
            )
            .len(),
        0
    );

    assert_eq!(
        trustdb
            .get_open_issues_for_version(
                SOURCE,
                NAME,
                &Version::parse("1.0.1").unwrap(),
                &trust_set,
                TrustLevel::Medium
            )
            .len(),
        1
    );

    let proof = build_proof_with_advisories(
        &id,
        Version::parse("2.0.1").unwrap(),
        vec![build_advisory("issueY", VersionRange::All)],
    );
    trustdb.import_from_iter(vec![(proof, url.clone())].into_iter());
    assert_eq!(
        trustdb
            .get_open_issues_for_version(
                SOURCE,
                NAME,
                &Version::parse("0.0.1").unwrap(),
                &trust_set,
                TrustLevel::Medium
            )
            .len(),
        1
    );
    assert_eq!(
        trustdb
            .get_open_issues_for_version(
                SOURCE,
                NAME,
                &Version::parse("2.0.0").unwrap(),
                &trust_set,
                TrustLevel::Medium
            )
            .len(),
        1
    );

    assert_eq!(
        trustdb
            .get_open_issues_for_version(
                SOURCE,
                NAME,
                &Version::parse("2.0.1").unwrap(),
                &trust_set,
                TrustLevel::Medium
            )
            .len(),
        0
    );

    let proof = build_proof_with_issues(
        &id,
        Version::parse("3.0.5").unwrap(),
        vec![build_issue("issueX")],
    );
    trustdb.import_from_iter(vec![(proof, url.clone())].into_iter());

    assert_eq!(
        trustdb
            .get_open_issues_for_version(
                SOURCE,
                NAME,
                &Version::parse("3.0.4").unwrap(),
                &trust_set,
                TrustLevel::Medium
            )
            .len(),
        0
    );

    assert_eq!(
        trustdb
            .get_open_issues_for_version(
                SOURCE,
                NAME,
                &Version::parse("3.0.5").unwrap(),
                &trust_set,
                TrustLevel::Medium
            )
            .len(),
        1
    );

    assert_eq!(
        trustdb
            .get_open_issues_for_version(
                SOURCE,
                NAME,
                &Version::parse("3.1.0").unwrap(),
                &trust_set,
                TrustLevel::Medium
            )
            .len(),
        1
    );

    let proof = build_proof_with_advisories(
        &id,
        Version::parse("3.1.0").unwrap(),
        vec![build_advisory("issueX", VersionRange::Major)],
    );
    trustdb.import_from_iter(vec![(proof, url)].into_iter());

    assert_eq!(
        trustdb
            .get_open_issues_for_version(
                SOURCE,
                NAME,
                &Version::parse("3.1.0").unwrap(),
                &trust_set,
                TrustLevel::Medium
            )
            .len(),
        0
    );
    assert_eq!(
        trustdb
            .get_open_issues_for_version(
                SOURCE,
                NAME,
                &Version::parse("4.0.0").unwrap(),
                &trust_set,
                TrustLevel::Medium
            )
            .len(),
        0
    );
    assert_eq!(
        trustdb
            .get_open_issues_for_version(
                SOURCE,
                NAME,
                &Version::parse("3.0.5").unwrap(),
                &trust_set,
                TrustLevel::Medium
            )
            .len(),
        1
    );

    assert_eq!(
        trustdb
            .get_open_issues_for_version(
                SOURCE,
                NAME,
                &Version::parse("3.0.7").unwrap(),
                &trust_set,
                TrustLevel::Medium
            )
            .len(),
        1
    );
    Ok(())
}
