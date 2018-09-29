use std::path::PathBuf;

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
pub enum Command {
    /// Verify review coverage of the project
    #[structopt(name = "verify")]
    Verify(Verify),
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "crev", about = "Distributed code review system")]
pub struct Opts {
    #[structopt(subcommand)]
    pub command: Command,
    //    #[structopt(flatten)]
    //    verbosity: Verbosity,
}
