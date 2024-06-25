use anyhow::{bail, Result};
use crev_data::{Level, Version};
use std::{ffi::OsString, path::PathBuf};
use structopt::StructOpt;
use term::color;

use crate::term::Term;

#[derive(Debug, StructOpt, Clone, Default)]
pub struct CrateSelector {
    /// This crate is not neccesarily a dependency of the current cargo project
    #[structopt(long = "unrelated", short = "u")]
    pub unrelated: bool,

    #[structopt(long = "vers", short = "v")]
    version: Option<Version>,

    pub name: Option<String>,
    version_positional: Option<Version>,
}

impl CrateSelector {
    pub fn new(name: Option<String>, version: Option<Version>, unrelated: bool) -> Self {
        Self {
            unrelated,
            name,
            version,
            version_positional: None,
        }
    }

    pub fn version(&self) -> Result<Option<&Version>> {
        match (self.version_positional.as_ref(), self.version.as_ref()) {
            (Some(p), Some(np)) => bail!(
                "Can't use both positional (`{}`) and non-positional (`{}`) version argument",
                p,
                np
            ),
            (Some(p), None) => Ok(Some(p)),
            (None, Some(np)) => Ok(Some(np)),
            (None, None) => Ok(None),
        }
    }

    /// If can't find manifest file in CWD, change to `-u`
    ///
    /// This is so some commands can conveniently work outside of any
    /// Rust project.
    pub fn auto_unrelated(self) -> Result<Self> {
        let repo = crate::Repo::auto_open_cwd_default()?;

        Ok(match repo.get_manifest_path() {
            Ok(_) => self,
            Err(e) => {
                Term::new().eprintln(
                    format_args!("Unable to find manifest: {e}. Assuming `-u` option."),
                    Some(color::YELLOW),
                )?;
                CrateSelector {
                    unrelated: true,
                    ..self
                }
            }
        })
    }
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.version.is_none()
    }

    pub fn ensure_name_given(&self) -> Result<()> {
        if self.name.is_none() {
            bail!("Crate name argument required!")
        }

        Ok(())
    }
}

impl ReviewCrateSelector {
    pub fn auto_unrelated(self) -> Result<Self> {
        Ok(Self {
            diff: self.diff,
            crate_: self.crate_.auto_unrelated()?,
        })
    }
}

#[derive(Debug, StructOpt, Clone, Default)]
pub struct CargoOpts {
    #[structopt(long = "features", value_name = "FEATURES")]
    /// [cargo] Space-separated list of features to activate
    pub features: Option<String>,

    #[structopt(long = "all-features")]
    /// [cargo] Activate all available features
    pub all_features: bool,

    #[structopt(long = "no-default-features")]
    /// [cargo] Do not activate the `default` feature
    pub no_default_features: bool,

    #[structopt(long = "dev-dependencies")]
    /// [cargo] Activate dev dependencies.
    dev_dependencies: bool,

    #[structopt(long = "no-dev-dependencies")]
    /// [cargo] Skip dev dependencies.
    no_dev_dependencies: bool,

    #[structopt(long = "manifest-path", value_name = "PATH", parse(from_os_str))]
    /// [cargo] Path to Cargo.toml
    pub manifest_path: Option<PathBuf>,

    #[structopt(short = "Z", value_name = "FLAG")]

    /// [cargo] Unstable (nightly-only) flags to Cargo
    #[structopt(long = "unstable-flags")]
    pub unstable_flags: Vec<String>,

    /// [cargo] Skip targets other than specified (no value = autodetect)
    #[structopt(long = "target")]
    pub target: Option<Option<String>>,
}

impl CargoOpts {
    pub fn dev_dependencies(&self) -> Result<bool> {
        if self.dev_dependencies && self.no_dev_dependencies {
            bail!("`--no-dev-dependencies` and `--dev-dependencies` can't be used together");
        }

        Ok(self.dev_dependencies)
    }
}

