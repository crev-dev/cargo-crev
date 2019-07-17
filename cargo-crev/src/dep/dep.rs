use cargo::core::{
    package::{Package, PackageSet},
    package_id::PackageId,
};
use semver::Version;
use std::path::PathBuf;

use crate::prelude::*;
use crate::term::{self, *};
use crate::repo::*;
use crate::shared::*;

use crev_data::*;
use crev_lib::*;

#[derive(Clone, Copy, Debug)]
pub struct Progress {
    pub done: usize,
    pub total: usize,
}
impl Progress {
    pub fn is_complete(&self) -> bool {
        self.done >= self.total
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TableComputationStatus {
    New,
    ComputingGeiger {
        progress: Progress,
    },
    Done,
}
impl TableComputationStatus {

}

pub enum RowComputationStatus {
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
    pub version: Version,
    pub latest_trusted_version: Option<Version>,
    pub trust: VerificationStatus,
    pub reviews: CrateCounts,
    pub downloads: Option<CrateCounts>,
    pub owners: Option<TrustCount>,
    pub issues: TrustCount,
    pub loc: Option<usize>,
    pub has_custom_build: bool, // duplicate data, improve that
    pub unclean_digest: bool,
    pub verified: bool,
}

pub struct DepRow {
    pub id: PackageId, // contains the name, version
    pub root: PathBuf,
    pub has_custom_build: bool,
    pub geiger_count: Option<u64>,
    pub computation_status: RowComputationStatus,
}

impl Dep {
    pub fn term_print(
        &self,
        term: &mut Term,
        geiger_count: Option<u64>,
        verbose: bool,
    ) -> Result<()> {
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
        match geiger_count {
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


impl DepRow {
    pub fn from(pkg: & Package) -> Self {
        let id = pkg.package_id();
        let root = pkg.root().to_path_buf();
        let has_custom_build = pkg.has_custom_build();
        DepRow {
            id,
            root,
            has_custom_build,
            geiger_count: None,
            computation_status: RowComputationStatus::New,
        }
    }

    pub fn download_if_needed(
        &mut self,
    ) -> Result<()> {
        if !self.root.exists() {
            let repo = Repo::auto_open_cwd()?;
            let mut source = repo.load_source()?;
            source.download(self.id)?;
        }
        Ok(())
    }

    pub fn count_geiger(
        &mut self,
    ) {
        debug_assert!(self.root.exists());
        self.geiger_count = get_geiger_count(&self.root).ok();
    }

    pub fn is_digest_unclean(&self) -> bool {
        match &self.computation_status {
            RowComputationStatus::Ok{dep} => dep.unclean_digest,
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
            RowComputationStatus::New => {
                println!("not yet computed");
            }
            RowComputationStatus::InProgress => {
                println!("in progress...");
            }
            RowComputationStatus::Failed => {
                println!("computation failed"); // TODO write the name
            }
            RowComputationStatus::Skipped => {
                println!("skipped"); // TODO write the name
            }
            RowComputationStatus::Ok{dep} => {
                dep.term_print(term, self.geiger_count, verbose)?;
                println!();
            }
        }
        Ok(())
    }
}


pub struct DepTable {
    pub rows: Vec<DepRow>,
    pub computation_status: TableComputationStatus,
}
impl DepTable {
    pub fn new(package_set: &PackageSet<'_>) -> Result<DepTable> {
        let pkgs = package_set.get_many(package_set.package_ids())?;
        let rows = pkgs.iter()
            .filter(|pkg| pkg.summary().source_id().is_registry())
            .map(|pkg| DepRow::from(pkg))
            .collect();
        Ok(DepTable {
            rows,
            computation_status: TableComputationStatus::New,
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
