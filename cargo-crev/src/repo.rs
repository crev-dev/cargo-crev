use crate::opts;
use cargo::core::dependency::Kind;
use cargo::core::manifest::ManifestMetadata;
use cargo::core::registry::PackageRegistry;
use cargo::core::resolver::Method;
use cargo::core::InternedString;
use cargo::core::{Resolve, Workspace};
use cargo::ops;
use cargo::util::CargoResult;
use cargo::{
    core::{
        dependency::Dependency, package::PackageSet, source::SourceMap, Package, PackageId,
        SourceId,
    },
    util::important_paths::find_root_manifest_for_wd,
};
use crev_common::convert::OptionDeref;
use crev_lib;
use failure::format_err;
use petgraph::graph::NodeIndex;
use std::collections::hash_map::Entry;
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;
use std::{collections::HashSet, env, path::PathBuf};

use crate::crates_io;
use crate::prelude::*;
use crate::shared::*;

#[derive(Debug)]
struct Node {
    #[allow(unused)]
    id: PackageId,
    #[allow(unused)]
    metadata: ManifestMetadata,
}

#[derive(Debug)]
pub struct Graph {
    graph: petgraph::Graph<Node, Kind>,
    nodes: HashMap<PackageId, NodeIndex>,
}

impl Graph {
    pub fn get_dependencies_of<'s>(
        &'s self,
        pkg_id: &PackageId,
    ) -> impl Iterator<Item = PackageId> + 's {
        self.nodes
            .get(pkg_id)
            .into_iter()
            .flat_map(move |node_idx| {
                self.graph
                    .neighbors_directed(*node_idx, petgraph::Direction::Outgoing)
            })
            .map(move |node_idx| self.graph.node_weight(node_idx).unwrap().id)
    }
}

fn resolve<'a, 'cfg>(
    registry: &mut PackageRegistry<'cfg>,
    workspace: &'a Workspace<'cfg>,
    features: &Vec<String>,
    all_features: bool,
    no_default_features: bool,
    no_dev_dependencies: bool,
) -> CargoResult<(PackageSet<'a>, Resolve)> {
    let (packages, resolve) = ops::resolve_ws(workspace)?;

    let method = Method::Required {
        dev_deps: !no_dev_dependencies,
        features: Rc::new(features.iter().map(|s| InternedString::new(s)).collect()),
        all_features,
        uses_default_features: !no_default_features,
    };

    let resolve =
        ops::resolve_with_previous(registry, workspace, method, Some(&resolve), None, &[], true)?;
    Ok((packages, resolve))
}

fn build_graph<'a>(
    resolve: &'a Resolve,
    packages: &'a PackageSet<'_>,
    roots: impl Iterator<Item = PackageId>,
) -> CargoResult<Graph> {
    let mut graph = Graph {
        graph: petgraph::Graph::new(),
        nodes: HashMap::new(),
    };

    let mut pending = vec![];
    for root in roots {
        let node = Node {
            id: root.clone(),
            metadata: packages.get_one(root)?.manifest().metadata().clone(),
        };
        graph.nodes.insert(root.clone(), graph.graph.add_node(node));
        pending.push(root);
    }

    while let Some(pkg_id) = pending.pop() {
        let idx = graph.nodes[&pkg_id];
        let pkg = packages.get_one(pkg_id)?;

        for raw_dep_id in resolve.deps_not_replaced(pkg_id) {
            let it = pkg
                .dependencies()
                .iter()
                .filter(|d| d.matches_ignoring_source(raw_dep_id));
            let dep_id = match resolve.replacement(raw_dep_id) {
                Some(id) => id,
                None => raw_dep_id,
            };
            for dep in it {
                let dep_idx = match graph.nodes.entry(dep_id) {
                    Entry::Occupied(e) => *e.get(),
                    Entry::Vacant(e) => {
                        pending.push(dep_id);
                        let node = Node {
                            id: dep_id,
                            metadata: packages.get_one(dep_id)?.manifest().metadata().clone(),
                        };
                        *e.insert(graph.graph.add_node(node))
                    }
                };
                graph.graph.add_edge(idx, dep_idx, dep.kind());
            }
        }
    }

    Ok(graph)
}

/// A handle to the current Rust project
pub struct Repo {
    manifest_path: PathBuf,
    config: cargo::util::config::Config,
    cargo_opts: opts::CargoOpts,
    #[allow(unused)]
    features_set: BTreeSet<InternedString>,
    features_list: Vec<String>,
}

impl Repo {
    pub fn auto_open_cwd_default() -> Result<Self> {
        Self::auto_open_cwd(Default::default())
    }

    pub fn auto_open_cwd(cargo_opts: opts::CargoOpts) -> Result<Self> {
        cargo::core::enable_nightly_features();
        let manifest_path = if let Some(ref path) = cargo_opts.manifest_path {
            path.to_owned()
        } else {
            let cwd = env::current_dir()?;
            find_root_manifest_for_wd(&cwd)?
        };
        let mut config = cargo::util::config::Config::default()?;
        config.configure(
            0,
            None,
            &None,
            /* frozen: */ false,
            /* locked: */ true,
            /* offline: */ false,
            &None,
            &cargo_opts.unstable_flags,
        )?;
        let features_set =
            Method::split_features(&[cargo_opts.features.clone().unwrap_or_else(|| String::new())]);

        let features_list = features_set.iter().map(|i| i.as_str().to_owned()).collect();
        Ok(Repo {
            manifest_path,
            config,
            features_set,
            features_list,
            cargo_opts,
        })
    }

