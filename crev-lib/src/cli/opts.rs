use structopt::StructOpt;

pub use crate::local::Local;

#[derive(Debug, StructOpt, Clone)]
pub struct NewId {
    #[structopt(long = "url")]
    /// URL of a git repository to be associated with the new Id
    pub url: Option<String>,
    #[structopt(long = "github-username")]
    /// Github username (instead of --url)
    pub github_username: Option<String>,
    #[structopt(long = "https-push")]
    /// Setup `https` instead of recommended `ssh`-based push url
    pub use_https_push: bool,
}

#[derive(Debug, StructOpt, Clone)]
pub struct ExportId {
    pub id: Option<String>,
}

#[derive(Debug, StructOpt, Clone)]
pub struct SwitchId {
    /// Own Id to switch to
    pub id: String,
}

#[derive(Debug, StructOpt, Clone)]
pub enum Id {
    /// Create a new Id
    #[structopt(name = "new", alias = "id")] // alias is a hack for back-compact
    New(NewId),

    /// Export your own Id
    #[structopt(name = "export")]
    Export(ExportId),

    /// Show your own Id
    #[structopt(name = "show")]
    Show,

    /// Change current Id
    #[structopt(name = "switch")]
    Switch(SwitchId),
}

#[derive(Debug, StructOpt, Clone)]
pub enum Command {
    /// Manage your own Id (create new, show, export, import, switch)
    #[structopt(name = "id", alias = "new")]
    Id(Id),
}

/// Cargo will pass the name of the `cargo-<tool>`
/// as first argument, so we just have to match it here.
#[derive(Debug, StructOpt, Clone)]
pub enum MainCommand {
    #[structopt(name = "crev")]
    Crev(Command),
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(about = "Distributed code review system")]
#[structopt(raw(global_setting = "structopt::clap::AppSettings::ColoredHelp"))]
pub struct Opts {
    #[structopt(subcommand)]
    pub command: MainCommand,
    //    #[structopt(flatten)]
    //    verbosity: Verbosity,
}