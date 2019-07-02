use crev_data::Level;
use semver::Version;
use std::ffi::OsString;
use structopt::StructOpt;

#[derive(Debug, StructOpt, Clone)]
pub struct CrateSelector {
    pub name: Option<String>,
    pub version: Option<Version>,
}

#[derive(Debug, StructOpt, Clone)]
pub struct NewId {
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
pub enum New {
    #[structopt(name = "id")]
    /// Generate a CrevID
    Id(NewId),
}

#[derive(Debug, StructOpt, Clone)]
pub struct SwitchId {
    /// Own Id to switch to
    pub id: String,
}

/// Parameters describing trust graph traversal
#[derive(Debug, StructOpt, Clone)]
pub struct TrustDistanceParams {
    #[structopt(long = "depth", default_value = "10")]
    pub depth: u64,
    #[structopt(long = "high-cost", default_value = "0")]
    pub high_cost: u64,
    #[structopt(long = "medium-cost", default_value = "1")]
    pub medium_cost: u64,
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

/// Verification Requirements
#[derive(Debug, StructOpt, Clone)]
pub struct VerificationRequirements {
    /// Minimum trust level of the reviewers for reviews
    #[structopt(long = "trust", default_value = "low")]
    pub trust_level: crev_data::Level,

    /// Number of reviews required
    #[structopt(long = "redundancy", default_value = "1")]
    pub redundancy: u64,
    /// Required understanding
    #[structopt(long = "understanding", default_value = "none")]
    pub understanding: Level,
    /// Required thoroughness
    #[structopt(long = "thoroughness", default_value = "none")]
    pub thoroughness: Level,
}

impl From<VerificationRequirements> for crev_lib::VerificationRequirements {
    fn from(req: VerificationRequirements) -> Self {
        crev_lib::VerificationRequirements {
            trust_level: req.trust_level,
            redundancy: req.redundancy,
            understanding: req.understanding,
            thoroughness: req.thoroughness,
        }
    }
}

#[derive(Debug, StructOpt, Clone)]
pub struct VerifyDeps {
    #[structopt(long = "verbose", short = "v")]
    pub verbose: bool,

    #[structopt(flatten)]
    pub trust_params: TrustDistanceParams,

    #[structopt(flatten)]
    pub requirements: VerificationRequirements,

    #[structopt(long = "skip-verified")]
    pub skip_verified: bool,

    #[structopt(long = "skip-known-owners")]
    pub skip_known_owners: bool,

    #[structopt(long = "for-id")]
    pub for_id: Option<String>,
}

#[derive(Debug, StructOpt, Clone)]
pub enum Verify {
    /// Verify dependencies
    #[structopt(
        name = "deps",
        after_help = r"This will show the following information:

- trust      - Trust check result: `pass` for trusted, `none` for lacking reviews, `flagged` or `dangerous` for crates with problem reports.
- reviews    - Number of reviews for the specific version and for all available versions (total)
- downloads  - Download counts from crates.io for the specific version and all versions
- own.       - Owner counts from crates.io (known/all)
- advisr.    - Number of aplicable advisories (important upgrades) repored (trusted sources/all)
- lines      - Lines of Rust code
- geiger     - Geiger score: number of `unsafe` lines
- flgs       - Flags for specific types of packages
  - CB         - Custom Build
- name       - Crate name
- version    - Crate version
- latest_t   - Latest trusted version"
    )]
    Deps(VerifyDeps),
}

#[derive(Debug, StructOpt, Clone)]
pub struct Trust {
    /// Public IDs to create Trust Proof for
    pub pub_ids: Vec<String>,

    #[structopt(flatten)]
    pub common_proof_create: CommonProofCreate,
}

#[derive(Debug, StructOpt, Clone)]
pub struct FetchUrl {
    /// URL to public proof repository
    pub url: String,
}

#[derive(Debug, StructOpt, Clone)]
pub enum Fetch {
    #[structopt(name = "trusted")]
    /// Fetch updates from trusted Ids
    Trusted(TrustDistanceParams),

    #[structopt(name = "url")]
    /// Fetch from a single public proof repository
    Url(FetchUrl),

