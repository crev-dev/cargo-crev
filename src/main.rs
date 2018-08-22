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
        let mut staged = index::Staged::auto_open()?;
        for path in add.paths {
            staged.insert(&path);
        }
        staged.close()?;
    }
    Some(opts::Command::Commit) => {
        let mut staged = index::Staged::auto_open()?;
        if staged.is_empty() {
            bail!("No reviews to commit. Use `add` first.");
        }
        let passphrase = util::read_passphrase()?;
        let id = id::OwnId::auto_open(&passphrase)?;
        let unsigned_proof = proof::ReviewProof::from_staged(&id, &staged);
    }
    Some(opts::Command::Init) => {
        util::project_dir_init()?;
    }
    None => {}
});

#[cfg(test)]
mod tests;
