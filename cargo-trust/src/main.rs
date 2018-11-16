#![allow(deprecated)]
//#[macro_use]

#[macro_use]
extern crate quicli;

#[macro_use]
extern crate structopt;

use failure::format_err;

use cargo::{
    core::{package_id::PackageId, SourceId},
    util::important_paths::find_root_manifest_for_wd,
};
use common_failures::prelude::*;
use crev_lib::{self, local::Local};
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

    fn for_every_dependency_dir(
        &self,
        mut f: impl FnMut(&PackageId, &Path) -> Result<()>,
    ) -> Result<()> {
        let workspace = cargo::core::Workspace::new(&self.manifest_path, &self.config)?;
        let specs = cargo::ops::Packages::All.to_package_id_specs(&workspace)?;
        let (package_set, _resolve) = cargo::ops::resolve_ws_precisely(
            &workspace,
            None,
            &[],
            true,  // all_features
            false, // no_default_features
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

            f(&pkg_id, &pkg.root())?;
        }

        Ok(())
    }

    fn find_dependency_dir(&self, name: &str, version: Option<String>) -> Result<PathBuf> {
        let mut dir = None;

        self.for_every_dependency_dir(|pkg_id, path| {
            if name == pkg_id.name().as_str()
                && (version.is_none() || version == Some(pkg_id.version().to_string()))
            {
                dir = Some(path.to_owned());
            }
            Ok(())
        })?;

        return dir.ok_or(format_err!("Not found"));
    }
}

enum TrustOrDistrust {
    Trust,
    Distrust,
}

impl TrustOrDistrust {
    fn to_default_score(&self) -> crev_data::Score {
        use self::TrustOrDistrust::*;
        match self {
            Trust => crev_data::Score::new_default_trust(),
            Distrust => crev_data::Score::new_default_trust(),
        }
    }
}

fn set_trust(args: &opts::Trust, trust: TrustOrDistrust) -> Result<()> {
    let repo = Repo::auto_open_cwd()?;
    let pkg_dir = repo.find_dependency_dir(&args.name, args.version.clone())?;
    let local = Local::auto_open()?;
    let crev_repo = crev_lib::repo::Repo::new(pkg_dir.clone())?;
    let project_config = crev_repo.load_project_config()?;

    let ignore_list = HashSet::new();
    let digest = crev_lib::calculate_recursive_digest_for_dir(&pkg_dir, ignore_list)?;
    let passphrase = crev_common::read_passphrase()?;
    let id = local.read_unlocked_id(&passphrase)?;

    let review = crev_data::proof::review::ProjectBuilder::default()
        .from(id.id.to_owned())
        .project(project_config.project)
        .digest(digest)
        .score(trust.to_default_score())
        .build()
        .map_err(|e| format_err!("{}", e))?;

    let review = crev_lib::util::edit_proof_content_iteractively(
        &review.into(),
        crev_data::proof::ProofType::Project,
    )?;

    let proof = review.sign_by(&id)?;

    local.append_proof(&proof)?;
    Ok(())
}

main!(|opts: opts::Opts| match opts.command {
    opts::Command::Verify(_verify_opts) => {
        let local = crev_lib::Local::auto_open()?;
        let repo = Repo::auto_open_cwd()?;
        let params = Default::default();
        let (db, trust_set) = local.load_db(&params)?;

        let mut ignore_list = HashSet::new();
        ignore_list.insert(PathBuf::from(".cargo-ok"));
        repo.for_every_dependency_dir(|_, path| {
            print!("{} ", path.display());
            println!(
                "{}",
                crev_lib::dir_verify(path, ignore_list.clone(), &db, &trust_set)?.to_string()
            );

            Ok(())
        })?;
    }
    opts::Command::Trust(args) => {
        set_trust(&args, TrustOrDistrust::Trust)?;
    }
    opts::Command::Distrust(args) => {
        set_trust(&args, TrustOrDistrust::Distrust)?;
    }
});
