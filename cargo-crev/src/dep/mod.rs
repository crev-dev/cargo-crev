use crev_common::convert::OptionDeref;
use crev_lib;
use std::{
    collections::{BTreeMap, HashSet},
    default::Default,
};

use crate::crates_io;
use crate::opts::*;
use crate::prelude::*;
use crate::repo::*;
use crate::shared::*;
use crate::term;
use crate::tokei;

pub fn verify_deps(args: Verify) -> Result<CommandExitStatus> {
    let mut term = term::Term::new();
    let local = crev_lib::Local::auto_create_or_open()?;
    let db = local.load_db()?;

    let trust_set = if let Some(for_id) = local.get_for_id_from_str_opt(args.for_id.as_deref())? {
        db.calculate_trust_set(&for_id, &args.trust_params.clone().into())
    } else {
        crev_lib::proofdb::TrustSet::default()
    };

    let repo = Repo::auto_open_cwd()?;
    let ignore_list = cargo_min_ignore_list();
    let crates_io = crates_io::Client::new(&local)?;

    if term.stderr_is_tty && term.stdout_is_tty {
        if args.verbose {
            eprint!("{:43} ", "digest");
        }
        eprint!(
            "{:6} {:8} {:^15} {:4} {:6} {:6} {:6} {:4}",
            "status", "reviews", "downloads", "own.", "issues", "lines", "geiger", "flgs"
        );
        eprintln!(" {:<20} {:<15} {:<15}", "crate", "version", "latest_t");
    }
    let requirements = crev_lib::VerificationRequirements::from(args.requirements.clone());
    let mut unclean_digests = BTreeMap::new();
    let known_owners = read_known_owners_list().unwrap_or_else(|_| HashSet::new());
    let mut total_verification_successful = true;
    repo.for_every_non_local_dep_crate(|crate_| {
        let crate_id = crate_.package_id();
        let crate_name = crate_id.name().as_str();
        let crate_version = crate_id.version();
        let crate_root = crate_.root();

        let digest = crev_lib::get_dir_digest(&crate_root, &ignore_list)?;

        if !is_digest_clean(&db, &crate_name, &crate_version, &digest) {
            unclean_digests.insert((crate_name, crate_version), digest.clone());
        }

        let result = db.verify_package_digest(&digest, &trust_set, &requirements);

        if !result.is_verified() {
            total_verification_successful = false;
        }

        if result.is_verified() && args.skip_verified {
            return Ok(());
        }

        let pkg_review_count =
            db.get_package_review_count(PROJECT_SOURCE_CRATES_IO, Some(crate_name), None);
        let pkg_version_review_count = db.get_package_review_count(
            PROJECT_SOURCE_CRATES_IO,
            Some(crate_name),
            Some(&crate_version),
        );

        let (version_downloads_str, total_downloads_str, version_downloads, total_downloads) =
            crates_io
                .get_downloads_count(&crate_name, &crate_version)
                .map(|(a, b)| (a.to_string(), b.to_string(), a, b))
                .unwrap_or_else(|_e| ("err".into(), "err".into(), 0, 0));

        let owners = crates_io.get_owners(&crate_name).ok();
        let (known_owners_count, total_owners_count) = if let Some(owners) = owners {
            let total_owners_count = owners.len();
            let known_owners_count = owners
                .iter()
                .filter(|o| known_owners.contains(o.as_str()))
                .count();

            if known_owners_count > 0 && args.skip_known_owners {
                return Ok(());
            }
            (Some(known_owners_count), Some(total_owners_count))
        } else {
            (None, None)
        };

        if args.verbose {
            print!("{:43} ", digest);
        }
        term.print(
            format_args!("{:6}", result),
            term::verification_status_color(&result),
        )?;
        print!(" {:2} {:2}", pkg_version_review_count, pkg_review_count,);
        term.print(
            format_args!(" {:>8}", version_downloads_str),
            if version_downloads < 1000 {
                Some(::term::color::YELLOW)
            } else {
                None
            },
        )?;
        term.print(
            format_args!(" {:>9}", total_downloads_str),
            if total_downloads < 10000 {
                Some(::term::color::YELLOW)
            } else {
                None
            },
        )?;
        term.print(
            format_args!(
                " {}",
                &known_owners_count
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "?".into())
            ),
            term::known_owners_count_color(known_owners_count.unwrap_or(0)),
        )?;
        print!(
            "/{} ",
            total_owners_count
                .map(|c| c.to_string())
                .unwrap_or_else(|| "?".into())
        );

        let issues_from_trusted = db.get_open_issues_for_version(
            PROJECT_SOURCE_CRATES_IO,
            crate_name,
            crate_version,
            &trust_set,
            args.requirements.trust_level.into(),
        );

        let issues_from_all = db.get_open_issues_for_version(
            PROJECT_SOURCE_CRATES_IO,
            crate_name,
            crate_version,
            &trust_set,
            crev_data::Level::None.into(),
        );

        term.print(
            format_args!("{:4}", issues_from_trusted.len()),
            if !issues_from_trusted.is_empty() {
                Some(::term::color::RED)
            } else {
                None
            },
        )?;
        print!("/");
        term.print(
            format_args!("{:<2}", issues_from_all.len()),
            if !issues_from_all.is_empty() {
                Some(::term::color::YELLOW)
            } else {
                None
            },
        )?;
        print!(
            "{:>6} {:>7}",
            tokei::get_rust_line_count(crate_root)
                .ok()
                .map(|n| n.to_string())
                .unwrap_or_else(|| "err".into()),
            get_geiger_count(crate_root)
                .map(|n| n.to_string())
                .unwrap_or_else(|_| "err".into()),
        );
        term.print(
            format_args!(" {:4}", if crate_.has_custom_build() { "CB" } else { "" }),
            ::term::color::YELLOW,
        )?;
        print!(
            " {:<20} {:<15}",
            crate_name,
            pad_left_manually(crate_version.to_string(), 15)
        );

        let latest_trusted_version = db.find_latest_trusted_version(
            &trust_set,
            PROJECT_SOURCE_CRATES_IO,
            &crate_name,
            &requirements,
        );
        print!(
            " {}",
            latest_trusted_version_string(crate_version.clone(), latest_trusted_version)
        );
        println!();

        Ok(())
    })?;

    if !unclean_digests.is_empty() {
        eprintln!();
    }

    for (name, version) in unclean_digests.keys() {
        term.eprint(
            format_args!("Unclean crate {} {}\n", name, version),
            ::term::color::RED,
        )?;
    }

    if !unclean_digests.is_empty() {
        bail!(
            "Unclean packages detected. Use `cargo crev clean <crate>` to wipe the local source."
        );
    }

    return Ok(if total_verification_successful {
        CommandExitStatus::Successs
    } else {
        CommandExitStatus::VerificationFailed
    });
}
