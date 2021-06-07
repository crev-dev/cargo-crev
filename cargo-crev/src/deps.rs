use crev_data::{proof, Digest, PublicId, Version};
use crev_lib::*;
use crev_wot::TrustSet;
use std::path::PathBuf;

use crate::{opts::*, prelude::*, shared::*, term};
use cargo::core::PackageId;
use std::{
    collections::{HashMap, HashSet},
    ops::Add,
};

mod print_term;
pub mod scan;

#[derive(Copy, Clone, Debug)]
/// Progress-bar kind of thing, you know?
pub(crate) struct Progress {
    pub done: usize,
    pub total: usize,
}

#[derive(Copy, Clone, Debug)]
/// A count of something, plus the "total" number of that thing.
///
/// This is kind of context-dependent
pub struct CountWithTotal<T = u64> {
    pub count: T, // or "known" in case of crate owners
    pub total: T,
}

impl<T> Add<CountWithTotal<T>> for CountWithTotal<T>
where
    T: Add<T>,
{
    type Output = CountWithTotal<<T as Add>::Output>;

    fn add(self, other: CountWithTotal<T>) -> Self::Output {
        CountWithTotal {
            count: self.count + other.count,
            total: self.total + other.total,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct DownloadsStats {
    pub version: u64,
    pub total: u64,
    pub recent: u64,
}

impl Add<DownloadsStats> for DownloadsStats {
    type Output = DownloadsStats;

    fn add(self, other: DownloadsStats) -> Self::Output {
        DownloadsStats {
            version: self.version + other.version,
            total: self.total + other.total,
            recent: self.recent + other.recent,
        }
    }
}
/// A set of set of owners
#[derive(Clone, Debug)]
pub struct OwnerSetSet(HashMap<PackageId, HashSet<String>>);

impl OwnerSetSet {
    fn new(pkg_id: PackageId, set: impl IntoIterator<Item = String>) -> Self {
        let mut owner_set = HashMap::new();

        owner_set.insert(pkg_id, set.into_iter().collect());

        OwnerSetSet(owner_set)
    }

    pub fn to_total_owners(&self) -> usize {
        let all_owners: HashSet<_> = self.0.iter().flat_map(|(_pkg, set)| set).collect();

        all_owners.len()
    }

    pub fn to_total_distinct_groups(&self) -> usize {
        let mut count = 0;

        'outer: for (group_i, (_pkg, group)) in self.0.iter().enumerate() {
            for (other_group_i, (_pkg, other_group)) in self.0.iter().enumerate() {
                if group_i == other_group_i {
                    continue;
                }

                if group.iter().all(|member| other_group.contains(member)) {
                    // there is an `other_group` that is a super-set of this `group`
                    continue 'outer;
                }
            }
            // there was no other_group that would contain all members of this one
            count += 1;
        }

        count
    }
}

impl std::ops::Add<OwnerSetSet> for OwnerSetSet {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut set = self.0.clone();
        for (k, v) in other.0 {
            set.insert(k, v);
        }

        OwnerSetSet(set)
    }
}

/// Crate statistics - details that can be accumulated
/// by recursively including dependencies
#[derive(Clone, Debug)]
pub struct AccumulativeCrateDetails {
    pub trust: VerificationStatus,
    pub has_trusted_ids: bool,
    pub trusted_issues: CountWithTotal,
    pub verified: bool,
    pub loc: Option<u64>,
    pub geiger_count: Option<u64>,
    pub has_custom_build: bool,
    pub is_unmaintained: bool,
    pub owner_set: OwnerSetSet,
    pub is_local_source_code: bool,
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

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, other: Self) -> Self {
        Self {
            trust: self.trust.min(other.trust),
            has_trusted_ids: self.has_trusted_ids || other.has_trusted_ids,
            trusted_issues: self.trusted_issues + other.trusted_issues,
            verified: self.verified && other.verified,
            loc: sum_options(self.loc, other.loc),
            geiger_count: sum_options(self.geiger_count, other.geiger_count),
            has_custom_build: self.has_custom_build || other.has_custom_build,
            is_unmaintained: self.is_unmaintained || other.is_unmaintained,
            owner_set: self.owner_set + other.owner_set,
            is_local_source_code: self.is_local_source_code || other.is_local_source_code,
        }
    }
}

