use crate::{
    edit, opts,
    opts::{CargoOpts, CrateSelector},
    prelude::*,
    term, url_to_status_str,
};
use anyhow::format_err;
use crev_data::{
    proof::{self, ContentExt},
    Rating,
};
use crev_lib::{self, local::Local, TrustProofType};
use std::{default::Default, fmt::Write};

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
    show_override_suggestions: bool,
    cargo_opts: CargoOpts,
) -> Result<()> {
    let repo = Repo::auto_open_cwd(cargo_opts)?;

    let pkg_id = repo.find_pkgid_by_crate_selector(crate_sel)?;
    let crate_ = repo.get_crate(&pkg_id)?;
    let crate_root = crate_.root();
    let effective_crate_version = crate_.version();

    assert!(!crate_root.starts_with(std::env::current_dir()?));
    let local = Local::auto_open()?;

    let diff_base_version = match crate_review_activity_check(
        &local,
        &pkg_id.name(),
        effective_crate_version,
        diff_version,
        skip_activity_check,
    ) {
        Ok(res) => res,
        Err(ActivityCheckError::NoPreviousReview) => bail!("No previous review activity to determine base version"),
        Err(ActivityCheckError::UnexpectedFullReview) => bail!(
            "Last review activity record for {}:{} indicates full review. \
             Use `--diff` flag? Use `--skip-activity-check` to override.",
            pkg_id.name(),
            effective_crate_version
        ),
        Err(ActivityCheckError::UnexpectedDiffReview) => bail!(
            "Last review activity record for {}:{} indicates differential review. \
             Use `--diff` flag? Use `--skip-activity-check` to override.",
            pkg_id.name(),
            effective_crate_version
        ),
        Err(ActivityCheckError::Expired) =>  bail!(
            "Last review activity record for {}:{} is too old. \
             Re-review or use `--skip-activity-check` to override.",
            pkg_id.name(),
            effective_crate_version
        ),
        Err(ActivityCheckError::NoRecord) => bail!(
            "No review activity record for {name}:{} found. \
             Make sure you have reviewed the code in this version before creating review proof. \n\
             Use `cargo crev open {name}` or `cargo crev goto {name}` to review the code, or `--skip-activity-check` to override.",
            effective_crate_version,
            name = pkg_id.name(),
        ),
        Err(ActivityCheckError::Other(e)) => return Err(e.into()),
    };

    let (digest_clean, vcs) =
        check_package_clean_state(&repo, crate_root, &crate_.name(), effective_crate_version)?;

    let diff_base = if let Some(ref diff_base_version) = diff_base_version {
        let crate_id = repo.find_pkgid(&crate_.name(), Some(diff_base_version), true)?;
        let crate_ = repo.get_crate(&crate_id)?;
        let crate_root = crate_.root();

        let (digest, vcs) =
            check_package_clean_state(&repo, crate_root, &crate_.name(), diff_base_version)?;

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

    let id = local.read_current_unlocked_id(&term::read_passphrase)?;

    let db = local.load_db()?;

    let default_review_content = if advise_common.is_some() || report_severity.is_some() {
        crev_data::Review::new_none()
    } else {
        trust.to_review()
    };

    let (previous_date, mut review) = if let Some(mut previous_review) = db
        .get_pkg_review(
            PROJECT_SOURCE_CRATES_IO,
            &crate_.name(),
            effective_crate_version,
            &id.id.id,
        )
        .cloned()
    {
        if trust == TrustProofType::Untrust {
            *previous_review.review_possibly_none_mut() = default_review_content;
        }
        (Some(previous_review.common.date), previous_review)
    } else {
        let mut fresh_review = proof::review::PackageBuilder::default()
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
            .review(default_review_content)
            .diff_base(diff_base)
            .build()
            .map_err(|e| format_err!("{}", e))?;

        if let Some(diff_base_version) = diff_base_version.clone() {
            if let Some(base_review) = db.get_pkg_review(
                PROJECT_SOURCE_CRATES_IO,
                &crate_.name(),
                &diff_base_version,
                &id.id.id,
            ) {
                fresh_review.comment = base_review.comment.to_owned();
                *fresh_review.review_possibly_none_mut() =
                    base_review.review_possibly_none().to_owned()
            }
        }
        (None, fresh_review)
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
        review.review_possibly_none_mut().rating = Rating::Negative;
    }

    review.flags = db
        .get_pkg_flags_by_author(&id.id.id, &review.package.id.id)
        .cloned()
        .unwrap_or_default();

    review.alternatives = db.get_pkg_alternatives_by_author(&id.id.id, &review.package.id.id);

    let mut review = edit::edit_proof_content_iteractively(
        &review,
        previous_date.as_ref(),
        diff_base_version.as_ref(),
        None,
        |text| {
            if show_override_suggestions && review.override_.is_empty() {
                writeln!(text, "# override:")?;
            }

            if show_override_suggestions {
                for review in db.get_package_reviews_for_package(
                    PROJECT_SOURCE_CRATES_IO,
                    Some(&pkg_id.name()),
                    Some(&pkg_id.version()),
                ) {
                    let id = &review.common.from.id;
                    let (status, url) = url_to_status_str(&db.lookup_url(id));
                    writeln!(text, "# - id-type: {}", "crev")?; // TODO: support other ids?
                    writeln!(text, "#   id: {}", id)?;
                    writeln!(text, "#   url: {} # {}", url, status)?;
                    writeln!(text, "#   comment: \"\"")?;
                }
            }

            Ok(())
        },
    )?;

    review.touch_date();
    let proof = review.sign_by(&id)?;

    let commit_msg = format!(
        "Add review for {crate} v{version}",
        crate = &crate_.name(),
        version = effective_crate_version
    );
    maybe_store(&local, &proof, &commit_msg, proof_create_opt)
}

pub fn find_reviews(crate_: &opts::CrateSelector) -> Result<Vec<proof::review::Package>> {
    let local = crev_lib::Local::auto_open()?;
    let db = local.load_db()?;
    Ok(db
        .get_package_reviews_for_package(
            PROJECT_SOURCE_CRATES_IO,
            crate_.name.as_deref(),
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
