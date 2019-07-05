use cargo::{
    core::{dependency::Dependency, source::SourceMap, Package, SourceId},
    util::important_paths::find_root_manifest_for_wd,
};
use crev_common::convert::OptionDeref;
use crev_lib::{self, local::Local, ProofStore, ReviewMode};
use failure::format_err;
use insideout::InsideOutIter;
use resiter::FlatMap;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashSet},
    default::Default,
    env,
    io::BufRead,
    path::{Path, PathBuf},
    process,
};
use structopt::StructOpt;

use crate::prelude::*;
use crate::crates_io::{self, *};
use crate::opts::{self, *};
use crate::unsorted_mess::*;
use crev_data::proof;
use crev_lib::TrustOrDistrust::{self, *};

/// A handle to the current Rust project
pub struct Repo {
    manifest_path: PathBuf,
    config: cargo::util::config::Config,
}

impl Repo {
    pub fn auto_open_cwd() -> Result<Self> {
        cargo::core::enable_nightly_features();
        let cwd = env::current_dir()?;
        let manifest_path = find_root_manifest_for_wd(&cwd)?;
        let mut config = cargo::util::config::Config::default()?;
        config.configure(0, None, &None, false, false, &None, &[])?;
        Ok(Repo {
            manifest_path,
            config,
        })
    }

    pub fn update_source(&self) -> Result<()> {
        let mut source = self.load_source()?;
        source.update()?;
        Ok(())
    }

    pub fn update_counts(&self) -> Result<()> {
        let local = crev_lib::Local::auto_create_or_open()?;
        let crates_io = crates_io::Client::new(&local)?;

        self.for_every_non_local_dep_crate(|crate_| {
            let _ = crates_io.get_downloads_count(&crate_.name(), &crate_.version());
            Ok(())
        })?;

        Ok(())
    }

    pub fn load_source<'a>(&'a self) -> Result<Box<cargo::core::source::Source + 'a>> {
        let source_id = SourceId::crates_io(&self.config)?;
        let map = cargo::sources::SourceConfigMap::new(&self.config)?;
        let yanked_whitelist = HashSet::new();
        let source = map.load(source_id, &yanked_whitelist)?;
        Ok(source)
    }

    /// Run `f` for every non-local dependency crate
    pub fn for_every_non_local_dep_crate(
        &self,
        mut f: impl FnMut(&Package) -> Result<()>,
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
        let mut source = self.load_source()?;

        let pkgs = package_set.get_many(package_set.package_ids())?;

        for pkg in pkgs {
            if !pkg.summary().source_id().is_registry() {
                continue;
            }

            if !pkg.root().exists() {
                source.download(pkg.package_id())?;
            }

            f(&pkg)?;
        }

        Ok(())
    }

    pub fn find_idependent_crate_dir(
        &self,
        name: &str,
        version: Option<&Version>,
    ) -> Result<Option<Package>> {
        let mut source = self.load_source()?;
        let mut summaries = vec![];
        let version_str = version.map(ToString::to_string);
        let dependency_request =
            Dependency::parse_no_deprecated(name, version_str.as_deref(), source.source_id())?;
        source.query(&dependency_request, &mut |summary| {
            summaries.push(summary.clone())
        })?;
        let summary = if let Some(version) = version {
            summaries.iter().find(|s| s.version() == version)
        } else {
            summaries.iter().max_by_key(|s| s.version())
        };

        let summary = if let Some(summary) = summary {
            summary
        } else {
            return Ok(None);
        };

        let mut source_map = SourceMap::new();
        source_map.insert(source);
        let package_set =
            cargo::core::PackageSet::new(&[summary.package_id()], source_map, &self.config)?;
        let pkg_id = summary.package_id();

        Ok(Some(package_set.get_one(pkg_id)?.to_owned()))
    }

    pub fn find_dependency(&self, name: &str, version: Option<&Version>) -> Result<Option<Package>> {
        let mut ret = vec![];

        self.for_every_non_local_dep_crate(|pkg| {
            let pkg_id = pkg.package_id();
            if name == pkg_id.name().as_str()
                && (version.is_none() || version == Some(&pkg_id.version()))
            {
                ret.push(pkg.to_owned());
            }
            Ok(())
        })?;

        match ret.len() {
            0 => Ok(None),
            1 => Ok(Some(ret[0].clone())),
            n => bail!("Ambiguous selection: {} matches found", n),
        }
    }

    pub fn find_crate(
        &self,
        name: &str,
        version: Option<&Version>,
        unrelated: UnrelatedOrDependency,
    ) -> Result<Package> {
        if unrelated.is_unrelated() {
            self.find_idependent_crate_dir(name, version)?
        } else {
            self.find_dependency(name, version)?
        }
        .ok_or_else(|| format_err!("Could not find requested crate"))
    }
}