/// Crate statistics - details
#[derive(Clone, Debug)]
pub struct CrateDetails {
    pub digest: Option<Digest>,
    pub latest_trusted_version: Option<Version>,
    pub trusted_reviewers: HashSet<PublicId>,
    pub version_reviews: CountWithTotal,
    pub downloads: Option<DownloadsStats>,
    pub known_owners: Option<CountWithTotal>,
    pub leftpad_idx: u64,
    pub dependencies: Vec<proof::PackageVersionId>,
    pub rev_dependencies: Vec<proof::PackageVersionId>,
    pub unclean_digest: bool,
    // own accumulative stats only
    pub accumulative_own: AccumulativeCrateDetails,
    // total recursive stats
    pub accumulative_recursive: AccumulativeCrateDetails,
    // in recursive mode this is the same as `accumulative_recursive` otherwise `accumulative_own`
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

/// A dependency, as returned by the scanner
#[derive(Debug)]
pub struct CrateStats {
    pub info: CrateInfo,
    pub details: CrateDetails,
}

impl CrateStats {
    pub fn is_digest_unclean(&self) -> bool {
        self.details().unclean_digest
    }

    pub fn has_custom_build(&self) -> bool {
        self.details.accumulative.has_custom_build
    }

    pub fn is_unmaintained(&self) -> bool {
        self.details.accumulative.is_unmaintained
    }

    pub fn details(&self) -> &CrateDetails {
        &self.details
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

pub fn crate_mvps(crate_: CrateSelector, common: CrateVerifyCommon) -> Result<()> {
    let mut args = CrateVerify::default();
    args.common = common;

    let scanner = scan::Scanner::new(crate_, &args)?;
    let trust_set = scanner.trust_set.clone();
    let db = scanner.db.clone();
    let events = scanner.run();

    let mut mvps: HashMap<PublicId, u64> = HashMap::new();

    for stats in events {
        for reviewer in &stats.details.trusted_reviewers {
            *mvps.entry(reviewer.to_owned()).or_default() += 1;
        }
    }

    let mut mvps: Vec<_> = mvps.into_iter().collect();

    mvps.sort_by(|a, b| a.1.cmp(&b.1).reverse());

    crate::print_mvp_ids(
        mvps.iter().map(|(id, count)| (&id.id, *count)),
        &trust_set,
        &db,
    )?;

    Ok(())
}

pub fn verify_deps(crate_: CrateSelector, args: CrateVerify) -> Result<CommandExitStatus> {
    let mut term = term::Term::new();

    let scanner = scan::Scanner::new(crate_, &args)?;
    let trust_set = scanner.trust_set.clone();
    let events = scanner.run();

    // print header, only after `scanner` had a chance to download everything
    if term.stderr_is_tty && term.stdout_is_tty {
        print_term::print_header(&mut term, &args.columns);
    }

    let mut crates_with_issues = false;

    let deps: Vec<_> = events
        .into_iter()
        .filter(|stats| {
            !args.skip_known_owners
                || stats
                    .details
                    .known_owners
                    .map(|it| it.count == 0)
                    .unwrap_or(false)
        })
        .filter(|stats| !args.skip_verified || !stats.details.accumulative.verified)
        .map(|stats| {
            print_term::print_dep(&stats, &mut term, &args.columns, args.recursive)?;
            Ok(stats)
        })
        .collect::<Result<_>>()?;

    let mut nb_unclean_digests = 0;
    let mut nb_unverified = 0;
    for dep in &deps {
        let details = dep.details();
        if details.unclean_digest {
            nb_unclean_digests += 1;
        }
        if !details.accumulative.verified {
            nb_unverified += 1;
        }

        if details.accumulative_own.trusted_issues.count > 0 {
            crates_with_issues = true;
        }
    }

    if nb_unclean_digests > 0 {
        eprintln!(
            "{} unclean package{} detected. Use `cargo crev crate clean <name>` to wipe the local source.",
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

    if term.stderr_is_tty && term.stdout_is_tty {
        if !args.columns.any_selected() {
            eprintln!("Some columns were hidden. Use one or more `--show-<column>` to print more details. Use `--help` for list of available columns and other options and help. Use `--show-all` to just display everything.");
        }

        if crates_with_issues {
            eprintln!("Crates with issues found. Use `cargo crev repo query issue <crate> [<version>]` for details.");
        }

        write_out_distrusted_ids_details(&mut std::io::stderr(), &trust_set)?;
    }

    Ok(if nb_unverified == 0 {
        CommandExitStatus::Success
    } else {
        CommandExitStatus::VerificationFailed
    })
}

fn write_out_distrusted_ids_details(
    stderr: &mut impl std::io::Write,
    trust_set: &TrustSet,
) -> Result<()> {
    for (distrusted_id, details) in &trust_set.distrusted {
        for reported_by in &details.reported_by {
            write!(
                stderr,
                "Note: {} was ignored as distrusted by {}\n",
                distrusted_id, reported_by
            )?;
        }
    }
    Ok(())
}
