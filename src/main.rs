#![allow(unused)]
#![allow(deprecated)]

#[macro_use]
extern crate failure;
extern crate blake2;
extern crate chrono;
extern crate common_failures;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate argonautica;
extern crate base64;
extern crate ed25519_dalek;
extern crate hex;
extern crate miscreant;
extern crate rand;
extern crate serde_cbor;
extern crate serde_yaml;
#[macro_use]
extern crate derive_builder;
#[macro_use]
extern crate quicli;
#[macro_use]
extern crate structopt;
extern crate app_dirs;
extern crate git2;
extern crate rpassword;
extern crate rprompt;
extern crate tempdir;

use common_failures::prelude::*;
use std::{
    env, ffi,
    io::{Read, Write},
    path::PathBuf,
};
use structopt::StructOpt;

mod id;
mod level;
mod opts;
pub mod review {
    pub use super::proof::review::*;
}
pub mod trust {
    pub use super::proof::trust::*;
}
mod util;
use opts::*;
mod local;
use local::*;
mod proof;
mod repo;

fn show_id() -> Result<()> {
    let local = Local::auto_open()?;
    let id = local.read_locked_id()?;
    let id = id.to_pubid();
    print!("{}", &id.to_string());
    Ok(())
}

fn gen_id() -> Result<()> {
    eprintln!("Crev relies on personal, publicly accessible repositories to circulate proofs.");
    eprintln!("Enter public git address you're planing to use for your CrevID.");
    eprintln!("E.g.: https://github.com/<myusername>/crev-proofs");
    eprintln!("Changing it later will require manual config file editing.");
    let mut url = String::new();
    loop {
        url = rprompt::prompt_reply_stdout("Git URL: ")?;
        eprintln!("");
        eprintln!("You've entered: {}", url);
        if util::yes_or_no_was_y("Is this correct? (y/n) ")? {
            break;
        }
    }

    let id = id::OwnId::generate(url);
    eprintln!("Your CrevID will be protected by a passphrase.");
    eprintln!("There's no way to recover your CrevID if you forget your passphrase.");
    let passphrase = util::read_new_passphrase()?;
    let locked = id.to_locked(&passphrase)?;

    let local = Local::auto_create()?;
    local.save_locked_id(&locked)?;
    local.save_current_id(&id)?;

    eprintln!("Your CrevID was created and will be printed blow in encrypted form.");
    eprintln!("Make sure to back it up on another device, to prevent loosing it.");

    println!("{}", locked.to_string()?);
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
            local.trust_ids(trust.pub_ids)?;
        }
    },
    opts::Command::Add(add) => {
        let mut repo = repo::Repo::auto_open()?;
        repo.add(add.paths)?;
    }
    opts::Command::Commit => {
        let mut repo = repo::Repo::auto_open()?;
        repo.commit()?;
    }
    opts::Command::Init => {
        repo::Repo::init(PathBuf::from(".".to_string()))?;
    }
    opts::Command::Status => {
        let mut repo = repo::Repo::auto_open()?;
        repo.status()?;
    }
    opts::Command::Remove(remove) => {
        let mut repo = repo::Repo::auto_open()?;
        repo.remove(remove.paths)?;
    }
});

#[cfg(test)]
mod tests;
