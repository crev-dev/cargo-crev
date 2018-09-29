#![allow(deprecated)]
//#[macro_use]

#[macro_use]
extern crate quicli;

#[macro_use]
extern crate structopt;

use common_failures::prelude::*;
use crev_data::id::OwnId;
use crev_lib::{id::LockedId, local::Local, repo::Repo};
use std::path::PathBuf;
use structopt::StructOpt;

mod opts;

fn download_all_deps() -> Result<Vec<String>> {
    unimplemented!();
}

fn possibly_download_updates(_dep_name: &str) -> Result<()> {
    unimplemented!();
}

fn verify_dependency(_dep_name: &str) -> Result<()> {
    unimplemented!();
}

main!(|opts: opts::Opts| match opts.command {
    opts::Command::Verify(_verify_opts) => {
        let list_of_deps = download_all_deps()?;
        for dep_name in &list_of_deps {
            possibly_download_updates(&dep_name)?;
        }
        for dep_name in &list_of_deps {
            verify_dependency(&dep_name)?;
        }
    }
});
