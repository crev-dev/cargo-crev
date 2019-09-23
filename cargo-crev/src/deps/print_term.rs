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

pub fn print_header(_term: &mut Term, verbose: bool) {
    if verbose {
        eprint!("{:43} ", "digest");
    }
    eprint!(
        "{:6} {:8} {:^15} {:6} {:6} {:6} {:6} {:4}",
        "status", "reviews", "downloads", "owner", "issues", "lines", "geiger", "flgs"
    );
    eprintln!(" {:<20} {:<15} {:<15}", "crate", "version", "latest_t");
}

#[allow(clippy::collapsible_if)]
pub fn print_details(
    cdep: &CrateDetails,
    term: &mut Term,
    verbose: bool,
    recursive_mode: bool,
) -> Result<()> {
    if verbose {
        print!("{:43} ", cdep.digest);
    }
    term.print(
        format_args!("{:6}", cdep.accumulative.trust),
        term::verification_status_color(cdep.accumulative.trust),
    )?;
    print!(
        " {:2} {:2}",
        cdep.version_reviews.count, cdep.version_reviews.total
    );
    if let Some(downloads) = &cdep.version_downloads {
        term.print(
            format_args!(" {:>8}", downloads.count),
            if downloads.count < 1000 {
                Some(::term::color::YELLOW)
            } else {
                None
            },
        )?;
        term.print(
            format_args!(" {:>9}", downloads.total),
            if downloads.total < 10000 {
                Some(::term::color::YELLOW)
            } else {
                None
            },
        )?;
    } else {
        println!(" {:>8} {:>9}", "?", "?");
    }

    if recursive_mode {
        term.print(
            format_args!(
                " {:>2} {:>2}",
                cdep.accumulative.owner_set.to_total_owners(),
                cdep.accumulative.owner_set.to_total_distinct_groups()
            ),
            None,
        )?;
    } else {
        if let Some(owners) = &cdep.known_owners {
            term.print(
                format_args!(" {:>2}", owners.count),
                term::known_owners_count_color(owners.count),
            )?;
            term.print(format_args!("/{:<2}", owners.total), None)?;
        } else {
            println!(" ???");
        }
    }

    term.print(
        format_args!("{:4}", cdep.accumulative.trusted_issues.count),
        if cdep.accumulative.trusted_issues.count > 0 {
            Some(::term::color::RED)
        } else {
            None
        },
    )?;
    print!("/");
    term.print(
        format_args!("{:<2}", cdep.accumulative.trusted_issues.total),
        if cdep.accumulative.trusted_issues.total > 0 {
            Some(::term::color::YELLOW)
        } else {
            None
        },
    )?;
    match cdep.accumulative.loc {
        Some(loc) => print!(" {:>6}", loc),
        None => print!(" {:>6}", "err"),
    }

    Ok(())
}

fn print_stats_crate_id(stats: &CrateStats, _term: &mut Term) {
    print!(
        " {:<20} {:<15}",
        stats.info.id.name(),
        pad_left_manually(stats.info.id.version().to_string(), 15)
    );
}

pub fn print_dep(
    stats: &CrateStats,
    term: &mut Term,
    verbose: bool,
    recursive_mode: bool,
) -> Result<()> {
    match &stats.details {
        Err(_) => {
            print_stats_crate_id(stats, term);
            println!(" -- computation failed");
        }
        Ok(None) => { /* just skip */ }
        Ok(Some(details)) => {
            print_details(&details, term, verbose, recursive_mode)?;
            match details.accumulative.geiger_count {
                Some(geiger_count) => print!(" {:>7}", geiger_count),
                None => print!(" {:>7}", "err"),
            }
            term.print(
                format_args!(
                    " {:4}",
                    if let Some(has_custom_build) = stats.has_custom_build() {
                        if has_custom_build {
                            "CB"
                        } else {
                            ""
                        }
                    } else {
                        "?"
                    }
                ),
                ::term::color::YELLOW,
            )?;
            print_stats_crate_id(stats, term);
            print!(
                " {}",
                latest_trusted_version_string(
                    &stats.info.id.version(),
                    &details.latest_trusted_version
                )
            );
            println!();
        }
    }
    Ok(())
}
