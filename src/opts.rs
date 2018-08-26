use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt, Clone)]
pub struct Id {
    #[structopt(subcommand)]
    pub id_command: IdCommand,
}

#[derive(Debug, StructOpt, Clone)]
pub enum IdCommand {
    #[structopt(name = "gen")]
    Gen,
    #[structopt(name = "show")]
    Show,
}

#[derive(Debug, StructOpt, Clone)]
pub struct Add {
    #[structopt(parse(from_os_str))]
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, StructOpt, Clone)]
pub enum Command {
    #[structopt(name = "id")]
    /// Set password
    Id(Id),
    #[structopt(name = "add")]
    Add(Add),
    #[structopt(name = "commit")]
    Commit,
    #[structopt(name = "init")]
    Init,
    #[structopt(name = "status")]
    Status,
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "crev", about = "Distributed code review system")]
pub struct Opts {
    #[structopt(subcommand)]
    pub command: Option<Command>,
    //    #[structopt(flatten)]
    //    verbosity: Verbosity,
}
