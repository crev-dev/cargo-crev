#![allow(deprecated)]
//#[macro_use]


#[macro_use]
extern crate quicli;
use crev_common;



use rprompt;
#[macro_use]
extern crate structopt;

use common_failures::prelude::*;
use crev_data::id::OwnId;
use crev_lib::{id::LockedId, local::Local, repo::Repo};
use std::path::PathBuf;
use structopt::StructOpt;

mod opts;
mod util;

fn show_id() -> Result<()> {
    let local = Local::auto_open()?;
    let id = local.read_locked_id()?;
    let id = id.to_pubid();
    print!("{}", id);
    Ok(())
}

fn gen_id() -> Result<()> {
    eprintln!("Crev relies on personal, publicly accessible repositories to circulate proofs.");
    eprintln!("Enter public git address you're planing to use for your CrevID.");
    eprintln!("E.g.: https://github.com/<myusername>/crev-proofs");
    eprintln!("Changing it later will require manual config file editing.");
    let mut url;
    loop {
        url = rprompt::prompt_reply_stdout("Git URL: ")?;
        eprintln!("");
        eprintln!("You've entered: {}", url);
        if crev_common::yes_or_no_was_y("Is this correct? (y/n) ")? {
            break;
        }
    }

    let id = OwnId::generate(url);
    eprintln!("Your CrevID will be protected by a passphrase.");
    eprintln!("There's no way to recover your CrevID if you forget your passphrase.");
    let passphrase = util::read_new_passphrase()?;
    let locked = LockedId::from_own_id(&id, &passphrase)?;

    let local = Local::auto_create()?;
    local.save_locked_id(&locked)?;
    local.save_current_id(&id)?;

    eprintln!("Your CrevID was created and will be printed blow in encrypted form.");
    eprintln!("Make sure to back it up on another device, to prevent loosing it.");

    println!("{}", locked);
    Ok(())
}

main!(|opts: opts::Opts| match opts.command {
    opts::Command::Id(id) => match id.id_command {
        opts::IdCommand::Show => show_id()?,
        opts::IdCommand::Gen => gen_id()?,
    },
    opts::Command::Trust(trust) => match trust {
        opts::Trust::Add(trust) => {
            let local = Local::auto_open()?;
            let passphrase = util::read_passphrase()?;
            local.trust_ids(trust.pub_ids, passphrase)?;
        }
        opts::Trust::Update => {
            let local = Local::auto_open()?;
            local.trust_update()?;
        }
    },
    opts::Command::Add(add) => {
        let mut repo = Repo::auto_open()?;
        repo.add(add.paths)?;
    }
    opts::Command::Commit => {
        let mut repo = Repo::auto_open()?;
        let passphrase = util::read_passphrase()?;
        repo.commit(passphrase)?;
    }
    opts::Command::Init => {
        let local = Local::auto_open()?;
        let cur_id = local.read_current_id()?;
        Repo::init(PathBuf::from(".".to_string()), cur_id)?;
    }
    opts::Command::Status => {
        let mut repo = Repo::auto_open()?;
        repo.status()?;
    }
    opts::Command::Remove(remove) => {
        let mut repo = Repo::auto_open()?;
        repo.remove(remove.paths)?;
    }
    opts::Command::Verify(_verify_opts) => {
        let mut repo = Repo::auto_open()?;
        repo.verify()?;
    }
});
