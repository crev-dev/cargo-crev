use std::ffi::OsString;

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
}

#[derive(Debug, StructOpt, Clone)]
pub struct Verify {
    #[structopt(long = "depth", default_value = "10")]
    depth: u64,
    #[structopt(long = "high-cost", default_value = "0")]
    high_cost: u64,
    #[structopt(long = "medium-cost", default_value = "1")]
    medium_cost: u64,
    #[structopt(long = "low-cost", default_value = "5")]
    low_cost: u64,
}

#[derive(Debug, StructOpt, Clone)]
pub struct Crate {
    pub name: String,
    pub version: Option<String>,
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
    /// Verify review coverage of the project
    #[structopt(name = "verify")]
    Verify(Verify),
    /// Positively review a crate
    #[structopt(name = "review")]
    Review(Crate),
    /// Flag a crate as buggy/low-quality/dangerous
    #[structopt(name = "flag")]
    Flag(Crate),
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
