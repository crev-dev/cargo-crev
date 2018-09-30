#![allow(deprecated)]
//#[macro_use]

#[macro_use]
extern crate quicli;

#[macro_use]
extern crate structopt;

use common_failures::prelude::*;
use std::path::PathBuf;
use structopt::StructOpt;

use cargo::util::important_paths::find_root_manifest_for_wd;

mod opts;

struct Repo {
    manifest_path: PathBuf,
    config: cargo::util::config::Config,
}

impl Repo {
    fn auto_open_cwd() -> Result<Self> {
        let cwd = std::env::current_dir()?;
        println!("0");
        let manifest_path = find_root_manifest_for_wd(&cwd)?;
        println!("01");
        let mut config = cargo::util::config::Config::default()?;
        config.configure(0, None, &None, false, false, &None, &[])?;
        Ok(Repo {
            manifest_path,
            config,
        })
    }

    fn download_all_deps(&self) -> Result<Vec<String>> {
        println!("1");
        let workspace = cargo::core::Workspace::new(&self.manifest_path, &self.config)?;
        println!("2");
        let specs = cargo::ops::Packages::All.to_package_id_specs(&workspace)?;
        println!("2");
        let (package_set, resolve) = cargo::ops::resolve_ws_precisely(
            &workspace,
            None,
            &[],
            true,  /* all_features */
            false, /* no_default_features */
            &specs,
        )?;
        for pkg_id in package_set.package_ids() {
            let pkg = package_set.get(pkg_id)?;
            println!("{:?}", pkg);
        }
        unimplemented!();
    }

    fn possibly_download_updates(&self, _dep_name: &str) -> Result<()> {
        unimplemented!();
    }

    fn verify_dependency(&self, _dep_name: &str) -> Result<()> {
        unimplemented!();
    }
}

main!(|opts: opts::Opts| match opts.command {
    opts::Command::Verify(_verify_opts) => {
        let repo = Repo::auto_open_cwd()?;

        let list_of_deps = repo.download_all_deps()?;
        for dep_name in &list_of_deps {
            repo.possibly_download_updates(&dep_name)?;
        }
        for dep_name in &list_of_deps {
            repo.verify_dependency(&dep_name)?;
        }
    }
});
