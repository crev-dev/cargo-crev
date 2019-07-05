use cargo::{
    core::{dependency::Dependency, source::SourceMap, Package, SourceId},
    util::important_paths::find_root_manifest_for_wd,
};
use crev_common::convert::OptionDeref;
use crev_lib::{self, local::Local, ProofStore, ReviewMode};
use failure::format_err;
use insideout::InsideOutIter;
use resiter::FlatMap;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashSet},
    default::Default,
    env,
    io::BufRead,
    path::{Path, PathBuf},
    process,
};
use structopt::StructOpt;

use crate::prelude::*;
use crate::crates_io::{self, *};
use crate::opts::{self, *};
use crate::repo::{self, *};
use crate::unsorted_mess::*;
use crev_data::proof;
use crev_lib::TrustOrDistrust::{self, *};
use crate::tokei::get_rust_line_count;

pub fn run(args: VerifyDeps) -> Result<CommandExitStatus> {
    let local = crev_lib::Local::auto_create_or_open()?;
    let db = local.load_db()?;

    let trust_set =
        if let Some(for_id) = local.get_for_id_from_str_opt(args.for_id.as_deref())? {
            db.calculate_trust_set(&for_id, &args.trust_params.clone().into())
        } else {
            crev_lib::proofdb::TrustSet::default()
        };

    let repo = Repo::auto_open_cwd()?;
    let ignore_list = cargo_min_ignore_list();
    let crates_io = crates_io::Client::new(&local)?;

    // if term.stderr_is_tty && term.stdout_is_tty {
    //     if args.verbose {
    //         eprint!("{:43} ", "digest");
    //     }
    //     eprint!(
    //         "{:6} {:8} {:^15} {:4} {:6} {:6} {:6} {:4}",
    //         "status",
    //         "reviews",
    //         "downloads",
    //         "own.",
    //         "advisr",
    //         "lines",
    //         "geiger",
    //         "flgs"
    //     );
    //     eprintln!(
    //         " {:<20} {:<15} {:<15}",
    //         "crate", "version", "latest_t"
    //     );
    // }
    if args.verbose {
        print!("{:43} ", "digest");
    }
    print!(
        "{:6} {:8} {:^15} {:4} {:6} {:6} {:6} {:4}",
        "status",
        "reviews",
        "downloads",
        "own.",
        "advisr",
        "lines",
        "geiger",
        "flgs"
    );
    println!(
        " {:<20} {:<15} {:<15}",
        "crate", "version", "latest_t"
    );
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

        let pkg_review_count = db.get_package_review_count(
            PROJECT_SOURCE_CRATES_IO,
            Some(crate_name),
            None,
        );
        let pkg_version_review_count = db.get_package_review_count(
            PROJECT_SOURCE_CRATES_IO,
            Some(crate_name),
            Some(&crate_version),
        );

        let (
            version_downloads_str,
            total_downloads_str,
            version_downloads,
            total_downloads,
        ) = crates_io
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
        //term.print(
        //    format_args!("{:6}", result),
        //    term::verification_status_color(&result),
        //)?;
        print!(
            "{:6}", result
        );
        print!(" {:2} {:2}", pkg_version_review_count, pkg_review_count,);
        //print!(
        //    format_args!(" {:>8}", version_downloads_str),
        //    if version_downloads < 1000 {
        //        Some(::term::color::YELLOW)
        //    } else {
        //        None
        //    },
        //)?;
        print!(
            " {:>8}", version_downloads_str
        );
        //term.print(
        //    format_args!(" {:>9}", total_downloads_str),
        //    if total_downloads < 10000 {
        //        Some(::term::color::YELLOW)
        //    } else {
        //        None
        //    },
        //)?;
        print!(
            " {:>9}", total_downloads_str
        );
        //term.print(
        //    format_args!(
        //        " {}",
        //        &known_owners_count
        //            .map(|c| c.to_string())
        //            .unwrap_or_else(|| "?".into())
        //    ),
        //    term::known_owners_count_color(known_owners_count.unwrap_or(0)),
        //)?;
        print!(
                " {}",
                &known_owners_count
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "?".into())
        );
        print!(
            "/{} ",
            total_owners_count
                .map(|c| c.to_string())
                .unwrap_or_else(|| "?".into())
        );

        let advisories = db.get_advisories_for_version(
            PROJECT_SOURCE_CRATES_IO,
            crate_name,
            crate_version,
        );
        let trusted_advisories = advisories
            .iter()
            .filter(|(_version, package_review)| {
                trust_set.contains_trusted(&package_review.from.id)
            })
            .count();

        //term.print(
        //    format_args!("{:4}", trusted_advisories),
        //    if trusted_advisories > 0 {
        //        Some(::term::color::RED)
        //    } else {
        //        None
        //    },
        //)?;
        print!(
            "{:4}", trusted_advisories
        );
        print!("/");
        //term.print(
        //    format_args!("{:<2}", advisories.len()),
        //    if advisories.is_empty() {
        //        None
        //    } else {
        //        Some(::term::color::YELLOW)
        //    },
        //)?;
        print!(
            "{:<2}", advisories.len()
        );
        print!(
            "{:>6} {:>7}",
            get_rust_line_count(crate_root)
                .ok()
                .map(|n| n.to_string())
                .unwrap_or_else(|| "err".into()),
            get_geiger_count(crate_root)
                .map(|n| n.to_string())
                .unwrap_or_else(|_| "err".into()),
        );
        //term.print(
        //    format_args!(" {:4}", if crate_.has_custom_build() { "CB" } else { "" }),
        //    ::term::color::YELLOW,
        //)?;
        print!(
            " {:4}", if crate_.has_custom_build() { "CB" } else { "" }
        );
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
            latest_trusted_version_string(
                crate_version.clone(),
                latest_trusted_version
            )
        );
        println!();

        Ok(())
    })?;

    if !unclean_digests.is_empty() {
        println!();
    }

    for (name, version) in unclean_digests.keys() {
        print!(
            "Unclean crate {} {}\n", name, version
        );
    }

    if !unclean_digests.is_empty() {
        bail!("Unclean packages detected. Use `cargo crev clean <crate>` to wipe the local source.");
    }

    return Ok(if total_verification_successful {
        CommandExitStatus::Successs
    } else {
        CommandExitStatus::VerificationFailed
    });
}