#[derive(Debug, StructOpt, Clone)]
pub struct IdNew {
    #[structopt(long = "url")]
    /// Publicly-visible HTTPS URL of a git repository to be associated with the new Id
    pub url: Option<String>,
    #[structopt(long = "github-username")]
    /// Github username (instead of --url)
    pub github_username: Option<String>,
    #[structopt(long = "https-push")]
    /// Use public HTTP URL for both pulling and pushing. Otherwise SSH is used for push
    pub use_https_push: bool,
}

#[derive(Debug, StructOpt, Clone)]
pub struct IdSwitch {
    /// Id to switch to
    pub id: String,
}

/// Parameters describing trust graph traversal
#[derive(Debug, StructOpt, Clone, Default)]
pub struct TrustDistanceParams {
    /// [trust-graph-traversal] Consider only direct trust relationships
    #[structopt(long = "direct")]
    pub direct: bool,

    #[structopt(long = "depth", default_value = "20")]
    /// [trust-graph-traversal] Maximum allowed distance from the root identity when traversing trust graph
    pub depth: u64,

    /// [trust-graph-traversal] Cost of traversing trust graph edge of high trust level
    #[structopt(long = "high-cost", default_value = "0")]
    pub high_cost: u64,
    /// [trust-graph-traversal] Cost of traversing trust graph edge of medium trust level
    #[structopt(long = "medium-cost", default_value = "1")]
    pub medium_cost: u64,
    /// [trust-graph-traversal] Cost of traversing trust graph edge of low trust level
    #[structopt(long = "low-cost", default_value = "5")]
    pub low_cost: u64,
    /// [trust-graph-traversal] Cost of traversing trust graph edge of none trust level
    #[structopt(long = "none-cost", default_value = "21")]
    pub none_cost: u64,
    /// [trust-graph-traversal] Cost of traversing trust graph edge of distrust trust level
    #[structopt(long = "distrust-cost", default_value = "21")]
    pub distrust_cost: u64,
}

impl From<TrustDistanceParams> for crev_lib::TrustDistanceParams {
    fn from(params: TrustDistanceParams) -> Self {
        if params.direct {
            crev_lib::TrustDistanceParams {
                max_distance: 1,
                high_trust_distance: 1,
                medium_trust_distance: 1,
                low_trust_distance: 1,
                none_trust_distance: 1000,
                distrust_distance: 1000,
            }
        } else {
            crev_lib::TrustDistanceParams {
                max_distance: params.depth,
                high_trust_distance: params.high_cost,
                medium_trust_distance: params.medium_cost,
                low_trust_distance: params.low_cost,
                none_trust_distance: params.none_cost,
                distrust_distance: params.distrust_cost,
            }
        }
    }
}

#[derive(Debug, StructOpt, Clone)]
pub struct Diff {
    /// Source version - defaults to the last reviewed one
    #[structopt(long = "src")]
    pub src: Option<Version>,

    /// Destination version - defaults to the current one
    #[structopt(long = "dst")]
    pub dst: Option<Version>,

    /// This crate is not neccesarily a dependency of the current cargo project
    #[structopt(long = "unrelated", short = "u")]
    pub unrelated: bool,

    #[structopt(flatten)]
    pub requirements: VerificationRequirements,

    #[structopt(flatten)]
    pub trust_params: TrustDistanceParams,

    /// Crate name
    pub name: String,

    /// Arguments to the `diff` command
    #[structopt(parse(from_os_str))]
    pub args: Vec<OsString>,
}

#[derive(Debug, StructOpt, Clone, Default)]
pub struct TrustLevelRequirements {
    /// Minimum trust level required
    #[structopt(long = "trust", default_value = "low")]
    pub trust_level: crev_data::Level,
}

/// Verification Requirements
#[derive(Debug, StructOpt, Clone, Default)]
pub struct VerificationRequirements {
    #[structopt(flatten)]
    pub trust_level: TrustLevelRequirements,

