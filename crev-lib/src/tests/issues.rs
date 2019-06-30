use super::*;

use crev_data::{
    proof::{self},
    OwnId,
};
use semver::Version;

#[test]
fn advisory_sanity() -> Result<()> {
    let id = OwnId::generate_for_git_url("https://a");
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
        .advisories(vec![proof::review::package::AdvisoryBuilder::default()
            .range(proof::review::package::AdvisoryRange::Major)
            .critical(false)
            .ids(vec!["someid".into()])
            .comment("comment".into())
            .build()
            .unwrap()])
        .build()
        .unwrap();

    let proof = review.sign_by(&id)?;

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

    let review = proof::review::PackageBuilder::default()
        .from(id.id.to_owned())
        .package(package_info)
        .comment("comment".into())
        .advisories(vec![proof::review::package::AdvisoryBuilder::default()
            .range(proof::review::package::AdvisoryRange::All)
            .ids(vec!["someid".into()])
            .critical(false)
            .build()
            .unwrap()])
        .build()
        .unwrap();

    let proof = review.sign_by(&id)?;

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
