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
    #[structopt(name = "url")]
    Url(UrlCommand),
}

#[derive(Debug, StructOpt, Clone)]
pub struct UrlAdd {
    pub urls: Vec<String>,
}

#[derive(Debug, StructOpt, Clone)]
pub enum UrlCommand {
    #[structopt(name = "add")]
    Add(UrlAdd),
}

#[derive(Debug, StructOpt, Clone)]
pub struct Add {
    #[structopt(parse(from_os_str))]
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, StructOpt, Clone)]
pub struct Remove {
    #[structopt(parse(from_os_str))]
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, StructOpt, Clone)]
pub struct Trust {
    pub pub_ids: Vec<String>,
}

#[derive(Debug, StructOpt, Clone)]
pub enum Command {
    #[structopt(name = "id")]
    Id(Id),
    #[structopt(name = "add")]
    Add(Add),
    #[structopt(name = "commit")]
    Commit,
    #[structopt(name = "init")]
    Init,
    #[structopt(name = "status")]
    Status,
    #[structopt(name = "rm")]
    Remove(Remove),
    #[structopt(name = "trust")]
    Trust(Trust),
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "crev", about = "Distributed code review system")]
pub struct Opts {
    #[structopt(subcommand)]
    pub command: Option<Command>,
    //    #[structopt(flatten)]
    //    verbosity: Verbosity,
}