    /// Number of reviews required
    #[structopt(long = "redundancy", default_value = "1")]
    pub redundancy: u64,
    /// Required understanding
    #[structopt(long = "understanding", default_value = "none")]
    pub understanding_level: Level,
    /// Required thoroughness
    #[structopt(long = "thoroughness", default_value = "none")]
    pub thoroughness_level: Level,
}

impl From<VerificationRequirements> for crev_lib::VerificationRequirements {
    fn from(req: VerificationRequirements) -> Self {
        crev_lib::VerificationRequirements {
            trust_level: req.trust_level.trust_level,
            redundancy: req.redundancy,
            understanding: req.understanding_level,
            thoroughness: req.thoroughness_level,
        }
    }
}

#[derive(Debug, StructOpt, Clone, Default)]
pub struct Update {
    #[structopt(flatten)]
    pub cargo_opts: CargoOpts,
}

#[derive(Debug, StructOpt, Clone, Default)]
pub struct WotOpts {
    #[structopt(flatten)]
    pub trust_params: TrustDistanceParams,

    #[structopt(long = "for-id")]
    /// Root identity to calculate the Web of Trust for [default: current user id]
    pub for_id: Option<String>,
}

#[derive(Debug, StructOpt, Clone, Default)]
pub struct CrateVerifyCommon {
    #[structopt(flatten)]
    pub requirements: VerificationRequirements,

    #[structopt(flatten)]
    pub cargo_opts: CargoOpts,
}

#[derive(Debug, StructOpt, Clone, Copy, Default)]
pub struct CrateVerifyColumns {
    #[structopt(long = "show-digest")]
    /// Show crate content digest
    pub show_digest: Option<Option<bool>>,

    #[structopt(long = "show-leftpad-index")]
    /// Show crate leftpad index (recent downloads / loc)
    pub show_leftpad_index: Option<Option<bool>>,

    #[structopt(long = "show-downloads")]
    /// Show crate download counts
    pub show_downloads: Option<Option<bool>>,

    #[structopt(long = "show-owners")]
    /// Show crate owners counts
    pub show_owners: Option<Option<bool>>,

    #[structopt(long = "show-latest-trusted")]
    /// Show latest trusted version
    pub show_latest_trusted: Option<Option<bool>>,

    #[structopt(long = "show-reviews")]
    /// Show reviews count
    pub show_reviews: Option<Option<bool>>,

    #[structopt(long = "show-loc")]
    /// Show Lines of Code
    pub show_loc: Option<Option<bool>>,

    #[structopt(long = "show-issues")]
    /// Show count of issues reported
    pub show_issues: Option<Option<bool>>,

    #[structopt(long = "show-geiger")]
    /// Show geiger (unsafe lines) count
    pub show_geiger: Option<Option<bool>>,

    #[structopt(long = "show-flags")]
    /// Show crate flags
    pub show_flags: Option<Option<bool>>,

    #[structopt(long = "show-all")]
    /// Show all
    pub show_all: bool,
}

macro_rules! show_x {
    ($name:ident, $default:expr) => {
        pub fn $name(self) -> bool {
            self.$name
                .unwrap_or(Some(self.show_all))
                .unwrap_or($default)
        }
    };
}

impl CrateVerifyColumns {
    pub fn any_selected(self) -> bool {
        self.show_digest.is_some()
            || self.show_leftpad_index.is_some()
            || self.show_downloads.is_some()
            || self.show_owners.is_some()
            || self.show_reviews.is_some()
            || self.show_latest_trusted.is_some()
            || self.show_flags.is_some()
            || self.show_issues.is_some()
            || self.show_loc.is_some()
            || self.show_geiger.is_some()
            || self.show_all
    }

    pub fn show_digest(self) -> bool {
        self.show_digest.flatten().unwrap_or(false)
    }