    pub fn name(&self) -> std::borrow::Cow<'_, str> {
        self.manifest_path
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
    }

    fn workspace(&self) -> CargoResult<Workspace<'_>> {
        Workspace::new(&self.manifest_path, &self.config)
    }

    fn registry<'a>(
        &'a self,
        source_ids: impl Iterator<Item = SourceId>,
    ) -> CargoResult<PackageRegistry<'a>> {
        let mut registry = PackageRegistry::new(&self.config)?;
        registry.add_sources(source_ids)?;
        Ok(registry)
    }

    pub fn get_dependency_graph(&self) -> CargoResult<Graph> {
        let workspace = self.workspace()?;
        let mut registry = self.registry(
            workspace
                .members()
                .map(|m| m.summary().source_id().to_owned()),
        )?;
        let (packages, resolve) = resolve(
            &mut registry,
            &workspace,
            &self.features_list,
            self.cargo_opts.all_features,
            self.cargo_opts.no_default_features,
            self.cargo_opts.no_dev_dependencies,
        )?;
        let ids = packages.package_ids().collect::<Vec<_>>();
        let packages = registry.get(&ids)?;

        let graph = build_graph(
            &resolve,
            &packages,
            workspace.members().map(|m| m.package_id()),
        )?;

        Ok(graph)
    }

    pub fn update_source(&self) -> Result<()> {
        let mut source = self.load_source()?;
        let _lock = self.config.acquire_package_cache_lock()?;
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

    pub fn load_source<'a>(&'a self) -> Result<Box<dyn cargo::core::source::Source + 'a>> {
        let source_id = SourceId::crates_io(&self.config)?;
        let map = cargo::sources::SourceConfigMap::new(&self.config)?;
        let yanked_whitelist = HashSet::new();
        let source = map.load(source_id, &yanked_whitelist)?;
        Ok(source)
    }

    pub fn load_source_with_whitelist<'a>(
        &'a self,
        yanked_whitelist: HashSet<PackageId>,
    ) -> Result<Box<dyn cargo::core::source::Source + 'a>> {
        let source_id = SourceId::crates_io(&self.config)?;
        let map = cargo::sources::SourceConfigMap::new(&self.config)?;
        let source = map.load(source_id, &yanked_whitelist)?;
        Ok(source)
    }

    /// Run `f` for every non-local dependency crate
    pub fn for_every_non_local_dep_crate(
        &self,
        mut f: impl FnMut(&Package) -> Result<()>,
    ) -> Result<()> {
        // let workspace = cargo::core::Workspace::new(&self.manifest_path, &self.config)?;
        let workspace = self.workspace()?;
        // take all the packages inside current workspace
        let specs = cargo::ops::Packages::All.to_package_id_specs(&workspace)?;
        let (package_set, _resolve) = cargo::ops::resolve_ws_precisely(
            &workspace,
            &[],
            self.cargo_opts.all_features,
            self.cargo_opts.no_default_features,
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

    pub fn get_deps_package_set(&self) -> Result<PackageSet<'_>> {
        let workspace = cargo::core::Workspace::new(&self.manifest_path, &self.config)?;
        let specs = cargo::ops::Packages::All.to_package_id_specs(&workspace)?;
        let (package_set, _resolve) = cargo::ops::resolve_ws_precisely(
            &workspace,
            &self.features_list,
            self.cargo_opts.all_features,
            self.cargo_opts.no_default_features,
            &specs,
        )?;
        Ok(package_set)
    }

    pub fn find_idependent_crate_dir(
        &self,
        name: &str,
        version: Option<&Version>,
    ) -> Result<Option<Package>> {
        let mut source = if let Some(version) = version {
            // special case - we need to whitelist the crate, in case it was yanked
            let mut yanked_whitelist = HashSet::default();
            let source_id = SourceId::crates_io(&self.config)?;
            yanked_whitelist.insert(PackageId::new(name, version, source_id)?);
            self.load_source_with_whitelist(yanked_whitelist)?
        } else {
            self.load_source()?
        };
        let mut summaries = vec![];
        let version_str = version.map(ToString::to_string);
        let dependency_request =
            Dependency::parse_no_deprecated(name, version_str.as_deref(), source.source_id())?;
        let _lock = self.config.acquire_package_cache_lock()?;
        source.query(&dependency_request, &mut |summary| {
            summaries.push(summary.clone())
        })?;
        let summary = if let Some(version) = version {
            summaries.iter().find(|s| s.version() == version)
        // special case - if the crate was yanked, it's not in our `Cargo.yaml`
        // so it's not possible to get it via normal means
        // return Ok(Some(Box::new(&mut source).download_now(&self.config)?));
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

    pub fn find_dependency(
        &self,
        name: &str,
        version: Option<&Version>,
    ) -> Result<Option<Package>> {
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
