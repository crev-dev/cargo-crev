#![allow(deprecated)]
//#[macro_use]

#[macro_use]
extern crate quicli;

#[macro_use]
extern crate structopt;

use cargo::{core::SourceId, util::important_paths::find_root_manifest_for_wd};
use common_failures::prelude::*;
use crev_lib;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use structopt::StructOpt;

mod opts;

struct Repo {
    manifest_path: PathBuf,
    config: cargo::util::config::Config,
}

impl Repo {
    fn auto_open_cwd() -> Result<Self> {
        cargo::core::enable_nightly_features();
        let cwd = std::env::current_dir()?;
        let manifest_path = find_root_manifest_for_wd(&cwd)?;
        let mut config = cargo::util::config::Config::default()?;
        config.configure(0, None, &None, false, false, &None, &[])?;
        Ok(Repo {
            manifest_path,
            config,
        })
    }

    fn for_every_dependency_dir(&self, f: impl Fn(&Path) -> Result<()>) -> Result<()> {
        let workspace = cargo::core::Workspace::new(&self.manifest_path, &self.config)?;
        let specs = cargo::ops::Packages::All.to_package_id_specs(&workspace)?;
        let (package_set, _resolve) = cargo::ops::resolve_ws_precisely(
            &workspace,
            None,
            &[],
            true,  /* all_features */
            false, /* no_default_features */
            &specs,
        )?;
        let source_id = SourceId::crates_io(&self.config)?;
        let map = cargo::sources::SourceConfigMap::new(&self.config)?;
        let mut source = map.load(&source_id)?;
        source.update()?;

        for pkg_id in package_set.package_ids() {
            let pkg = package_set.get(pkg_id)?;

            if !pkg.root().exists() {
                source.download(pkg_id)?;
            }

            f(&pkg.root())?;
        }

        Ok(())
    }
}

main!(|opts: opts::Opts| match opts.command {
    opts::Command::Verify(_verify_opts) => {
        let local = crev_lib::Local::auto_open()?;
        let repo = Repo::auto_open_cwd()?;
        let params = Default::default();
        let (db, trust_set) = local.load_db(&params)?;

        let mut ignore_list = HashSet::new();
        ignore_list.insert(PathBuf::from(".cargo-ok"));
        repo.for_every_dependency_dir(|path| {
            print!("{} ", path.display());
            println!(
                "{}",
                crev_lib::dir_verify(path, ignore_list.clone(), &db, &trust_set)?.to_string()
            );

            Ok(())
        })?;
    }
    opts::Command::Trust(_) | opts::Command::Distrust(_) => unimplemented!(),
});
