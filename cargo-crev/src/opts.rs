use common_failures::Result;
use crev_data::Level;
use failure::bail;
use semver::Version;
use std::{ffi::OsString, path::PathBuf};
use structopt::StructOpt;

#[derive(Debug, StructOpt, Clone, Default)]
pub struct CrateSelector {
    /// This crate is not neccesarily a dependency of the current cargo project
    #[structopt(long = "unrelated", short = "u")]
    pub unrelated: bool,

    pub name: Option<String>,
    pub version: Option<Version>,
}

impl CrateSelector {
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
    #[structopt(long = "target", value_name = "TARGET")]
    /// [cargo] Set the target triple
    pub target: Option<String>,
    #[structopt(long = "no-dev-dependencies")]
    /// [cargo] Skip dev dependencies.
    pub no_dev_dependencies: bool,
    #[structopt(long = "manifest-path", value_name = "PATH", parse(from_os_str))]
    /// [cargo] Path to Cargo.toml
    pub manifest_path: Option<PathBuf>,
    #[structopt(short = "Z", value_name = "FLAG")]
    /// [cargo] Unstable (nightly-only) flags to Cargo
    pub unstable_flags: Vec<String>,
}

#[derive(Debug, StructOpt, Clone)]
pub struct IdNew {
    #[structopt(long = "url")]
    /// URL of a git repository to be associated with the new Id
    pub url: Option<String>,
    #[structopt(long = "github-username")]
    /// Github username (instead of --url)
    pub github_username: Option<String>,
    #[structopt(long = "https-push")]
    /// Setup `https` instead of recommended `ssh`-based push url
    pub use_https_push: bool,
}

#[derive(Debug, StructOpt, Clone)]
pub struct IdSwitch {
    /// Own Id to switch to
    pub id: String,
}

/// Parameters describing trust graph traversal
#[derive(Debug, StructOpt, Clone, Default)]
pub struct TrustDistanceParams {
    #[structopt(long = "depth", default_value = "10")]
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
}