    show_x!(show_reviews, false);
    show_x!(show_leftpad_index, false);
    show_x!(show_downloads, false);
    show_x!(show_latest_trusted, true);
    show_x!(show_flags, true);
    show_x!(show_owners, false);
    show_x!(show_issues, true);
    show_x!(show_loc, false);
    show_x!(show_geiger, false);
}

#[derive(Debug, StructOpt, Clone, Default)]
#[structopt(
    after_help = r#"Recursive mode will calculate most metrics for the crate together with all its transitive dependencies.

Column description:

- status     - Trust check result: `pass` for trusted, `none` for lacking reviews, `flagged` or `dangerous` for crates with problem reports. `N/A` when crev is not configured yet.
- reviews    - Number of reviews for the specific version and for all available versions (total)
- issues     - Number of issues repored (from trusted sources/all)
- owner
  - In non-recursive mode: Owner counts from crates.io (known/all)
  - In recursive mode:
    - Total number of owners from crates.io
    - Total number of owner groups ignoring subsets
- downloads  - Download counts from crates.io for the specific version and all versions
- loc        - Lines of Rust code
- lpidx      - "left-pad" index (ratio of downloads to lines of code)
- geiger     - Geiger score: number of `unsafe` lines
- flgs       - Flags for specific types of packages
  - CB         - Custom Build (runs arbitrary code at build time)
  - UM         - Unmaintained crate
- name       - Crate name
- version    - Crate version
- latest_t   - Latest trusted version
"#
)]
pub struct CrateVerify {
    #[structopt(flatten)]
    pub common: CrateVerifyCommon,

    #[structopt(flatten)]
    pub wot: WotOpts,

    #[structopt(flatten)]
    pub columns: CrateVerifyColumns,

    #[structopt(long = "interactive", short = "i")]
    /// No-op
    pub interactive: bool,

    #[structopt(long = "skip-verified")]
    /// Display only crates not passing the verification
    pub skip_verified: bool,

    #[structopt(long = "skip-known-owners")]
    /// Skip crate from known owners (use `edit known` to edit the list)
    pub skip_known_owners: bool,

    #[structopt(long = "skip-indirect")]
    /// Skip dependencies that are not direct
    pub skip_indirect: bool,

    #[structopt(long = "recursive")]
    /// Calculate recursive metrics for your packages
    pub recursive: bool,
}

#[derive(Debug, StructOpt, Clone)]
pub struct IdTrust {
    #[structopt(long = "overrides")]
    /// Enable overrides suggestions
    pub overrides: bool,

    /// Public IDs to create Trust Proof for
    pub public_ids: Vec<String>,

    /// Shortcut for setting trust level without editing
    #[structopt(long = "level")]
    pub level: Option<crev_data::TrustLevel>,

    #[structopt(flatten)]
    pub common_proof_create: CommonProofCreate,
}

#[derive(Debug, StructOpt, Clone)]
pub struct TrustUrls {
    #[structopt(long = "overrides")]
    /// Enable overrides suggestions
    pub overrides: bool,

    /// Public IDs or proof repo URLs to create Trust Proof for
    pub public_ids_or_urls: Vec<String>,

    /// Shortcut for setting trust level without editing.
    /// Possible values are: "none" or "untrust", "low", "medium", "high" and "distrust".
    #[structopt(long = "level")]
    pub level: Option<crev_data::TrustLevel>,

    #[structopt(flatten)]
    pub common_proof_create: CommonProofCreate,
}

#[derive(Debug, StructOpt, Clone)]
pub struct RepoFetchUrl {
    /// URL to public proof repository
    pub url: String,
}

#[derive(Debug, StructOpt, Clone)]
pub enum RepoFetch {
    /// Fetch updates from trusted Ids
    #[structopt(name = "trusted")]
    Trusted {
        #[structopt(flatten)]
        distance_params: TrustDistanceParams,

        #[structopt(long = "for-id")]
        for_id: Option<String>,
    },

    #[structopt(name = "url")]
    /// Fetch from a single public proof repository
    Url(RepoFetchUrl),

