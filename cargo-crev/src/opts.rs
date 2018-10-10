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
pub struct Trust {
    name: String,
    version: Option<String>,
}

#[derive(Debug, StructOpt, Clone)]
pub enum Command {
    /// Verify review coverage of the project
    #[structopt(name = "verify")]
    Verify(Verify),
    #[structopt(name = "trust")]
    Trust(Trust),
    #[structopt(name = "distrust")]
    Distrust(Trust),
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "crev", about = "Distributed code review system")]
pub struct Opts {
    #[structopt(subcommand)]
    pub command: Command,
    //    #[structopt(flatten)]
    //    verbosity: Verbosity,
}
