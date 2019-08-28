use crev_data::Rating;
use crev_lib::{self, local::Local};
use failure::format_err;
use std::default::Default;

use crate::opts;
use crate::prelude::*;
use crev_data::proof;
use crev_lib::TrustOrDistrust;

use crate::repo::*;
use crate::shared::*;

/// Review a crate
///
/// * `unrelated` - the crate might not actually be a dependency
#[allow(clippy::option_option)]
pub fn create_review_proof(
    name: &str,
    version: Option<&Version>,
    unrelated: UnrelatedOrDependency,
    report_severity: Option<crev_data::Level>,
    advise_common: Option<opts::AdviseCommon>,
    trust: TrustOrDistrust,
    proof_create_opt: &opts::CommonProofCreate,
    diff_version: &Option<Option<Version>>,
    skip_activity_check: bool,
) -> Result<()> {
    let repo = Repo::auto_open_cwd()?;

    let crate_ = repo.find_crate(name, version, unrelated)?;
    let crate_root = crate_.root();
    let effective_crate_version = crate_.version();

    assert!(!crate_root.starts_with(std::env::current_dir()?));
    let local = Local::auto_open()?;

    let diff_base_version = crate_review_activity_check(
        &local,
        name,
        &effective_crate_version,
        &diff_version,
        skip_activity_check,
    )?;

    let (digest_clean, vcs) =
        check_package_clean_state(&repo, &crate_root, name, &effective_crate_version)?;

    let diff_base = if let Some(ref diff_base_version) = diff_base_version {
        let crate_ = repo.find_crate(
            name,
            Some(diff_base_version),
            UnrelatedOrDependency::Unrelated,
        )?;
        let crate_root = crate_.root();

        let (digest, vcs) =
            check_package_clean_state(&repo, &crate_root, name, &diff_base_version)?;

        Some(proof::PackageInfo {
            id: None,
            source: PROJECT_SOURCE_CRATES_IO.to_owned(),
            name: name.to_owned(),
            version: diff_base_version.to_owned(),
            digest: digest.into_vec(),
            digest_type: proof::default_digest_type(),
            revision: vcs_info_to_revision_string(vcs),
            revision_type: proof::default_revision_type(),
        })
    } else {
        None
    };

    let id = local.read_current_unlocked_id(&crev_common::read_passphrase)?;

    let db = local.load_db()?;
    let mut review = proof::review::PackageBuilder::default()
        .from(id.id.to_owned())
        .package(proof::PackageInfo {
            id: None,
            source: PROJECT_SOURCE_CRATES_IO.to_owned(),
            name: name.to_owned(),
            version: effective_crate_version.to_owned(),
            digest: digest_clean.into_vec(),
            digest_type: proof::default_digest_type(),
            revision: vcs_info_to_revision_string(vcs),
            revision_type: proof::default_revision_type(),
        })
        .review(if advise_common.is_some() || report_severity.is_some() {
            crev_data::Review::new_none()
        } else {
            trust.to_review()
        })
        .diff_base(diff_base)
        .build()
        .map_err(|e| format_err!("{}", e))?;

    let previous_date =
        if let Some((prev_date, prev_review, prev_advisories, prev_issues, prev_comment)) =
            find_previous_review_data(
                &db,
                &id.id,
                name,
                effective_crate_version,
                &diff_base_version,
            )
        {
            review.review = prev_review;
            review.comment = prev_comment;
            review.advisories = prev_advisories;
            review.issues = prev_issues;
            prev_date
        } else {
            None
        };

    if let Some(advise_common) = advise_common {
        let mut advisory: proof::review::package::Advisory = advise_common.affected.into();
        advisory.severity = advise_common.severity;
        review.advisories.push(advisory);
    }
    if let Some(severity) = report_severity {
        let mut report = proof::review::package::Issue::new_with_severity("".into(), severity);
        report.severity = severity;
        review.issues.push(report);
        review.review.rating = Rating::Negative;
    }
    let review = crev_lib::util::edit_proof_content_iteractively(
        &review.into(),
        previous_date.as_ref(),
        diff_base_version.as_ref(),
    )?;

    let proof = review.sign_by(&id)?;

    let commit_msg = format!(
        "Add review for {crate} v{version}",
        crate = name,
        version = effective_crate_version
    );
    maybe_store(&local, &proof, &commit_msg, proof_create_opt)
}

pub fn find_previous_review_data(
    db: &crev_lib::ProofDB,
    id: &crev_data::PubId,
    name: &str,
    crate_version: &Version,
    diff_base_version: &Option<Version>,
) -> Option<(
    Option<crev_data::proof::Date>,
    crev_data::proof::review::Review,
    Vec<crev_data::proof::review::package::Advisory>,
    Vec<crev_data::proof::review::package::Issue>,
    String,
)> {
    if let Some(previous_review) =
        db.get_pkg_review(PROJECT_SOURCE_CRATES_IO, name, crate_version, &id.id)
    {
        return Some((
            Some(previous_review.date),
            previous_review.review.to_owned(),
            previous_review.advisories.to_owned(),
            previous_review.issues.to_owned(),
            previous_review.comment.to_owned(),
        ));
    } else if let Some(diff_base_version) = diff_base_version {
        if let Some(base_review) =
            db.get_pkg_review(PROJECT_SOURCE_CRATES_IO, name, &diff_base_version, &id.id)
        {
            return Some((
                None,
                base_review.review.to_owned(),
                vec![],
                vec![],
                base_review.comment.to_owned(),
            ));
        }
    }
    None
}

pub fn find_reviews(crate_: &opts::CrateSelector) -> Result<Vec<proof::review::Package>> {
    let local = crev_lib::Local::auto_open()?;
    let db = local.load_db()?;
    Ok(db
        .get_package_reviews_for_package(
            PROJECT_SOURCE_CRATES_IO,
            crate_.name.as_ref().map(String::as_str),
            crate_.version.as_ref(),
        )
        .cloned()
        .collect())
}

pub fn list_reviews(crate_: &opts::CrateSelector) -> Result<()> {
    for review in find_reviews(crate_)? {
        println!("{}", review);
    }

    Ok(())
}
