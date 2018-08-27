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
mod review;
mod trust;
mod util;
use opts::*;
mod local;
use local::*;
mod repo;

fn show_id() -> Result<()> {
    let local = Local::auto_open()?;
    let id = local.read_locked_id()?;
    let id = id.to_pubid();
    print!("{}", &id.to_string());
    Ok(())
}

fn gen_id() -> Result<()> {
    let name = rprompt::prompt_reply_stdout("Name: ")?;
    let id = id::OwnId::generate(name);
    let passphrase = util::read_new_passphrase()?;
    let locked = id.to_locked(&passphrase)?;

    let local = Local::auto_open()?;
    local.save_locked_id(&locked)?;

    Ok(())
}

main!(|opts: opts::Opts| match opts.command {
    Some(opts::Command::Id(id)) => match id.id_command {
        opts::IdCommand::Show => show_id()?,
        opts::IdCommand::Gen => gen_id()?,
        opts::IdCommand::Url(opts::UrlCommand::Add(add)) => {
            let local = Local::auto_open()?;
            local.add_id_urls(add.urls)?;
        }
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
    Some(opts::Command::Remove(remove)) => {
        let mut repo = repo::Repo::auto_open()?;
        repo.remove(remove.paths)?;
    }
    None => {}
});

#[cfg(test)]
mod tests;