    #[structopt(name = "all")]
    /// Fetch all previously retrieved public proof repositories
    All,
}

#[derive(Debug, StructOpt, Clone)]
pub enum IdQuery {
    /// Show current Id
    #[structopt(name = "current")]
    Current {
        #[structopt(flatten)]
        trust_params: TrustDistanceParams,
    },

    /// Show all known Ids
    #[structopt(name = "all")]
    All {
        #[structopt(flatten)]
        trust_params: TrustDistanceParams,

        #[structopt(long = "for-id")]
        for_id: Option<String>,
    },

    /// Show own Ids
    #[structopt(name = "own")]
    Own {
        #[structopt(flatten)]
        trust_params: TrustDistanceParams,
    },

    /// List trusted ids
    #[structopt(name = "trusted")]
    Trusted {
        #[structopt(flatten)]
        trust_params: TrustDistanceParams,

        #[structopt(long = "for-id")]
        for_id: Option<String>,

        #[structopt(flatten)]
        trust_level: TrustLevelRequirements,
    },
}

#[derive(Debug, StructOpt, Clone)]
pub struct RepoQueryReview {
    #[structopt(flatten)]
    pub crate_: CrateSelector,
}

#[derive(Debug, StructOpt, Clone)]
pub struct RepoQueryAdvisory {
    #[structopt(flatten)]
    pub crate_: CrateSelector,
}

#[derive(Debug, StructOpt, Clone)]
pub struct RepoQueryIssue {
    #[structopt(flatten)]
    pub crate_: CrateSelector,

    #[structopt(flatten)]
    pub trust_params: TrustDistanceParams,

    /// Minimum trust level of the reviewers for reviews
    #[structopt(long = "trust", default_value = "none")]
    pub trust_level: crev_data::Level,
}

#[derive(Debug, StructOpt, Clone)]
pub struct CrateDir {
    #[structopt(flatten)]
    pub common: ReviewCrateSelector,
}

#[derive(Debug, StructOpt, Clone)]
pub enum RepoQuery {
    /// Query reviews
    #[structopt(name = "review")]
    Review(RepoQueryReview),

    /// Query applicable advisories
    #[structopt(name = "advisory")]
    Advisory(RepoQueryAdvisory),

    /// Query applicable issues
    #[structopt(name = "issue")]
    Issue(RepoQueryIssue),
}

#[derive(Debug, StructOpt, Clone)]
pub enum RepoEdit {
    /// Edit your README.md file
    #[structopt(name = "readme")]
    Readme,

    /// Edit your KNOWN_CRATE_OWNERS.md file
    #[structopt(name = "known")]
    Known,
}

#[derive(Debug, StructOpt, Clone)]
pub struct RepoGit {
    /// Arguments to the `git` command
    #[structopt(parse(from_os_str))]
    pub args: Vec<OsString>,
}

#[derive(Debug, StructOpt, Clone)]
pub struct ReviewCrateSelector {
    #[structopt(flatten)]
    pub crate_: CrateSelector,

    /// Review the delta since the given version
    #[structopt(long = "diff", name = "base-version")]
    #[allow(clippy::option_option)]
    pub diff: Option<Option<Version>>,
}

#[derive(Debug, StructOpt, Clone)]
pub struct CrateOpen {
    /// Shell command to execute with crate directory as an argument. Eg. "code --wait -n" for VSCode
    #[structopt(long = "cmd")]
    pub cmd: Option<String>,

    /// Save the `--cmd` argument to be used a default in the future
    #[structopt(long = "cmd-save")]
    pub cmd_save: bool,

    #[structopt(flatten)]
    pub common: ReviewCrateSelector,
}

#[derive(Debug, StructOpt, Clone)]
pub struct CommonProofCreate {
    /// Don't auto-commit local Proof Repository
    #[structopt(long = "no-commit")]
    pub no_commit: bool,

    /// Print unsigned proof content on stdout
    #[structopt(long = "print-unsigned")]
    pub print_unsigned: bool,

