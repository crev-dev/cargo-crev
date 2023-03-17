use ::term::color::YELLOW;
use crev_data::{proof, review, Digest, PublicId, Version};
use crev_lib::*;
use crev_wot::TrustSet;
use std::{io, io::Write as _, path::PathBuf};

use crate::{opts::*, prelude::*, shared::*, term};
use cargo::core::PackageId;
use std::{
    collections::{HashMap, HashSet},
    ops::Add,
};

use self::scan::RequiredDetails;

mod print_term;
pub mod scan;

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
        let mut set = self.0;
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
    // Someone reported a different digest, our local copy is possibly wrong
    pub digest_mismatches: Vec<review::Package>,
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
    pub fn has_digest_mismatch(&self) -> bool {
        !self.details.digest_mismatches.is_empty()
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
                String::new()
            } else {
                latest_trusted_version.to_string()
            },
        )
    } else {
        String::new()
    }
}

pub fn crate_mvps(
    crate_: CrateSelector,
    common: CrateVerifyCommon,
    wot_opts: WotOpts,
) -> Result<()> {
    let args = CrateVerify {
        common,
        wot: wot_opts,
        ..Default::default()
    };
    let scanner = scan::Scanner::new(crate_, &args)?;
    let trust_set = scanner.trust_set.clone();
    let db = scanner.db.clone();
    let events = scanner.run(&RequiredDetails::none());

    let mut mvps: HashMap<PublicId, u64> = HashMap::new();

    for stats in events {
        for reviewer in &stats.details.trusted_reviewers {
            *mvps.entry(reviewer.clone()).or_default() += 1;
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
    let has_trusted_ids = scanner.has_trusted_ids;
    let column_widths =
        print_term::VerifyOutputColumnWidths::from_pkgsids(scanner.all_crates_ids.iter());

    let trust_set = scanner.trust_set.clone();

    let events = scanner.run(&RequiredDetails {
        geiger: args.columns.show_geiger(),
        owners: args.columns.show_owners() || args.skip_known_owners,
        downloads: args.columns.show_downloads() || args.columns.show_leftpad_index(),
        loc: args.columns.show_loc() || args.columns.show_leftpad_index(),
    });

    // print header, only after `scanner` had a chance to download everything
    if term.is_interactive() {
        print_term::print_header(&mut term, &args.columns, column_widths)?;
    }

    let mut crates_with_issues = false;

    let deps: Vec<_> = events
        .filter(|stats| !args.skip_known_owners || !crate_has_known_owner(stats))
        .filter(|stats| !args.skip_verified || !stats.details.accumulative.verified)
        .map(|stats| {
            print_term::print_dep(
                &stats,
                &mut term,
                &args.columns,
                args.recursive,
                column_widths,
            )?;
            Ok(stats)
        })
        .collect::<Result<_>>()?;

    let mut num_crates_with_digest_mismatch = 0;
    let mut nb_unverified = 0;
    for dep in &deps {
        let details = dep.details();
        if dep.has_digest_mismatch() {
            num_crates_with_digest_mismatch += 1;
        }
        if !details.accumulative.verified {
            nb_unverified += 1;
        }

        if details.accumulative_own.trusted_issues.count > 0 {
            crates_with_issues = true;
        }
    }

    if num_crates_with_digest_mismatch > 0 {
        eprintln!(
            "{} local crate{} with digest mismatch detected. Use `cargo crev crate clean [<name>]` to clean any potential unclean local copies. If problem persists, contact the reporter.",
            num_crates_with_digest_mismatch,
            if num_crates_with_digest_mismatch > 1 { "s" } else { "" },
        );
        let name_column_width = deps
            .iter()
            .filter(|dep| dep.has_digest_mismatch())
            .map(|dep| dep.info.id.name().len())
            .max()
            .expect("at least one crate should be present");

        let version_column_width = deps
            .iter()
            .filter(|dep| dep.has_digest_mismatch())
            .map(|dep| dep.info.id.version().to_string().len())
            .max()
            .expect("at least one crate should be present");
        for dep in deps {
            if dep.has_digest_mismatch() {
                for mismatch in &dep.details.digest_mismatches {
                    term.eprint(
                        format_args!(
                            "Crate {:<name_column_width$} {:<version_column_width$}; local digest: {} != {} reported by {} ({})\n",
                            &dep.info.id.name(),
                            &dep.info.id.version(),
                            &dep.details
                                .digest
                                .clone().map_or_else(|| "-".to_string(), |d| d.to_string()),
                            &Digest::from_bytes(&mismatch.package.digest).map_or_else(|| "-".to_string(), |d| d.to_string()),
                            &mismatch.common.from.id,
                            &mismatch.common.from.url_display(),
                        ),
                        ::term::color::RED,
                    )?;
                }
            }
        }
    }

    if term.is_interactive() {
        if !args.columns.any_selected() {
            eprintln!("Some columns were hidden. Use one or more `--show-<column>` to print more details. Use `--help` for list of available columns and other options and help. Use `--show-all` to just display everything.");
        }

        if crates_with_issues {
            eprintln!("Crates with issues found. Use `cargo crev repo query issue <crate> [<version>]` for details.");
        }

        write_out_distrusted_ids_details(&mut std::io::stderr(), &trust_set)?;

        if !has_trusted_ids {
            term.eprint(format_args!("NOTE: "), YELLOW)?;
            write!(io::stderr(), "No trusted Ids available. Nothing to verify against. Use `cargo crev trust` to add trusted reviewers or visit https://github.com/crev-dev/cargo-crev/discussions/ for help.")?;
        }
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
            writeln!(
                stderr,
                "Note: {distrusted_id} was ignored as distrusted by {reported_by}"
            )?;
        }
    }
    Ok(())
}

fn crate_has_known_owner(stats: &CrateStats) -> bool {
    match stats.details.known_owners {
        Some(known_owners) => known_owners.count > 0,
        None => false,
    }
}
