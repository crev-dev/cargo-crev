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
mod opts;
mod proof;
mod util;
use opts::*;
mod repo;

fn show_id() -> Result<()> {
    let id = id::LockedId::auto_open()?;
    let id = id.to_pubid();
    print!("{}", &id.to_string());
    Ok(())
}

fn gen_id() -> Result<()> {
    let name = rprompt::prompt_reply_stdout("Name: ")?;
    let id = id::OwnId::generate(name);
    let passphrase = util::read_new_passphrase()?;
    let locked = id.to_locked(&passphrase)?;

    locked.auto_save()?;

    Ok(())
}

main!(|opts: opts::Opts| match opts.command {
    Some(opts::Command::Id(id)) => match id.id_command {
        opts::IdCommand::Show => show_id()?,
        opts::IdCommand::Gen => gen_id()?,
    },
    Some(opts::Command::Add(add)) => {
        let mut repo = repo::Repo::auto_open()?;
        repo.add(add.paths)?;
    }
    Some(opts::Command::Commit) => {
        let mut repo = repo::Repo::auto_open()?;
        repo.commit()?;
    }
    Some(opts::Command::Init) => {
        repo::Repo::init(PathBuf::from(".".to_string()))?;
    }
    Some(opts::Command::Status) => {
        let mut repo = repo::Repo::auto_open()?;
        repo.status()?;
    }
    None => {}
});

#[cfg(test)]
mod tests;
