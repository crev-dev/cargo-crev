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
extern crate serde_yaml;
#[macro_use]
extern crate derive_builder;
#[macro_use]
extern crate quicli;
#[macro_use]
extern crate structopt;
extern crate app_dirs;
extern crate rpassword;
extern crate rprompt;

use common_failures::prelude::*;
use std::io::{Read, Write};
use structopt::StructOpt;

mod id;
mod index;
mod opts;
mod proof;
mod util;
use opts::*;

fn show_id() -> Result<()> {
    let path = util::user_config_path()?;
    let id = id::LockedId::read_from_yaml_file(&path)?;
    let id = id.to_pubid();
    print!("{}", &id.to_string());
    Ok(())
}

fn gen_id() -> Result<()> {
    let path = util::user_config_path()?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let name = rprompt::prompt_reply_stdout("Name: ")?;
    let passphrase = util::read_new_passphrase()?;
    let id = id::OwnId::generate(name);
    let id = serde_yaml::to_string(&id.to_locked(&passphrase)?)?;
    write!(file, "{}", id)?;

    Ok(())
}

main!(|opts: opts::Opts| match opts.command {
    Some(opts::Command::Id(id)) => match id.id_command {
        opts::IdCommand::Show => show_id()?,
        opts::IdCommand::Gen => gen_id()?,
    },
    Some(opts::Command::Add(add)) => {
        let project_dir = util::project_dir_find()?;
        let index_file = project_dir.join("index");
        let mut index = index::Index::read_fom_file(&index_file)?;
        for path in add.paths {
            index.insert(&path);
        }
        index.write_to_file(&index_file)?;
    }
    Some(opts::Command::Init) => {
        util::project_dir_init()?;
    }
    None => {}
});

#[cfg(test)]
mod tests;