impl From<TrustDistanceParams> for crev_lib::TrustDistanceParams {
    fn from(params: TrustDistanceParams) -> Self {
        crev_lib::TrustDistanceParams {
            max_distance: params.depth,
            high_trust_distance: params.high_cost,
            medium_trust_distance: params.medium_cost,
            low_trust_distance: params.low_cost,
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
pub struct CrateVerifyCommon {
    #[structopt(flatten)]
    pub trust_params: TrustDistanceParams,

    #[structopt(flatten)]
    pub requirements: VerificationRequirements,

    #[structopt(long = "for-id")]
    /// Root identity to calculate the Web of Trust for [default: current user id]
    pub for_id: Option<String>,

    #[structopt(flatten)]
    pub cargo_opts: CargoOpts,

    #[structopt(flatten)]
    pub crate_: CrateSelector,
}

#[derive(Debug, StructOpt, Clone, Default)]
pub struct CrateVerify {
    #[structopt(flatten)]
    pub common: CrateVerifyCommon,

    #[structopt(long = "verbose", short = "v")]
    /// Display more informations about the crates
    pub verbose: bool,

    #[structopt(long = "interactive", short = "i")]
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
    /// Public IDs to create Trust Proof for
    pub pub_ids: Vec<String>,

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
    #[structopt(name = "trusted", alias = "t")]
    /// Fetch updates from trusted Ids
    Trusted(TrustDistanceParams),

    #[structopt(name = "url", alias = "u")]
    /// Fetch from a single public proof repository
    Url(RepoFetchUrl),

    #[structopt(name = "all", alias = "a")]
    /// Fetch all previously retrieved public proof repositories
    All,
}

#[derive(Debug, StructOpt, Clone)]
pub enum IdQuery {
    /// Show current Id
    #[structopt(name = "current", alias = "c")]
    Current {
        #[structopt(flatten)]
        trust_params: TrustDistanceParams,
    },

    /// Show all known Ids
    #[structopt(name = "all", alias = "a")]
    All {
        #[structopt(flatten)]
        trust_params: TrustDistanceParams,

        #[structopt(long = "for-id")]
        for_id: Option<String>,
    },

    /// Show own Ids
    #[structopt(name = "own", alias = "o")]
    Own {
        #[structopt(flatten)]
        trust_params: TrustDistanceParams,
    },

    /// List trusted ids
    #[structopt(name = "trusted", alias = "t")]
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
    pub common: ReviewOrGotoCommon,
}

#[derive(Debug, StructOpt, Clone)]
pub enum RepoQuery {
    /// Query reviews
    #[structopt(name = "review", alias = "r")]
    Review(RepoQueryReview),

    /// Query applicable advisories
    #[structopt(name = "advisory", alias = "a")]
    Advisory(RepoQueryAdvisory),

    /// Query applicable issues
    #[structopt(name = "issue", alias = "i")]
    Issue(RepoQueryIssue),
}

#[derive(Debug, StructOpt, Clone)]
pub enum RepoEdit {
    /// Edit your README.md file
    #[structopt(name = "readme", alias = "r")]
    Readme,

    /// Edit your KNOWN_CRATE_OWNERS.md file
    #[structopt(name = "known", alias = "k")]
    Known,
}

#[derive(Debug, StructOpt, Clone)]
pub struct RepoGit {
    /// Arguments to the `git` command
    #[structopt(parse(from_os_str))]
    pub args: Vec<OsString>,
}

#[derive(Debug, StructOpt, Clone)]
pub struct ReviewOrGotoCommon {
    #[structopt(flatten)]
    pub crate_: CrateSelector,
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
    pub common: ReviewOrGotoCommon,
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
    pub common: ReviewOrGotoCommon,

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

    #[structopt(long = "diff")]
    #[allow(clippy::option_option)]
    pub diff: Option<Option<semver::Version>>,

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
    #[structopt(name = "new", alias = "n")]
    New(IdNew),

    /// Export your own Id
    #[structopt(name = "export", alias = "e")]
    Export(IdExport),

    /// Import an Id as your own
    #[structopt(name = "import", alias = "i")]
    Import,

    /// Show your current Id
    #[structopt(name = "current", alias = "c")]
    Current,

    /// Change current Id
    #[structopt(name = "switch", alias = "s")]
    Switch(IdSwitch),

    /// Trust an Id
    #[structopt(name = "trust", alias = "t")]
    Trust(IdTrust),

    /// Untrust (remove) trust
    #[structopt(name = "untrust", alias = "u")]
    Untrust(IdTrust),

    /// Distrust an Id
    #[structopt(name = "distrust", alias = "d")]
    Distrust(IdTrust),

    /// Distrust an Id
    #[structopt(name = "query", alias = "q")]
    Query(IdQuery),
}

#[derive(Debug, StructOpt, Clone)]
pub enum Crate {
    /// Start a shell in source directory of a crate under review
    #[structopt(name = "goto", alias = "g")]
    Goto(ReviewOrGotoCommon),

    /// Open source code of a crate
    #[structopt(name = "open", alias = "o")]
    Open(CrateOpen),

    /// Clean a crate source code (eg. after review)
    #[structopt(name = "clean", alias = "c")]
    Clean(ReviewOrGotoCommon),

    /// Diff between two versions of a package
    #[structopt(name = "diff", alias = "d")]
    #[structopt(raw(setting = "structopt::clap::AppSettings::TrailingVarArg"))]
    #[structopt(raw(setting = "structopt::clap::AppSettings::AllowLeadingHyphen"))]
    Diff(Diff),

    /// Query source directory of a package
    #[structopt(name = "dir")]
    Dir(CrateDir),

    /// Verify dependencies
    #[structopt(
        name = "verify",
        alias = "v",
        after_help = r"This will show the following information:

Recursive mode will will calculate most metrics for the crate together with all its dependencies.

- trust      - Trust check result: `pass` for trusted, `none` for lacking reviews, `flagged` or `dangerous` for crates with problem reports.
- reviews    - Number of reviews for the specific version and for all available versions (total)
- downloads  - Download counts from crates.io for the specific version and all versions
- owner
  - In non-recursive mode: Owner counts from crates.io (known/all)
  - In recursive mode:
    - Total number of owners from crates.io
    - Total number of owner groups ignoring subsets
- issues     - Number of issues repored (from trusted sources/all)
- lines      - Lines of Rust code
- geiger     - Geiger score: number of `unsafe` lines
- flgs       - Flags for specific types of packages
  - CB         - Custom Build
- name       - Crate name
- version    - Crate version
- latest_t   - Latest trusted version"
    )]
    Verify(CrateVerify),

    /// Most valuable players (reviewers)
    #[structopt(name = "mvp", alias = "m")]
    Mvp(CrateVerifyCommon),

    /// Review a crate (code review, security advisory, flag issues)
    #[structopt(name = "review", alias = "r")]
    Review(CrateReview),

    /// Untrust (remove) trust
    #[structopt(name = "unreview", alias = "u")]
    Unreview(CrateReview),

    /// Search crates on crates.io sorting by review count
    #[structopt(name = "search", alias = "s")]
    Search(CrateSearch),
}

#[derive(Debug, StructOpt, Clone)]
pub enum Config {
    /// Edit the config file
    #[structopt(name = "edit", alias = "e")]
    Edit,
}

#[derive(Debug, StructOpt, Clone)]
/// Local Proof Repository
pub enum Repo {
    // TODO: `Dir`
    /// Publish to remote repository
    #[structopt(name = "publish", alias = "p")]
    Publish,

