use std::path::PathBuf;
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
pub struct Add {
    #[structopt(parse(from_os_str))]
    /// Paths to add
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, StructOpt, Clone)]
pub struct Remove {
    #[structopt(parse(from_os_str))]
    /// Paths to remove
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, StructOpt, Clone)]
pub struct TrustAdd {
    /// Public IDs to create Trust Proof for
    pub pub_ids: Vec<String>,
}

#[derive(Debug, StructOpt, Clone)]
pub enum Trust {
    #[structopt(name = "add")]
    /// Create a new Trust Proof
    Add(TrustAdd),

}

#[derive(Debug, StructOpt, Clone)]
pub enum Db {
    #[structopt(name = "git")]
    /// Run git commands in your local db
    Git(Git),
    #[structopt(name = "fetch")]
    /// Update trustdb by fetching updates from trusted sources
    Fetch,
}

#[derive(Debug, StructOpt, Clone)]
pub struct Git {
    /// Arguments to git command
    #[structopt(parse(from_os_str))]
    pub args: Vec<OsString>,
}

#[derive(Debug, StructOpt, Clone)]
pub struct ProjectReview {
    #[structopt(long = "allow-dirty")]
    pub allow_dirty: bool,
}

#[derive(Debug, StructOpt, Clone)]
pub enum Project {
    #[structopt(name = "init")]
    /// Init `.crev` directory
    Init,
    #[structopt(name = "review")]
    /// Create a Review Proof for the whole directory
    Review(ProjectReview),
    #[structopt(name = "verify")]
    /// Create a Review Proof for the whole directory
    Verify,
}


#[derive(Debug, StructOpt, Clone)]
pub struct Verify {
    depth: u64,
    #[structopt(long = "high-cost")]
    high_cost: u64,
    #[structopt(long = "medium-cost")]
    medium_cost: u64,
    #[structopt(long = "low-cost")]
    low_cost: u64,
}

#[derive(Debug, StructOpt, Clone)]
pub struct Commit {
    #[structopt(long = "all", short = "a")]
    pub all: bool,
    #[structopt(long = "allow-dirty")]
    pub allow_dirty: bool,
}

#[derive(Debug, StructOpt, Clone)]
pub enum Command {
    #[structopt(name = "id")]
    /// CrevID management
    Id(Id),
    #[structopt(name = "add")]
    /// Add paths to reviewed list
    Add(Add),
    /// Create a new Review Proof from reviewed list
    #[structopt(name = "commit")]
    Commit(Commit),
    #[structopt(name = "project")]
    /// Project settings
    Project(Project),
    #[structopt(name = "status")]
    /// Display pending review list
    Status,
    #[structopt(name = "rm")]
    /// Remove path from reviewed list
    Remove(Remove),
    /// Verify review coverage of the project
    Verify(Verify),
    #[structopt(name = "trust")]
    /// Trust Store management
    Trust(Trust),
    /// Trust Store
    #[structopt(name = "db")]
    Db(Db),
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "crev", about = "Distributed code review system")]
pub struct Opts {
    #[structopt(subcommand)]
    pub command: Command,
    //    #[structopt(flatten)]
    //    verbosity: Verbosity,
}