    #[structopt(name = "all")]
    /// Fetch all previously retrieved public proof repositories
    All,
}

#[derive(Debug, StructOpt, Clone)]
pub enum QueryId {
    /// Show current Id
    #[structopt(name = "current")]
    Current,

    /// Show all known Ids
    #[structopt(name = "all")]
    All,

    /// Show own Ids
    #[structopt(name = "own")]
    Own,

    /// List trusted ids
    #[structopt(name = "trusted")]
    Trusted {
        #[structopt(flatten)]
        trust_params: TrustDistanceParams,

        #[structopt(long = "for-id")]
        for_id: Option<String>,
    },
}

#[derive(Debug, StructOpt, Clone)]
pub struct QueryReview {
    #[structopt(flatten)]
    pub crate_: CrateSelector,
}

#[derive(Debug, StructOpt, Clone)]
pub struct QueryAdvisory {
    #[structopt(flatten)]
    pub crate_: CrateSelector,
}

#[derive(Debug, StructOpt, Clone)]
pub struct QueryDir {
    #[structopt(flatten)]
    pub common: ReviewOrGotoCommon,
}

#[derive(Debug, StructOpt, Clone)]
pub enum Query {
    /// Query Ids
    #[structopt(name = "id")]
    Id(QueryId),

    /// Query reviews
    #[structopt(name = "review")]
    Review(QueryReview),

    /// Query applicable advisories
    #[structopt(name = "advisory")]
    Advisory(QueryAdvisory),

    /// Query source directory of a package
    #[structopt(name = "dir")]
    Dir(QueryDir),
}

#[derive(Debug, StructOpt, Clone)]
pub enum Switch {
    /// Change current Id
    #[structopt(name = "id")]
    Id(SwitchId),
}

#[derive(Debug, StructOpt, Clone)]
pub enum Edit {
    /// Edit your README.md file
    #[structopt(name = "readme")]
    Readme,

    /// Edit your user config
    #[structopt(name = "config")]
    Config,

    /// Edit your KNOWN_CRATE_OWNERS.md file
    #[structopt(name = "known")]
    Known,
}

#[derive(Debug, StructOpt, Clone)]
pub struct Git {
    /// Arguments to the `git` command
    #[structopt(parse(from_os_str))]
    pub args: Vec<OsString>,
}

#[derive(Debug, StructOpt, Clone)]
pub struct ReviewOrGotoCommon {
    #[structopt(flatten)]
    pub crate_: CrateSelector,

    /// This crate is not neccesarily a dependency of the current cargo project
    #[structopt(long = "unrelated", short = "u")]
    pub unrelated: bool,
}

#[derive(Debug, StructOpt, Clone)]
pub struct Open {
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
pub struct Review {
    #[structopt(flatten)]
    pub common: ReviewOrGotoCommon,

    #[structopt(flatten)]
    pub common_proof_create: CommonProofCreate,

    #[structopt(long = "advisory")]
    pub advisory: bool,

    #[structopt(long = "issue")]
    pub issue: bool,

    #[structopt(long = "skip-activity-check")]
    pub skip_activity_check: bool,

    #[structopt(long = "diff")]
    pub diff: Option<Option<semver::Version>>,
}

#[derive(Debug, StructOpt, Clone, Default)]
pub struct AdviseCommon {
    /// This release contains advisory (important fix)
    #[structopt(
        long = "affected",
        // default_value = "proof::review::package::AdvisoryRange::Major"
        default_value = "major"
    )]
    pub affected: crev_data::proof::review::package::AdvisoryRange,

    #[structopt(long = "severity")]
    pub severity: Level,
}

#[derive(Debug, StructOpt, Clone)]
pub struct Advise {
    #[structopt(flatten)]
    pub common: ReviewOrGotoCommon,

    #[structopt(flatten)]
    pub common_proof_create: CommonProofCreate,

    #[structopt(flatten)]
    pub advise_common: AdviseCommon,
}

#[derive(Debug, StructOpt, Clone, Default)]
pub struct ReportCommon {
    #[structopt(long = "severity")]
    pub severity: Level,
}

