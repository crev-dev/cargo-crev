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
    #[structopt(name = "id")]
    Id(Id),
}

/// Cargo will pass the name of the `cargo-<tool>`
/// as first argument, so we just have to match it here.
#[derive(Debug, StructOpt, Clone)]
pub enum MainCommand {
    #[structopt(name = "trust")]
    Trust(Command),
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "crev", about = "Distributed code review system")]
pub struct Opts {
    #[structopt(subcommand)]
    pub command: MainCommand,
    //    #[structopt(flatten)]
    //    verbosity: Verbosity,
}
