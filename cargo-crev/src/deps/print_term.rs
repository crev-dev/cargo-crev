// Functions related to printing dependencies in the standard
// terminal (not in the context of a real terminal application)

use super::*;
use crate::term::{self, *};

fn pad_left_manually(s: String, width: usize) -> String {
    if s.len() <= width {
        let padding = std::iter::repeat(" ")
            .take(width - s.len())
            .collect::<String>();
        format!("{}{}", s, padding)
    } else {
        s
    }
}

pub fn print_header(_term: &mut Term, columns: &CrateVerifyColumns) {
    eprint!("{:>6} ", "status");

    if columns.show_reviews() {
        eprint!("{:>7} ", "reviews");
    }

    if columns.show_issues() {
        eprint!("{:>6} ", "issues");
    }

    if columns.show_owners() {
        eprint!("{:>5} ", "owner");
    }

    if columns.show_downloads() {
        eprint!("{:>14} ", "downloads");
    }

    if columns.show_loc() {
        eprint!("{:>6} ", "loc");
    }

    if columns.show_leftpad_index() {
        eprint!("{:>5} ", "lpidx");
    }

    if columns.show_geiger() {
        eprint!("{:>6} ", "geiger");
    }

    if columns.show_flags() {
        eprint!("{:>4} ", "flgs");
    }

    eprint!("{:<20} {:<12} ", "crate", "version");

    if columns.show_latest_trusted() {
        eprint!("{:<12}", "latest_t");
    }

    if columns.show_digest() {
        eprint!("digest");
    }

    eprintln!();
}

#[allow(clippy::collapsible_if)]
pub fn print_details(
    cdep: &CrateDetails,
    term: &mut Term,
    columns: &CrateVerifyColumns,
    recursive_mode: bool,
) -> Result<()> {
    if cdep.accumulative.is_local_source_code {
        term.print(format_args!("{:6} ", "local"), None)?;
    } else if !cdep.accumulative.has_trusted_ids && cdep.accumulative.trust == VerificationStatus::Insufficient {
        term.print(format_args!("{:6} ", "N/A"), None)?;
    } else {
        term.print(
            format_args!("{:6} ", cdep.accumulative.trust),
            term::verification_status_color(cdep.accumulative.trust),
        )?;
    }

    if columns.show_reviews() {
        print!(
            "{:3} {:3} ",
            cdep.version_reviews.count, cdep.version_reviews.total
        );
    }

    if columns.show_issues() {
        term.print(
            format_args!("{:2} ", cdep.accumulative.trusted_issues.count),
            if cdep.accumulative.trusted_issues.count > 0 {
                Some(::term::color::RED)
            } else {
                None
            },
        )?;
        term.print(
            format_args!("{:3} ", cdep.accumulative.trusted_issues.total),
            if cdep.accumulative.trusted_issues.total > 0 {
                Some(::term::color::YELLOW)
            } else {
                None
            },
        )?;
    }

    if columns.show_owners() {
        if recursive_mode {
            term.print(
                format_args!(
                    "{:>2} {:>2} ",
                    cdep.accumulative.owner_set.to_total_owners(),
                    cdep.accumulative.owner_set.to_total_distinct_groups()
                ),
                None,
            )?;
        } else {
            if let Some(known_owners) = &cdep.known_owners {
                term.print(
                    format_args!("{:>2} ", known_owners.count),
                    term::known_owners_count_color(known_owners.count),
                )?;
                term.print(format_args!("{:>2} ", known_owners.total), None)?;
            } else {
                term.print(
                    format_args!("{:>2} ", "?"),
                    term::known_owners_count_color(0),
                )?;
                term.print(format_args!("{:>2} ", "?"), None)?;
            }
        }
    }

    if columns.show_downloads() {
        if let Some(downloads) = &cdep.downloads {
            term.print(
                format_args!("{:>5}K ", downloads.version / 1000),
                if downloads.version < 2000 {
                    Some(::term::color::YELLOW)
                } else {
                    None
                },
            )?;
            term.print(
                format_args!("{:>6}K ", downloads.total / 1000),
                if downloads.total < 20000 {
                    Some(::term::color::YELLOW)
                } else {
                    None
                },
            )?;
        } else {
            term.print(format_args!("{:>8} {:>9} ", "?", "?"), None)?;
        }
    }

    if columns.show_loc() {
        match cdep.accumulative.loc {
            Some(loc) => print!("{:>6} ", loc),
            None => print!("{:>6} ", "err"),
        }
    }

    if columns.show_leftpad_index() {
        print!("{:>5} ", (cdep.leftpad_idx as f64).sqrt().round() as usize);
    }

    Ok(())
}

fn print_stats_crate_id(stats: &CrateStats, _term: &mut Term) {
    print!(
        "{:<20} {:<12}",
        stats.info.id.name(),
        pad_left_manually(
            stats.info.id.version().to_string()
                + if stats.info.id.source_id().is_registry() {
                    ""
                } else {
                    "*"
                },
            15
        )
    );
}

pub fn print_dep(
    stats: &CrateStats,
    term: &mut Term,
    columns: &CrateVerifyColumns,
    recursive_mode: bool,
) -> Result<()> {
    let details = stats.details();

    print_details(&details, term, columns, recursive_mode)?;
    if columns.show_geiger() {
        match details.accumulative.geiger_count {
            Some(geiger_count) => print!("{:>6} ", geiger_count),
            None => print!("{:>6} ", "err"),
        }
    }

    if columns.show_flags() {
        if stats.has_custom_build() {
            term.print(format_args!("CB"), ::term::color::YELLOW)?;
        } else {
            print!("__");
        }

        if stats.is_unmaintained() {
            print!("UM");
        } else {
            print!("__");
        }
        print!(" ");
    }

    print_stats_crate_id(stats, term);

    if columns.show_latest_trusted() {
        print!(
            "{:<12}",
            latest_trusted_version_string(
                &stats.info.id.version(),
                &details.latest_trusted_version
            )
        );
    }

    if columns.show_digest() {
        print!(
            "{}",
            details
                .digest
                .as_ref()
                .map(|d| d.to_string())
                .unwrap_or_else(|| "-".into())
        );
    }

    println!();
    Ok(())
}
