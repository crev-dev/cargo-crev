// Functions related to writing dependencies in the standard
// terminal (not in the context of a real terminal application)

use std::io::Write;
use std::{io, write, writeln};

use super::*;
use crate::term::{self, Term};

const CRATE_VERIFY_CRATE_COLUMN_TITLE: &str = "crate";
const CRATE_VERIFY_VERSION_COLUMN_TITLE: &str = "version";
const TRUNCATED_VERSION_WIDTH: usize = 10;
const TRUNCATED_LATEST_T_WIDTH: usize = 10;

#[derive(Copy, Clone, Debug)]
pub struct VerifyOutputColumnWidths {
    pub name: usize,
    pub version: usize,
    pub latest_trusted: usize,
    pub human: bool,
}

impl VerifyOutputColumnWidths {
    pub fn from_pkgsids<'a>(
        pkgs_ids: impl Iterator<Item = &'a cargo::core::PackageId>,
        human: bool,
    ) -> Self {
        let (name, version) = pkgs_ids.fold(
            (
                CRATE_VERIFY_CRATE_COLUMN_TITLE.len(),
                CRATE_VERIFY_VERSION_COLUMN_TITLE.len(),
            ),
            |(name, version), pkgid| {
                (
                    name.max(pkgid.name().len()),
                    version.max(pkgid.version().to_string().len()),
                )
            },
        );

        let version = if human {
            version.min(TRUNCATED_VERSION_WIDTH)
        } else {
            version
        };
        let latest_trusted = if human { TRUNCATED_LATEST_T_WIDTH } else { 12 };

        Self {
            name,
            version,
            latest_trusted,
            human,
        }
    }
}

fn truncate_str(s: &str, max_width: usize) -> String {
    if s.chars().count() <= max_width {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_width - 1).collect();
        format!("{truncated}…")
    }
}

fn format_downloads(count: u64, human: bool) -> String {
    if !human {
        format!("{count:>9}")
    } else if count >= 1_000_000 {
        format!("{:>6}M", (count + 500_000) / 1_000_000)
    } else {
        format!("{:>6}K", count / 1000)
    }
}

pub fn print_header(
    _term: &mut Term,
    columns: &CrateVerifyColumns,
    column_widths: VerifyOutputColumnWidths,
) -> Result<()> {
    write!(io::stdout(), "{:>6} ", "status")?;

    if columns.show_reviews() {
        write!(io::stdout(), "{:>7} ", "reviews")?;
    }

    if columns.show_issues() {
        write!(io::stdout(), "{:>6} ", "issues")?;
    }

    if columns.show_owners() {
        write!(io::stdout(), "{:>5} ", "owner")?;
    }

    if columns.show_downloads() {
        let w = if column_widths.human { 15 } else { 19 };
        write!(io::stdout(), "{:>w$} ", "downloads")?;
    }

    if columns.show_loc() {
        write!(io::stdout(), "{:>6} ", "loc")?;
    }

    if columns.show_leftpad_index() {
        write!(io::stdout(), "{:>5} ", "lpidx")?;
    }

    if columns.show_geiger() {
        write!(io::stdout(), "{:>6} ", "geiger")?;
    }

    if columns.show_flags() {
        write!(io::stdout(), "{:>4} ", "flgs")?;
    }

    let name_column_width = column_widths.name;
    let version_column_width = column_widths.version;
    write!(
        io::stdout(),
        "{:<name_column_width$} {:<version_column_width$} ",
        "crate",
        "version"
    )?;

    if columns.show_latest_trusted() {
        let w = column_widths.latest_trusted;
        write!(io::stdout(), "{:<w$}", "latest_t")?;
    }

    if columns.show_digest() {
        write!(io::stdout(), "digest")?;
    }

    writeln!(io::stdout())?;
    Ok(())
}

