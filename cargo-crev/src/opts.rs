use std::ffi::OsString;

#[derive(Debug, StructOpt, Clone)]
pub struct CrateSelector {
    pub name: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, StructOpt, Clone)]
pub struct CrateSelectorNameRequired {
    pub name: String,
    pub version: Option<String>,
}

impl From<CrateSelectorNameRequired> for CrateSelector {
    fn from(c: CrateSelectorNameRequired) -> Self {
        Self {
            name: Some(c.name),
            version: c.version,
        }
    }
}

#[derive(Debug, StructOpt, Clone)]
pub enum New {
    #[structopt(name = "id")]
    /// Generate a CrevID
    Id,
}

#[derive(Debug, StructOpt, Clone)]
pub struct ChangeId {
    /// Own Id to switch to
    pub id: String,
}

/// Parameters describing trust graph traversal
#[derive(Debug, StructOpt, Clone)]
pub struct TrustParams {
    #[structopt(long = "depth", default_value = "10")]
    pub depth: u64,
    #[structopt(long = "high-cost", default_value = "0")]
    pub high_cost: u64,
    #[structopt(long = "medium-cost", default_value = "1")]
    pub medium_cost: u64,
    #[structopt(long = "low-cost", default_value = "5")]
    pub low_cost: u64,
}

impl From<TrustParams> for crev_lib::trustdb::TrustDistanceParams {
    fn from(params: TrustParams) -> Self {
        crev_lib::trustdb::TrustDistanceParams {
            max_distance: params.depth,
            high_trust_distance: params.high_cost,
            medium_trust_distance: params.medium_cost,
            low_trust_distance: params.low_cost,
        }
    }
}

#[derive(Debug, StructOpt, Clone)]
pub struct Verify {
    #[structopt(long = "verbose", short = "v")]
    pub verbose: bool,
    #[structopt(flatten)]
    pub trust_params: TrustParams,
}

#[derive(Debug, StructOpt, Clone)]
pub struct Trust {
    /// Public IDs to create Trust Proof for
    pub pub_ids: Vec<String>,
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
    Trusted(TrustParams),

    #[structopt(name = "url")]
    /// Fetch from a single public proof repository
    Url(FetchUrl),
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
    Trusted(QueryIdTrusted),
}

#[derive(Debug, StructOpt, Clone)]
pub struct QueryIdTrusted {
    #[structopt(flatten)]
    pub trust_params: TrustParams,
}

#[derive(Debug, StructOpt, Clone)]
pub struct QueryReview {
    #[structopt(flatten)]
    pub crate_: CrateSelector,
}

#[derive(Debug, StructOpt, Clone)]
pub enum Query {
    /// Query Ids
    #[structopt(name = "id")]
    Id(QueryId),

    /// Query reviews
    #[structopt(name = "review")]
    Review(QueryReview),
}

#[derive(Debug, StructOpt, Clone)]
pub enum Change {
    /// Change current Id
    #[structopt(name = "id")]
    Id(ChangeId),
}

#[derive(Debug, StructOpt, Clone)]
pub struct Git {
    /// Arguments to git command
    #[structopt(parse(from_os_str))]
    pub args: Vec<OsString>,
}

#[derive(Debug, StructOpt, Clone)]
pub enum Command {
    /// Create an Id, ...
    #[structopt(name = "new")]
    New(New),

    /// Change Ids, readme, ...
    #[structopt(name = "change")]
    Change(Change),

    /// Verify dependencies
    #[structopt(name = "verify")]
    Verify(Verify),

    /// Review a crate
    #[structopt(name = "review")]
    Review(CrateSelectorNameRequired),

    /// Flag a crate as buggy/low-quality/dangerous
    #[structopt(name = "flag")]
    Flag(CrateSelectorNameRequired),

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
    Git(Git),

    /// See changes in the local proof repository (alias to `git diff`)
    #[structopt(name = "diff")]
    Diff,

    /// Commit changes to the local proof repository (alias to `git commit -a`)
    #[structopt(name = "commit")]
    Commit,

    /// Push local changes to the public proof repository (alias to `git push HEAD`)
    #[structopt(name = "push")]
    Push,

    /// Pull changes from the public proof repository (alias to `git pull`)
    #[structopt(name = "pull")]
    Pull,
}

/// Cargo will pass the name of the `cargo-<tool>`
/// as first argument, so we just have to match it here.
#[derive(Debug, StructOpt, Clone)]
pub enum MainCommand {
    #[structopt(name = "crev")]
    Crev(Command),
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "crev", about = "Distributed code review system")]
pub struct Opts {
    #[structopt(subcommand)]
    pub command: MainCommand,
    //    #[structopt(flatten)]
    //    verbosity: Verbosity,
}