    /// Print signed proof content on stdout
    #[structopt(long = "print-signed")]
    pub print_signed: bool,

    /// Don't store the proof
    #[structopt(long = "no-store")]
    pub no_store: bool,
}

#[derive(Debug, StructOpt, Clone)]
pub struct CrateReview {
    #[structopt(flatten)]
    pub common: ReviewCrateSelector,

    #[structopt(flatten)]
    pub common_proof_create: CommonProofCreate,

    /// Create advisory urging to upgrade to a safe version
    #[structopt(long = "advisory")]
    pub advisory: bool,

    /// This release contains advisory (important fix)
    #[structopt(long = "affected")]
    pub affected: Option<crev_data::proof::review::package::VersionRange>,

    /// Severity of bug/security issue [none low medium high]
    #[structopt(long = "severity")]
    pub severity: Option<Level>,

    /// Flag the crate as buggy/low-quality/dangerous
    #[structopt(long = "issue")]
    pub issue: bool,

    #[structopt(long = "skip-activity-check")]
    pub skip_activity_check: bool,

    #[structopt(long = "overrides")]
    /// Enable overrides suggestions
    pub overrides: bool,

    #[structopt(flatten)]
    pub cargo_opts: CargoOpts,
}

#[derive(Debug, Clone, Default)]
pub struct AdviseCommon {
    /// This release contains advisory (important fix)
    pub affected: crev_data::proof::review::package::VersionRange,
    pub severity: Level,
}

#[derive(Debug, StructOpt, Clone)]
pub struct CrateSearch {
    /// Number of results
    #[structopt(long = "count", default_value = "10")]
    pub count: usize,
    /// Query to use
    pub query: String,
}

#[derive(Debug, StructOpt, Clone)]
pub struct IdExport {
    pub id: Option<String>,
}

#[derive(Debug, StructOpt, Clone)]
pub struct IdSetUrl {
    #[structopt(long = "https-push")]
    /// Setup `https` instead of recommended `ssh`-based push url
    pub use_https_push: bool,

    /// Public read-only HTTPS git URL to use for the new crev-proofs repo
    pub url: String,
}

#[derive(Debug, StructOpt, Clone)]
pub struct RepoImport {
    /// Reset proof date to current date
    #[structopt(long = "reset-date")]
    pub reset_date: bool,

    #[structopt(flatten)]
    pub common: CommonProofCreate,
}

#[derive(Debug, StructOpt, Clone)]
pub enum Id {
    /// Create a new Id
    #[structopt(name = "new")]
    New(IdNew),

    /// Export your own Id
    #[structopt(name = "export")]
    Export(IdExport),

    /// Import an Id as your own
    #[structopt(name = "import")]
    Import,

    /// Show your current Id
    #[structopt(name = "current")]
    Current,

    /// Change current Id
    #[structopt(name = "switch")]
    Switch(IdSwitch),

    /// Change passphrase
    #[structopt(name = "passwd")]
    Passwd,

    /// Change public HTTPS repo URL for the current Id
    #[structopt(name = "set-url")]
    SetUrl(IdSetUrl),

    /// Trust an Id
    #[structopt(name = "trust")]
    Trust(IdTrust),

    /// Untrust (remove) trust
    #[structopt(name = "untrust")]
    Untrust(IdTrust),

    /// Distrust an Id
    #[structopt(name = "distrust")]
    Distrust(IdTrust),

    /// Query Ids
    #[structopt(name = "query")]
    Query(IdQuery),
}

#[derive(Debug, StructOpt, Clone)]
pub struct CrateVerifyFull {
    #[structopt(flatten)]
    pub opts: CrateVerify,

    #[structopt(flatten)]
    pub crate_: CrateSelector,
}

#[derive(Debug, StructOpt, Clone)]
pub enum Crate {
    /// Start a shell in source directory of a crate under review
    #[structopt(name = "goto")]
    Goto(CrateSelector),

