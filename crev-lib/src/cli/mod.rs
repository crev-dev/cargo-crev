use structopt::StructOpt;
use crate::prelude::*;

mod opts;
pub use crate::local::Local;

fn run_command(command: opts::Command) -> Result<()> {
    match command {
        opts::Command::Id(opts::Id::New(args)) => {
            let local = Local::auto_create_or_open()?;
            let res = local.generate_id(args.url, args.github_username, args.use_https_push);
            if res.is_err() {
                eprintln!("Visit https://github.com/dpc/crev/wiki/Proof-Repository for help.");
            }
            res?;
        }
        opts::Command::Id(opts::Id::Export(params)) => {
            let local = Local::auto_open()?;
            println!("{}", local.export_locked_id(params.id)?);
        }
        opts::Command::Id(opts::Id::Switch(args)) => {
            let local = Local::auto_open()?;
            local.switch_id(&args.id)?
        }
        opts::Command::Id(opts::Id::Show) => {
            let local = Local::auto_open()?;
            local.show_own_ids()?;
        }
    }
    Ok(())
}

pub fn parse() -> Result<()> {
    let opts = opts::Opts::from_args();
    let opts::MainCommand::Crev(command) = opts.command;
    run_command(command)
}