#[allow(clippy::collapsible_if)]
pub fn write_details(
    cdep: &CrateDetails,
    term: &mut Term,
    columns: &CrateVerifyColumns,
    recursive_mode: bool,
    human: bool,
) -> Result<()> {
    if cdep.accumulative.is_local_source_code {
        term.print(format_args!("{:6} ", "local"), None)?;
    } else if !cdep.accumulative.has_trusted_ids
        && cdep.accumulative.trust == VerificationStatus::Insufficient
    {
        term.print(format_args!("{:6} ", "N/A"), None)?;
    } else {
        term.print(
            format_args!("{:6} ", cdep.accumulative.trust),
            term::verification_status_color(cdep.accumulative.trust),
        )?;
    }

    if columns.show_reviews() {
        write!(
            io::stdout(),
            "{:3} {:3} ",
            cdep.version_reviews.count,
            cdep.version_reviews.total
        )?;
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
        } else if let Some(known_owners) = &cdep.known_owners {
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

    if columns.show_downloads() {
        if let Some(downloads) = &cdep.downloads {
            term.print(
                format_args!("{} ", format_downloads(downloads.version, human)),
                if downloads.version < 2000 {
                    Some(::term::color::YELLOW)
                } else {
                    None
                },
            )?;
            term.print(
                format_args!("{} ", format_downloads(downloads.total, human)),
                if downloads.total < 20000 {
                    Some(::term::color::YELLOW)
                } else {
                    None
                },
            )?;
        } else {
            if human {
                term.print(format_args!("{:>8} {:>6} ", "?", "?"), None)?;
            } else {
                term.print(format_args!("{:>10} {:>8} ", "?", "?"), None)?;
            }
        }
    }

    if columns.show_loc() {
        match cdep.accumulative.loc {
            Some(loc) => write!(io::stdout(), "{loc:>6} ")?,
            None => write!(io::stdout(), "{:>6} ", "err")?,
        }
    }

    if columns.show_leftpad_index() {
        write!(
            io::stdout(),
            "{:>5} ",
            (cdep.leftpad_idx as f64).sqrt().round() as usize
        )?;
    }

    Ok(())
}

fn write_stats_crate_id(
    stats: &CrateStats,
    _term: &mut Term,
    column_widths: VerifyOutputColumnWidths,
) -> Result<()> {
    let name_column_width = column_widths.name;
    let version_column_width = column_widths.version;
    let version_str = stats.info.id.version().to_string()
        + if stats.info.id.source_id().is_registry() {
            ""
        } else {
            "*"
        };
    let version_str = if column_widths.human {
        truncate_str(&version_str, version_column_width)
    } else {
        version_str
    };
    write!(
        io::stdout(),
        "{:name_column_width$} {:<version_column_width$} ",
        stats.info.id.name(),
        version_str
    )?;
    Ok(())
}

pub fn print_dep(
    stats: &CrateStats,
    term: &mut Term,
    columns: &CrateVerifyColumns,
    recursive_mode: bool,
    column_widths: VerifyOutputColumnWidths,
) -> Result<()> {
    let details = stats.details();

    write_details(details, term, columns, recursive_mode, column_widths.human)?;
    if columns.show_geiger() {
        match details.accumulative.geiger_count {
            Some(geiger_count) => write!(io::stdout(), "{geiger_count:>6} ")?,
            None if cfg!(feature = "geiger") => write!(io::stdout(), "{:>6} ", "err")?,
            None => write!(io::stdout(), "{:>6} ", "dis")?,
        }
    }

    if columns.show_flags() {
        if stats.has_custom_build() {
            term.print(format_args!("CB"), ::term::color::YELLOW)?;
        } else {
            write!(io::stdout(), "__")?;
        }

        if stats.is_unmaintained() {
            write!(io::stdout(), "UM")?;
        } else {
            write!(io::stdout(), "__")?;
        }
        write!(io::stdout(), " ")?;
    }

    write_stats_crate_id(stats, term, column_widths)?;

    if columns.show_latest_trusted() {
        let w = column_widths.latest_trusted;
        let s =
            latest_trusted_version_string(stats.info.id.version(), &details.latest_trusted_version);
        let s = if column_widths.human {
            truncate_str(&s, w)
        } else {
            s
        };
        write!(io::stdout(), "{:<w$}", s)?;
    }

    if columns.show_digest() {
        write!(
            io::stdout(),
            "{}",
            details
                .digest
                .as_ref()
                .map_or_else(|| "-".into(), |d| d.to_string())
        )?;
    }

    writeln!(io::stdout())?;
    Ok(())
}