    /// Open the source code of a crate
    #[structopt(name = "open")]
    Open(CrateOpen),

    /// WIP: Expand the crate source using `cargo-expand` like functionality
    // https://github.com/dtolnay/cargo-expand/issues/11
    #[structopt(name = "expand")]
    Expand(ReviewCrateSelector),

    /// Clean the source code directory of a crate (eg. after review)
    #[structopt(name = "clean")]
    Clean(CrateSelector),

    /// Diff between two versions of a package
    #[structopt(name = "diff")]
    #[structopt(setting = structopt::clap::AppSettings::TrailingVarArg)]
    #[structopt(setting = structopt::clap::AppSettings::AllowLeadingHyphen)]
    Diff(Diff),

    /// Display the path of the source code directory of a crate
    #[structopt(name = "dir")]
    Dir(CrateDir),

    /// Verify dependencies
    #[structopt(name = "verify")]
    Verify(CrateVerifyFull),

    /// Most valuable players (reviewers)
    #[structopt(name = "mvp")]
    Mvp {
        #[structopt(flatten)]
        opts: CrateVerifyCommon,

        #[structopt(flatten)]
        wot: WotOpts,

        #[structopt(flatten)]
        crate_: CrateSelector,
    },

    /// Review a crate (code review, security advisory, flag issues)
    #[structopt(name = "review")]
    Review(CrateReview),

    /// Unreview (overwrite with an null review)
    #[structopt(name = "unreview")]
    Unreview(CrateReview),

    /// Search crates on crates.io sorting by review count
    #[structopt(name = "search")]
    Search(CrateSearch),

    /// Display rich info about the given crate
    #[structopt(name = "info")]
    Info {
        #[structopt(flatten)]
        opts: CrateVerifyCommon,

        #[structopt(flatten)]
        wot: WotOpts,

        #[structopt(flatten)]
        crate_: CrateSelector,
    },
}

#[derive(Debug, StructOpt, Clone)]
pub enum Config {
    /// Edit the config file
    #[structopt(name = "edit")]
    Edit,

    /// Completions
    #[structopt(name = "completions")]
    Completions {
        #[structopt(long = "shell")]
        shell: Option<String>,
    },

    /// Print the dir containing config files
    #[structopt(name = "dir")]
    Dir,

    /// Print the dir containing data files
    #[structopt(name = "data-dir")]
    DataDir,

    /// Print the dir containing cache files
    #[structopt(name = "cache-dir")]
    CacheDir,
}

#[derive(Debug, StructOpt, Clone)]
pub struct ProofFind {
    #[structopt(name = "crate", long = "crate")]
    pub crate_: Option<String>,

    #[structopt(name = "vers", long = "vers")]
    pub version: Option<Version>,

    /// Find a proof by a crev Id
    #[structopt(name = "author", long = "author")]
    pub author: Option<String>,
}

#[derive(Debug, StructOpt, Clone)]
pub struct ProofReissue {
    #[structopt(name = "crate", long = "crate")]
    pub crate_: Option<String>,

    #[structopt(name = "vers", long = "vers")]
    pub version: Option<Version>,

    /// Reissue all proofs by a crev Id. Mandatory.
    #[structopt(name = "author", long = "author")]
    pub author: String,

    /// Comment for human readers. Mandatory.
    #[structopt(name = "comment", long = "comment")]
    pub comment: String,

    /// Skip check if we already reissued a review using the current id
    #[structopt(long = "skip-reissue-check")]
    pub skip_reissue_check: bool,

    #[structopt(flatten)]
    pub common_proof_create: CommonProofCreate,
}

#[derive(Debug, StructOpt, Clone)]
/// Local Proof Repository
pub enum Repo {
    /// Publish to remote repository
    #[structopt(name = "publish")]
    Publish,

    /// Update data from online sources (proof repositories, crates.io)
    #[structopt(name = "update")]
    Update(Update),

