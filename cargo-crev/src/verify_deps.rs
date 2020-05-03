use cargo::core::{package::PackageSet, Package};
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
use crate::table::*;
use crate::tokei::get_rust_line_count;
use crev_data::Digest;
use crev_lib::proofdb::ProofDB;
use crev_lib::VerificationStatus;
use std::io;

use crossterm::{AlternateScreen, Color::*, TerminalCursor};
use minimad::Alignment;
use termimad;

pub fn run(args: Verify) -> Result<()> {
    let repo = Repo::auto_open_cwd()?;
    let package_set = repo.non_local_dep_crates()?;
    let mut source = repo.load_source()?;

    let mut unclean_digests = BTreeMap::new();
    let cursor = TerminalCursor::new();
    cursor.hide()?;
    let table = run_on_deps(args, package_set, &mut unclean_digests, &mut source)?;
    cursor.show()?;

    table.print();

    if !unclean_digests.is_empty() {
        println!();
    }
    for (name, version) in unclean_digests.keys() {
        print!("Unclean crate {} {}\n", name, version);
    }
    if !unclean_digests.is_empty() {
        bail!("Unclean packages detected. Use `cargo crev crate clean` to wipe all unclean crates.");
    }

    Ok(())
}

// run in the alternate screen
fn run_on_deps<'a>(
    args: Verify,
    package_set: PackageSet,
    unclean_digests: &mut BTreeMap<(String, String), Digest>,
    source: &mut Box<dyn cargo::core::source::Source + 'a>,
) -> Result<Table> {
    let _alt_screen = AlternateScreen::to_alternate(true);
    let local = crev_lib::Local::auto_create_or_open()?;
    let db = local.load_db()?;
    let ignore_list = cargo_min_ignore_list();
    let trust_set = if let Some(for_id) = local.get_for_id_from_str_opt(args.for_id.as_deref())? {
        db.calculate_trust_set(&for_id, &args.trust_params.clone().into())
    } else {
        crev_lib::proofdb::TrustSet::default()
    };
    let crates_io = crates_io::Client::new(&local)?;
    let requirements = crev_lib::VerificationRequirements::from(args.requirements.clone());
    let known_owners = read_known_owners_list().unwrap_or_else(|_| HashSet::new());

    let mut table = Table::new();
    if args.verbose {
        table.center_col("digest");
    }
    table.left_col("crate");
    table.left_col("version");
    table.left_col("latest_t");
    table.center_col("trust");
    table.right_col("vers. rev.");
    table.left_col("reviews");
    table.right_col("vers. down.");
    table.left_col("downloads");
    table.center_col("own");
    table.center_col("advsr");
    table.center_col("lines");
    table.center_col("geiger");
    table.center_col("flags");

    table.display_view()?;
    let pkgs = package_set.get_many(package_set.package_ids())?;
    let nb_packages = pkgs.len();
    let cursor = TerminalCursor::new();
    let (_, h) = termimad::terminal_size();
    for (idx, pkg) in pkgs.iter().enumerate() {
        if !pkg.summary().source_id().is_registry() {
            continue;
        }

        if !pkg.root().exists() {
            source.download(pkg.package_id())?;
        }
        let crate_ = &pkg;
        let crate_id = crate_.package_id();
        let crate_name = crate_id.name().as_str();
        let crate_version = crate_id.version();
        let crate_root = crate_.root();

        let digest = crev_lib::get_dir_digest(&crate_root, &ignore_list)?;

        if !is_digest_clean(&db, &crate_name, &crate_version, &digest) {
            unclean_digests.insert(
                (crate_name.to_string(), crate_version.to_string()),
                digest.clone(),
            );
        }

        let result = crev_lib::verify_package_digest(&digest, &trust_set, &requirements, &db);

        if result.is_verified() && args.skip_verified {
            continue;
        }

        let latest_trusted_version = crev_lib::find_latest_trusted_version(
            &trust_set,
            PROJECT_SOURCE_CRATES_IO,
            &crate_name,
            &requirements,
            &db,
        );
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
                continue;
            }
            (Some(known_owners_count), Some(total_owners_count))
        } else {
            (None, None)
        };

        let mut row = String::new();
        row.push('|');
        if args.verbose {
            row.push_str(&format!(" {}|", digest));
        }
        row.push_str(&format!(
            " {}| {}|",
            crate_name,
            pad_left_manually(crate_version.to_string(), 15)
        ));

        row.push_str(&format!(
            " {}|",
            latest_trusted_version_string(crate_version.clone(), latest_trusted_version)
        ));
        //term.print(
        //    format_args!("{:6}", result),
        //    term::verification_status_color(&result),
        //)?;
        row.push_str(match result {
            VerificationStatus::Verified => " high|",
            VerificationStatus::Insufficient => " |",
            VerificationStatus::Flagged => " *flag*|",
            VerificationStatus::Dangerous => " **/!\\**|",
        });
        row.push_str(&format!(
            " {}| {}|",
            pkg_version_review_count, pkg_review_count,
        ));
        //row.push_str(&format!(
        //    format_args!(" {}|", version_downloads_str),
        //    if version_downloads < 1000 {
        //        Some(::term::color::YELLOW)
        //    } else {
        //        None
        //    },
        //)?;
        row.push_str(&format!(" {}|", version_downloads_str));
        //term.print(
        //    format_args!(" {}|", total_downloads_str),
        //    if total_downloads < 10000 {
        //        Some(::term::color::YELLOW)
        //    } else {
        //        None
        //    },
        //)?;
        row.push_str(&format!(" {}|", total_downloads_str));
        //term.print(
        //    format_args!(
        //        " {}",
        //        &known_owners_count
        //            .map(|c| c.to_string())
        //            .unwrap_or_else(|| "?".into())
        //    ),
        //    term::known_owners_count_color(known_owners_count.unwrap_or(0)),
        //)?;
        row.push_str(&format!(
            " {}",
            &known_owners_count
                .map(|c| c.to_string())
                .unwrap_or_else(|| "?".into())
        ));
        row.push_str(&format!(
            "/{} |",
            total_owners_count
                .map(|c| c.to_string())
                .unwrap_or_else(|| "?".into())
        ));

        let advisories =
            db.get_advisories_for_version(PROJECT_SOURCE_CRATES_IO, crate_name, crate_version);
        let trusted_advisories = advisories
            .iter()
            .filter(|(_version, package_review)| {
                trust_set.contains_trusted(&package_review.from.id)
            })
            .count();

        //term.print(
        //    format_args!("{}|", trusted_advisories),
        //    if trusted_advisories > 0 {
        //        Some(::term::color::RED)
        //    } else {
        //        None
        //    },
        //)?;
        row.push_str(&if trusted_advisories > 0 {
            format!(" *{}*", trusted_advisories)
        } else {
            format!(" {}", trusted_advisories)
        });
        row.push_str(&format!("/"));
        //term.print(
        //    format_args!("{}|", advisories.len()),
        //    if advisories.is_empty() {
        //        None
        //    } else {
        //        Some(::term::color::YELLOW)
        //    },
        //)?;
        row.push_str(&format!("{} |", advisories.len()));
        row.push_str(&format!(
            "{}| {}|",
            get_rust_line_count(crate_root)
                .ok()
                .map(|n| n.to_string())
                .unwrap_or_else(|| "err".into()),
            get_geiger_count(crate_root)
                .map(|n| n.to_string())
                .unwrap_or_else(|_| "err".into()),
        ));
        //term.print(
        //    format_args!(" {}|", if crate_.has_custom_build() {}| else {}|),
        //    ::term::color::YELLOW,
        //)?;
        row.push_str(&format!(
            " {}|",
            if crate_.has_custom_build() {
                "*CB*"
            } else {
                ""
            }
        ));
        table.add_row(row);
        table.display_view()?;
        cursor.goto(0, h - 1)?;
        table.skin.print_inline(&format!(
            "**Cargo Crev** verify deps: *{}* / {}",
            idx + 1,
            nb_packages
        ));
    }

    Ok(table)
}
