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
pub struct Id {
    #[structopt(subcommand)]
    pub id_command: IdCommand,
}

#[derive(Debug, StructOpt, Clone)]
pub enum IdCommand {
    #[structopt(name = "gen")]
    /// Generate a CrevID
    Gen,
    #[structopt(name = "show")]
    /// Show CrevID information
    Show,
    #[structopt(name = "switch")]
    /// Switch to a different CrevID
    Switch(IdSwitch),
    #[structopt(name = "list")]
    /// List available CrevIDs
    List,
}

#[derive(Debug, StructOpt, Clone)]
pub struct IdSwitch {
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
pub enum Db {
    #[structopt(name = "git")]
    /// Run git commands in your own proof repository
    Git(Git),
    #[structopt(name = "fetch")]
    /// Update proof database by fetching updates from trusted sources
    Fetch,
}

#[derive(Debug, StructOpt, Clone)]
pub struct Git {
    /// Arguments to git command
    #[structopt(parse(from_os_str))]
    pub args: Vec<OsString>,
}

#[derive(Debug, StructOpt, Clone)]
pub enum Command {
    /// Verify dependencies of the current project
    #[structopt(name = "verify")]
    Verify(Verify),
    /// Positively review a crate
    #[structopt(name = "review")]
    Review(CrateSelectorNameRequired),
    /// Flag a crate as buggy/low-quality/dangerous
    #[structopt(name = "flag")]
    Flag(CrateSelectorNameRequired),
    /// ID-related operations
    #[structopt(name = "id")]
    Id(Id),
    /// Trust another user
    #[structopt(name = "trust")]
    Trust(Trust),
    /// Distrust another user
    #[structopt(name = "distrust")]
    Distrust(Trust),
    /// Trust Database operations
    #[structopt(name = "db")]
    Db(Db),
    /// List trusted ids
    #[structopt(name = "list-trusted-ids")]
    ListTrustedIds(ListTrustedIds),
    /// List reviews for a given package
    #[structopt(name = "list-reviews")]
    ListReviews(ListReviews),
}

#[derive(Debug, StructOpt, Clone)]
pub struct ListTrustedIds {
    #[structopt(flatten)]
    pub trust_params: TrustParams,
}

#[derive(Debug, StructOpt, Clone)]
pub struct ListReviews {
    #[structopt(flatten)]
    pub crate_: CrateSelector,
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
