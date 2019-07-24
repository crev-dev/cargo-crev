// Functions related to printing dependencies in the standard
// terminal (not in the context of a real terminal application)

use crate::dep::dep::*;
use crate::prelude::*;
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

pub fn term_print_header(_term: &mut Term, verbose: bool) {
    if verbose {
        eprint!("{:43} ", "digest");
    }
    eprint!(
        "{:6} {:8} {:^15} {:4} {:6} {:6} {:6} {:4}",
        "status", "reviews", "downloads", "own.", "issues", "lines", "geiger", "flgs"
    );
    eprintln!(" {:<20} {:<15} {:<15}", "crate", "version", "latest_t");
}

pub fn term_print_computed_dep(cdep: &ComputedDep, term: &mut Term, verbose: bool) -> Result<()> {
    if verbose {
        print!("{:43} ", cdep.digest);
    }
    term.print(
        format_args!("{:6}", cdep.trust),
        term::verification_status_color(&cdep.trust),
    )?;
    print!(" {:2} {:2}", cdep.reviews.version, cdep.reviews.total);
    if let Some(downloads) = &cdep.downloads {
        term.print(
            format_args!(" {:>8}", downloads.version),
            if downloads.version < 1000 {
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
    if let Some(owners) = &cdep.owners {
        term.print(
            format_args!(" {}", owners.trusted),
            term::known_owners_count_color(owners.trusted),
        )?;
        term.print(format_args!(" {}", owners.total), None)?;
    } else {
        println!(" ???");
    }

    term.print(
        format_args!("{:4}", cdep.issues.trusted),
        if cdep.issues.trusted > 0 {
            Some(::term::color::RED)
        } else {
            None
        },
    )?;
    print!("/");
    term.print(
        format_args!("{:<2}", cdep.issues.total),
        if cdep.issues.total > 0 {
            Some(::term::color::YELLOW)
        } else {
            None
        },
    )?;
    match cdep.loc {
        Some(loc) => print!(" {:>6}", loc),
        None => print!(" {:>6}", "err"),
    }

    Ok(())
}

fn term_print_dep_id(dep: &Dep, _term: &mut Term) {
    print!(
        " {:<20} {:<15}",
        dep.name,
        pad_left_manually(dep.version.to_string(), 15)
    );
}

pub fn term_print_dep(dep: &Dep, term: &mut Term, verbose: bool) -> Result<()> {
    match &dep.computation_status {
        DepComputationStatus::Failed => {
            term_print_dep_id(dep, term);
            println!(" -- computation failed");
        }
        DepComputationStatus::Skipped => {
            term_print_dep_id(dep, term);
            println!(" -- skipped");
        }
        DepComputationStatus::Ok { computed_dep } => {
            term_print_computed_dep(&computed_dep, term, verbose)?;
            match dep.geiger_count {
                Some(geiger_count) => print!(" {:>7}", geiger_count),
                None => print!(" {:>7}", "err"),
            }
            term.print(
                format_args!(" {:4}", if dep.has_custom_build { "CB" } else { "" }),
                ::term::color::YELLOW,
            )?;
            term_print_dep_id(dep, term);
            print!(
                " {}",
                latest_trusted_version_string(&dep.version, &computed_dep.latest_trusted_version)
            );
            println!();
        }
    }
    Ok(())
}
