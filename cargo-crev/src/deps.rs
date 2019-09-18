use semver::Version;
use std::path::PathBuf;

use crev_data::*;
use crev_lib::*;

use crate::opts::*;
use crate::prelude::*;
use crate::shared::*;
use crate::term;
use std::ops::Add;

mod print_term;
pub mod scan;

#[derive(Copy, Clone, Debug)]
pub struct Progress {
    pub done: usize,
    pub total: usize,
}

impl Progress {
    pub fn is_valid(self) -> bool {
        self.done <= self.total
    }

    pub fn is_complete(self) -> bool {
        self.done >= self.total
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ReviewCount {
    pub version: u64,
    pub total: u64,
}

#[derive(Copy, Clone, Debug)]
pub struct DownloadCount {
    pub version: u64,
    pub total: u64,
}

#[derive(Copy, Clone, Debug)]
pub struct TrustCount {
    pub trusted: usize, // or "known" in case of crate owners
    pub total: usize,
}

impl Add<TrustCount> for TrustCount {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            trusted: self.trusted + other.trusted,
            total: self.total + other.total,
        }
    }
}

/// Crate statistics - details that can be accumulated
/// by recursively including dependencies
#[derive(Clone, Debug, Copy)]
pub struct AccumulativeCrateDetails {
    pub trust: VerificationStatus,
    pub issues: TrustCount,
    pub verified: bool,
    pub loc: Option<usize>,
    pub geiger_count: Option<u64>,
}

fn sum_options<T>(a: Option<T>, b: Option<T>) -> Option<T::Output>
where
    T: Add<T>,
{
    match (a, b) {
        (Some(a), Some(b)) => Some(a + b),
        _ => None,
    }
}

impl std::ops::Add<AccumulativeCrateDetails> for AccumulativeCrateDetails {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            trust: self.trust.min(other.trust),
            issues: self.issues + other.issues,
            verified: self.verified && other.verified,
            loc: sum_options(self.loc, other.loc),
            geiger_count: sum_options(self.geiger_count, other.geiger_count),
        }
    }
}

/// Crate statistics - details
#[derive(Clone, Debug)]
pub struct CrateDetails {
    pub digest: Digest,
    pub latest_trusted_version: Option<Version>,
    pub reviews: ReviewCount,
    pub downloads: Option<DownloadCount>,
    pub owners: Option<TrustCount>,
    pub unclean_digest: bool,
    pub accumulative_single: AccumulativeCrateDetails,
    pub accumulative: AccumulativeCrateDetails,
}

/// Basic crate info of a crate we're scanning
#[derive(Clone, Debug)]
pub struct CrateInfo {
    pub id: cargo::core::PackageId, // contains the name, version
    pub root: PathBuf,
    pub has_custom_build: bool,
}

impl CrateInfo {
    pub fn from_pkg(pkg: &cargo::core::Package) -> Self {
        let id = pkg.package_id();
        let root = pkg.root().to_path_buf();
        let has_custom_build = pkg.has_custom_build();
        CrateInfo {
            id,
            root,
            has_custom_build,
        }
    }

    pub fn download_if_needed(&self, cargo_opts: CargoOpts) -> Result<()> {
        if !self.root.exists() {
            let repo = crate::Repo::auto_open_cwd(cargo_opts)?;
            let mut source = repo.load_source()?;
            source.download(self.id)?;
        }
        Ok(())
    }
}

impl PartialOrd for CrateInfo {
    fn partial_cmp(&self, other: &CrateInfo) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl Ord for CrateInfo {
    fn cmp(&self, other: &CrateInfo) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialEq for CrateInfo {
    fn eq(&self, other: &CrateInfo) -> bool {
        self.id == other.id
    }
}

impl Eq for CrateInfo {}

/// A dependency, as returned by the computer. It may
///  contain (depending on success/slipping) the computed
///  dep.
pub struct CrateStats {
    pub info: CrateInfo,
    pub details: Result<Option<CrateDetails>>,
}

impl CrateStats {
    pub fn is_digest_unclean(&self) -> bool {
        self.details().map_or(false, |d| d.unclean_digest)
    }

    pub fn has_details(&self) -> bool {
        self.details().is_some()
    }

    pub fn has_custom_build(&self) -> bool {
        self.info.has_custom_build
    }

    pub fn details(&self) -> Option<&CrateDetails> {
        if let Ok(Some(ref details)) = self.details {
            Some(details)
        } else {
            None
        }
    }
}

pub fn latest_trusted_version_string(
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
            if base_version == latest_trusted_version {
                "".into()
            } else {
                latest_trusted_version.to_string()
            },
        )
    } else {
        "".to_owned()
    }
}

pub fn verify_deps(args: Verify) -> Result<CommandExitStatus> {
    let mut term = term::Term::new();
    if term.stderr_is_tty && term.stdout_is_tty {
        self::print_term::print_header(&mut term, args.verbose);
    }

    let scanner = scan::Scanner::new(&args)?;
    let events = scanner.run();

    let deps: Vec<_> = events
        .into_iter()
        .map(|stats| {
            print_term::print_dep(&stats, &mut term, args.verbose)?;
            Ok(stats)
        })
        .collect::<Result<_>>()?;

    let mut nb_unclean_digests = 0;
    let mut nb_unverified = 0;
    for dep in &deps {
        if dep.is_digest_unclean() {
            let details = dep.details().unwrap();
            if details.unclean_digest {
                nb_unclean_digests += 1;
            }
            if !details.accumulative.verified {
                nb_unverified += 1;
            }
        }
    }

    if nb_unclean_digests > 0 {
        println!(
            "{} unclean package{} detected. Use `cargo crev clean <crate>` to wipe the local source.",
            nb_unclean_digests,
            if nb_unclean_digests > 1 { "s" } else { "" },
        );
        for dep in deps {
            if dep.is_digest_unclean() {
                term.eprint(
                    format_args!(
                        "Unclean crate {} {}\n",
                        &dep.info.id.name(),
                        &dep.info.id.version()
                    ),
                    ::term::color::RED,
                )?;
            }
        }
    }

    Ok(if nb_unverified == 0 {
        CommandExitStatus::Success
    } else {
        CommandExitStatus::VerificationFailed
    })
}
