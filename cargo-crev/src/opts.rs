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
pub struct NewId {
    pub id: String,
}

#[derive(Debug, StructOpt, Clone)]
pub enum New {
    #[structopt(name = "id")]
    /// Generate a CrevID
    Id,
}

#[derive(Debug, StructOpt, Clone)]
pub struct ChangeId {
    pub id: String,
}

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
    /// Fetch just one url
    pub url: String,
}

#[derive(Debug, StructOpt, Clone)]
pub enum Fetch {
    #[structopt(name = "trusted")]
    /// Run git commands in your own proof repository
    Trusted(TrustParams),
    #[structopt(name = "url")]
    /// Run git commands in your own proof repository
    Url(FetchUrl),
}

#[derive(Debug, StructOpt, Clone)]
pub enum QueryId {
    /// Show current Id
    #[structopt(name = "current")]
    Current,

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
    /// Ids
    #[structopt(name = "id")]
    Id(QueryId),

    /// Reviews
    #[structopt(name = "review")]
    Review(QueryReview),
}

#[derive(Debug, StructOpt, Clone)]
pub enum Change {
    /// Change current Id
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
    /// Create an object
    #[structopt(name = "new")]
    New(New),

    /// Change
    #[structopt(name = "change")]
    Change(Change),

    /// Verify dependencies of the current project
    #[structopt(name = "verify")]
    Verify(Verify),

    /// Positively review a crate
    #[structopt(name = "review")]
    Review(CrateSelectorNameRequired),

    /// Flag a crate as buggy/low-quality/dangerous
    #[structopt(name = "flag")]
    Flag(CrateSelectorNameRequired),

    /// Query
    #[structopt(name = "find")]
    Query(Query),

    /// Trust another user
    #[structopt(name = "trust")]
    Trust(Trust),

    /// Distrust another user
    #[structopt(name = "distrust")]
    Distrust(Trust),

    #[structopt(name = "fetch")]
    /// Fetch proofs from other users
    Fetch(Fetch),

    #[structopt(name = "git")]
    /// Run raw git commands in your own proof repository
    Git(Git),

    #[structopt(name = "diff")]
    /// See changes
    Diff,

    #[structopt(name = "commit")]
    /// Commit current changes
    Commit,

    #[structopt(name = "push")]
    /// Pull from remote repo
    Push,

    #[structopt(name = "pull")]
    /// Pull from remote repo
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