    /// Update data from online sources (proof repositories, crates.io)
    #[structopt(name = "update", alias = "pull")]
    Update(Update),

    /// Run raw git commands in the local proof repository
    #[structopt(name = "git", alias = "g")]
    #[structopt(raw(setting = "structopt::clap::AppSettings::TrailingVarArg"))]
    #[structopt(raw(setting = "structopt::clap::AppSettings::AllowLeadingHyphen"))]
    Git(RepoGit),

    /// Edit README.md of the current Id, ...
    #[structopt(name = "edit", alias = "e")]
    Edit(RepoEdit),

    /// Import proofs
    #[structopt(name = "import", alias = "i")]
    Import(RepoImport),

    /*
    /// Export proofs
    #[structopt(name = "export")]
    Export,
    */
    /// Query proofs
    #[structopt(name = "query", alias = "q")]
    Query(RepoQuery),

    /// Fetch proofs from external sources
    #[structopt(name = "fetch", alias = "f")]
    Fetch(RepoFetch),
}

#[derive(Debug, StructOpt, Clone)]
pub enum Command {
    /// Id (own and of other users)
    #[structopt(name = "id", alias = "i")]
    Id(Id),

    /// Crate related operations (review, verify...)
    #[structopt(name = "crate", alias = "c")]
    Crate(Crate),

    /// Proof Repository - store of proofs
    #[structopt(name = "repo", alias = "r")]
    Repo(Repo),

    /// Config
    #[structopt(name = "config", alias = "co")]
    Config(Config),
}

/// Cargo will pass the name of the `cargo-<tool>`
/// as first argument, so we just have to match it here.
#[derive(Debug, StructOpt, Clone)]
pub enum MainCommand {
    #[structopt(name = "crev")]
    Crev(Command),
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(about = "Distributed code review system")]
#[structopt(raw(global_setting = "structopt::clap::AppSettings::ColoredHelp"))]
pub struct Opts {
    #[structopt(subcommand)]
    pub command: MainCommand,
    //    #[structopt(flatten)]
    //    verbosity: Verbosity,
}