#[derive(Debug, StructOpt, Clone)]
pub struct Report {
    #[structopt(flatten)]
    pub common: ReviewOrGotoCommon,

    #[structopt(flatten)]
    pub common_proof_create: CommonProofCreate,

    #[structopt(flatten)]
    pub report_common: ReportCommon,
}

#[derive(Debug, StructOpt, Clone)]
pub struct ExportId {
    pub id: Option<String>,
}

#[derive(Debug, StructOpt, Clone)]
pub enum Export {
    #[structopt(name = "id")]
    Id(ExportId),
}

#[derive(Debug, StructOpt, Clone)]
pub enum Import {
    /// Import an Id
    #[structopt(name = "id")]
    Id,

    /// Import proofs: resign proofs using current id
    ///
    /// Useful for mass-import of proofs signed by another ID
    #[structopt(name = "proof")]
    Proof(ImportProof),
}

#[derive(Debug, StructOpt, Clone)]
pub struct ImportProof {
    /// Reset proof date to current date
    #[structopt(long = "reset-date")]
    pub reset_date: bool,

    #[structopt(flatten)]
    pub common: CommonProofCreate,
}

#[derive(Debug, StructOpt, Clone)]
pub enum Command {
    /// Create an Id, ...
    #[structopt(name = "new")]
    New(New),

    /// Switch current Id, ...
    #[structopt(name = "switch")]
    Switch(Switch),

    /// Edit README.md of the current Id, ...
    #[structopt(name = "edit")]
    Edit(Edit),

    /// Verify dependencies
    #[structopt(name = "verify")]
    Verify(Verify),

    /// Review a crate
    #[structopt(name = "review")]
    Review(Review),

    /// Flag a crate as buggy/low-quality/dangerous
    #[structopt(name = "flag")]
    Flag(Review),

    /// Report that the crate is affected by a certain issue
    #[structopt(name = "report")]
    Report(Report),

    /// Create advisory urging to upgrade to given package version
    #[structopt(name = "advise")]
    Advise(Advise),

    /// Query Ids, packages, reviews...
    #[structopt(name = "query")]
    Query(Query),

    /// Trust an Id
    #[structopt(name = "trust")]
    Trust(Trust),

    /// Distrust an Id
    #[structopt(name = "distrust")]
    Distrust(Trust),

    /// Fetch proofs from external sources
    #[structopt(name = "fetch")]
    Fetch(Fetch),

    /// Run raw git commands in the local proof repository
    #[structopt(name = "git")]
    #[structopt(raw(setting = "structopt::clap::AppSettings::TrailingVarArg"))]
    #[structopt(raw(setting = "structopt::clap::AppSettings::AllowLeadingHyphen"))]
    Git(Git),

    /// Push local changes to the public proof repository (alias to `git push HEAD`)
    #[structopt(name = "push")]
    Push,

    /// Commit and Push local changes to the public proof repository (alias to `git commit -a && git push HEAD`)
    #[structopt(name = "publish")]
    Publish,

    /// Pull changes from the public proof repository (alias to `git pull`)
    #[structopt(name = "pull")]
    Pull,

    /// Start a shell in source directory of a crate under review
    #[structopt(name = "goto")]
    Goto(ReviewOrGotoCommon),

    /// Open source code of a crate
    #[structopt(name = "open")]
    Open(Open),

    /// Clean a crate source code (eg. after review)
    #[structopt(name = "clean")]
    Clean(ReviewOrGotoCommon),

    /// Export an id, ...
    #[structopt(name = "export")]
    Export(Export),

    /// Import an Id, ...
    #[structopt(name = "import")]
    Import(Import),

    /// Update data from online sources (crates.io)
    #[structopt(name = "update")]
    Update,

    /// Diff between two versions of a package
    #[structopt(name = "diff")]
    #[structopt(raw(setting = "structopt::clap::AppSettings::TrailingVarArg"))]
    #[structopt(raw(setting = "structopt::clap::AppSettings::AllowLeadingHyphen"))]
    Diff(Diff),
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
pub struct Opts {
    #[structopt(subcommand)]
    pub command: MainCommand,
    //    #[structopt(flatten)]
    //    verbosity: Verbosity,
}
