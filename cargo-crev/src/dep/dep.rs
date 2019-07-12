use cargo::core::{
    package::{Package, PackageSet},
};
use semver::Version;

use crate::prelude::*;
use crate::term::{self, *};

use crev_data::*;
use crev_lib::*;

pub enum ComputationStatus {
    New,
    InProgress,
    Ok {
        dep: Dep,
    },
    Skipped, // imply it's verified and args ask for skipping of verified
    Failed,
}

pub struct CrateCounts {
    pub version: u64,
    pub total: u64,
}
pub struct TrustCount {
    pub trusted: usize, // or "known" in case of crate owners
    pub total: usize,
}

/// The computed content for a dep. One field should be one
///  cell in the displayed dep table
pub struct Dep {
    pub digest: Digest,
    pub name: String,
    pub version: Version, //VersionInfo,
    pub latest_trusted_version: Option<Version>,
    pub trust: VerificationStatus,
    pub reviews: CrateCounts,
    pub downloads: Option<CrateCounts>,
    pub owners: Option<TrustCount>,
    pub issues: TrustCount,
    pub loc: Option<usize>,
    pub geiger_count: Option<u64>,
    pub has_custom_build: bool,
    pub unclean_digest: bool,
    pub verified: bool,
}

impl Dep {
    pub fn term_print(&self, term: &mut Term, verbose: bool) -> Result<()> {
        if verbose {
            print!("{:43} ", self.digest);
        }
        term.print(
            format_args!("{:6}", self.trust),
            term::verification_status_color(&self.trust),
        )?;
        print!(" {:2} {:2}", self.reviews.version, self.reviews.total);
        if let Some(downloads) = &self.downloads {
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
        if let Some(owners) = &self.owners {
            term.print(
                format_args!(" {}", owners.trusted),
                term::known_owners_count_color(owners.trusted)
            )?;
            term.print(
                format_args!(" {}", owners.total),
                term::known_owners_count_color(owners.total)
            )?;
        } else {
            println!(" ???");
        }

        term.print(
            format_args!("{:4}", self.issues.trusted),
            if self.issues.trusted > 0 {
                Some(::term::color::RED)
            } else {
                None
            },
        )?;
        print!("/");
        term.print(
            format_args!("{:<2}", self.issues.total),
            if self.issues.total > 0 {
                Some(::term::color::YELLOW)
            } else {
                None
            },
        )?;
        match self.loc {
            Some(loc) => print!(" {:>6}", loc),
            None => print!(" {:>6}", "err"),
        }
        match self.geiger_count {
            Some(geiger_count) => print!(" {:>7}", geiger_count),
            None => print!(" {:>7}", "err"),
        }
        term.print(
            format_args!(" {:4}", if self.has_custom_build { "CB" } else { "" }),
            ::term::color::YELLOW,
        )?;
        print!(
            " {:<20} {:<15}",
            self.name,
            pad_left_manually(self.version.to_string(), 15)
        );

        print!(
            " {}",
            latest_trusted_version_string(
                &self.version,
                &self.latest_trusted_version
            )
        );
        Ok(())
    }

}

pub struct DepRow<'p> {
    pub pkg: &'p Package,
    //pub crate_id: PackageId, // contains the name, version
    pub computation_status: ComputationStatus,
    //pub dep: Option<Dep>,
}

impl<'p> DepRow<'p> {
    pub fn from(pkg: &'p Package) -> Self {
        DepRow {
            pkg,
            computation_status: ComputationStatus::New,
            //dep: None,
        }
    }

    pub fn is_digest_unclean(&self) -> bool {
        match &self.computation_status {
            ComputationStatus::Ok{dep} => dep.unclean_digest,
            _ => false,
        }
    }

    pub fn term_print_header(_term: &mut Term, verbose: bool) {
        if verbose {
            eprint!("{:43} ", "digest");
        }
        eprint!(
            "{:6} {:8} {:^15} {:4} {:6} {:6} {:6} {:4}",
            "status",
            "reviews",
            "downloads",
            "own.",
            "issues",
            "lines",
            "geiger",
            "flgs"
        );
        eprintln!(" {:<20} {:<15} {:<15}", "crate", "version", "latest_t");
    }

    pub fn term_print(&self, term: &mut Term, verbose: bool) -> Result<()> {
        match &self.computation_status {
            ComputationStatus::New => {
                println!("not yet computed");
            }
            ComputationStatus::InProgress => {
                println!("in progress...");
            }
            ComputationStatus::Failed => {
                println!("computation failed"); // TODO write the name
            }
            ComputationStatus::Skipped => {
                println!("skipped"); // TODO write the name
            }
            ComputationStatus::Ok{dep} => {
                dep.term_print(term, verbose)?;
                println!();
            }
        }
        Ok(())
    }
}


pub struct DepTable<'p> {
    pub rows: Vec<DepRow<'p>>,
}
impl<'p> DepTable<'p> {
    pub fn new(package_set: &'p PackageSet<'_>) -> Result<DepTable<'p>> {
        let pkgs = package_set.get_many(package_set.package_ids())?;
        let rows = pkgs.iter()
            .filter(|pkg| pkg.summary().source_id().is_registry())
            .map(|pkg| DepRow::from(pkg))
            .collect();
        //let rows = Vec::new();
        //for pkg in pkgs {
        //    if !pkg.summary().source_id().is_registry() {
        //        continue;
        //    }
        Ok(DepTable {
            rows,
        })
    }
}

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

fn latest_trusted_version_string(
    base_version: &Version,
    latest_trusted_version: &Option<Version>,
) -> String {
    if let Some(latest_trusted_version) = latest_trusted_version {
        format!(
            "{}{}",
            if base_version < latest_trusted_version {
                "↑"
            } else if latest_trusted_version < base_version {
                "↓"
            } else {
                "="
            },
            &latest_trusted_version,
        )
    } else {
        "".to_owned()

    }
}
