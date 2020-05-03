use crev_data::Rating;
use crev_lib::{self, local::Local};
use failure::format_err;
use std::default::Default;

use crate::{
    opts,
    opts::{CargoOpts, CrateSelector},
    prelude::*,
};
use crev_data::proof::{self, ContentExt};
use crev_lib::TrustProofType;
use crev_wot;

use crate::{repo::*, shared::*};

/// Review a crate
///
/// * `unrelated` - the crate might not actually be a dependency
#[allow(clippy::option_option)]
pub fn create_review_proof(
    crate_sel: &CrateSelector,
    report_severity: Option<crev_data::Level>,
    advise_common: Option<opts::AdviseCommon>,
    trust: TrustProofType,
    proof_create_opt: &opts::CommonProofCreate,
    diff_version: &Option<Option<Version>>,
    skip_activity_check: bool,
    cargo_opts: CargoOpts,
) -> Result<()> {
    let repo = Repo::auto_open_cwd(cargo_opts)?;

    let pkg_id = repo.find_pkgid_by_crate_selector(crate_sel)?;
    let crate_ = repo.get_crate(&pkg_id)?;
    let crate_root = crate_.root();
    let effective_crate_version = crate_.version();

    assert!(!crate_root.starts_with(std::env::current_dir()?));
    let local = Local::auto_open()?;

    let diff_base_version = crate_review_activity_check(
        &local,
        &pkg_id.name(),
        &effective_crate_version,
        &diff_version,
        skip_activity_check,
    )?;

    let (digest_clean, vcs) =
        check_package_clean_state(&repo, &crate_root, &crate_.name(), &effective_crate_version)?;

    let diff_base = if let Some(ref diff_base_version) = diff_base_version {
        let crate_id = repo.find_pkgid(&crate_.name(), Some(diff_base_version), true)?;
        let crate_ = repo.get_crate(&crate_id)?;
        let crate_root = crate_.root();

        let (digest, vcs) =
            check_package_clean_state(&repo, &crate_root, &crate_.name(), &diff_base_version)?;

        Some(proof::PackageInfo {
            id: proof::PackageVersionId::new(
                PROJECT_SOURCE_CRATES_IO.to_owned(),
                crate_.name().to_string(),
                diff_base_version.to_owned(),
            ),
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
            id: proof::PackageVersionId::new(
                PROJECT_SOURCE_CRATES_IO.to_owned(),
                crate_.name().to_string(),
                effective_crate_version.to_owned(),
            ),
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
                &crate_.name(),
                effective_crate_version,
                &diff_base_version,
            )
        {
            if trust != TrustProofType::Untrust {
                review.review = prev_review;
            }
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

    review.flags = db
        .get_pkg_flags_by_author(&id.id.id, &review.package.id.id)
        .cloned()
        .unwrap_or_default();

    review.alternatives = db.get_pkg_alternatives_by_author(&id.id.id, &review.package.id.id);

    let review = crev_lib::util::edit_proof_content_iteractively(
        &review,
        previous_date.as_ref(),
        diff_base_version.as_ref(),
    )?;

    let proof = review.sign_by(&id)?;

    let commit_msg = format!(
        "Add review for {crate} v{version}",
        crate = &crate_.name(),
        version = effective_crate_version
    );
    maybe_store(&local, &proof, &commit_msg, proof_create_opt)
}

pub fn find_previous_review_data(
    db: &crev_wot::ProofDB,
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
            Some(previous_review.common.date),
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
            crate_.version()?,
        )
        .cloned()
        .collect())
}

pub fn list_reviews(crate_: &opts::CrateSelector) -> Result<()> {
    for review in find_reviews(crate_)? {
        println!("---\n{}", review);
    }

    Ok(())
}