    /// Run raw git commands in the local proof repository
    #[structopt(name = "git")]
    #[structopt(setting = structopt::clap::AppSettings::TrailingVarArg)]
    #[structopt(setting = structopt::clap::AppSettings::AllowLeadingHyphen)]
    Git(RepoGit),

    /// Edit README.md of the current Id, ...
    #[structopt(name = "edit")]
    Edit(RepoEdit),

    /// Import proofs
    #[structopt(name = "import")]
    Import(RepoImport),

    /*
    /// Export proofs
    #[structopt(name = "export")]
    Export,
    */
    /// Query proofs
    #[structopt(name = "query")]
    Query(RepoQuery),

    /// Fetch proofs from external sources
    #[structopt(name = "fetch")]
    Fetch(RepoFetch),

    /// Print the dir containing local copy of the proof repository
    #[structopt(name = "dir")]
    Dir,
}

#[derive(Debug, StructOpt, Clone)]
/// Local Proof Repository
pub enum Proof {
    /// Find a proof
    #[structopt(name = "find")]
    Find(ProofFind),
    /// Reissue proofs with current id
    #[structopt(name = "reissue")]
    Reissue(ProofReissue),
}

#[derive(Debug, StructOpt, Clone)]
pub enum Wot {
    /// Create a new Id
    #[structopt(name = "log")]
    Log {
        #[structopt(flatten)]
        wot: WotOpts,
    },
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(setting = structopt::clap::AppSettings::DeriveDisplayOrder)]
#[structopt(setting = structopt::clap::AppSettings::DisableHelpSubcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Command {
    /// Local configuration
    #[structopt(name = "config")]
    Config(Config),

    /// Crate related operations (review, verify...)
    #[structopt(name = "crate")]
    Crate(Crate),

    /// Id (own and of other users)
    #[structopt(name = "id")]
    Id(Id),

    /// Find a proof in the proof repo
    #[structopt(name = "proof")]
    Proof(Proof),

    /// Proof Repository
    #[structopt(name = "repo")]
    Repo(Repo),

    /// Add a Trust proof by an Id or a URL
    Trust(TrustUrls),

    /// Web of Trust
    #[structopt(name = "wot")]
    Wot(Wot),

    /// Shortcut for `crate goto`
    #[structopt(name = "goto")]
    Goto(CrateSelector),

    /// Shortcut for `crate open`
    ///
    /// Crev will remember the last crate you've opened and default `crev review` to the same crate.
    #[structopt(name = "open")]
    Open(CrateOpen),

    /// Shortcut for `repo publish`
    #[structopt(name = "publish")]
    Publish,

    /// Shortcut for `crate review`
    ///
    /// You can omit crate name if you're called `open` recently.
    #[structopt(name = "review")]
    Review(CrateReview),

    /// Shortcut for `repo update`
    #[structopt(name = "update")]
    Update(Update),

    /// Shortcut for `crate verify`
    #[structopt(name = "verify")]
    Verify(CrateVerifyFull),
}

/// Cargo will pass the name of the `cargo-<tool>`
/// as first argument, so we just have to match it here.
#[derive(Debug, StructOpt, Clone)]
pub enum MainCommand {
    #[structopt(name = "crev")]
    #[structopt(after_help = r#"All commands can be abbreviated.

Help and feedback: https://github.com/crev-dev/cargo-crev/discussions/
User documentation: https://docs.rs/crate/cargo-crev
        "#)]
    Crev(Command),
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(about = "Distributed code review system")]
// without this the name will be `cargo-crev-crev` because the `crev` main command will be automatically appended
#[structopt(bin_name = "cargo")]
#[structopt(global_setting = structopt::clap::AppSettings::ColoredHelp)]
#[structopt(global_setting = structopt::clap::AppSettings::InferSubcommands)]
pub struct Opts {
    #[structopt(subcommand)]
    pub command: MainCommand,
    //    #[structopt(flatten)]
    //    verbosity: Verbosity,
}
