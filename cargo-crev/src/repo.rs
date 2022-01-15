use crate::{opts, opts::CrateSelector};
use anyhow::format_err;
use cargo::{
    core::{
        dependency::{DepKind, Dependency},
        manifest::ManifestMetadata,
        package::PackageSet,
        registry::PackageRegistry,
        resolver::{features::RequestedFeatures, CliFeatures, ResolveOpts, VersionPreferences},
        source::SourceMap,
        Package, PackageId, Resolve, SourceId, Workspace,
    },
    util::{
        config::{Config, ConfigValue},
        important_paths::find_root_manifest_for_wd,
        CargoResult, Rustc,
    },
};
use cargo_platform::Cfg;
use petgraph::graph::NodeIndex;
use std::{
    collections::{hash_map::Entry, BTreeSet, HashMap, HashSet},
    env,
    path::PathBuf,
    str::{self, FromStr},
};

use crate::{crates_io, prelude::*};

#[derive(Debug)]
struct Node {
    id: PackageId,
    #[allow(unused)]
    metadata: ManifestMetadata,
}

#[derive(Debug)]
pub struct Graph {
    graph: petgraph::Graph<Node, DepKind>,
    nodes: HashMap<PackageId, NodeIndex>,
}

impl Graph {
    pub fn get_all_pkg_ids<'s>(&'s self) -> impl Iterator<Item = PackageId> + 's {
        self.nodes.keys().cloned()
    }

    pub fn get_dependencies_of<'s>(
        &'s self,
        pkg_id: PackageId,
    ) -> impl Iterator<Item = PackageId> + 's {
        self.nodes
            .get(&pkg_id)
            .into_iter()
            .flat_map(move |node_idx| {
                self.graph
                    .neighbors_directed(*node_idx, petgraph::Direction::Outgoing)
            })
            .map(move |node_idx| self.graph.node_weight(node_idx).unwrap().id)
    }

    pub fn get_reverse_dependencies_of<'s>(
        &'s self,
        pkg_id: PackageId,
    ) -> impl Iterator<Item = PackageId> + 's {
        self.nodes
            .get(&pkg_id)
            .into_iter()
            .flat_map(move |node_idx| {
                self.graph
                    .neighbors_directed(*node_idx, petgraph::Direction::Incoming)
            })
            .map(move |node_idx| self.graph.node_weight(node_idx).unwrap().id)
    }

    pub fn get_recursive_dependencies_of(&self, root_pkg_id: PackageId) -> HashSet<PackageId> {
        let mut pending = BTreeSet::new();
        let mut processed = HashSet::new();

        pending.insert(root_pkg_id);

        while let Some(pkg_id) = pending.iter().next().cloned() {
            pending.remove(&pkg_id);

            if processed.contains(&pkg_id) {
                continue;
            } else {
                processed.insert(pkg_id);
            }

            if let Some(node_idx) = self.nodes.get(&pkg_id) {
                for node_idx in self
                    .graph
                    .neighbors_directed(*node_idx, petgraph::Direction::Outgoing)
                {
                    pending.insert(self.graph.node_weight(node_idx).unwrap().id);
                }
            } else {
                eprintln!(
                    "No node for {} when checking recdeps for {}",
                    pkg_id, root_pkg_id
                );
            }
        }

        processed.remove(&root_pkg_id);

        processed
    }
}

fn get_cfgs(rustc: &Rustc, target: Option<&str>) -> Result<Vec<Cfg>> {
    let mut process = rustc.process();
    process.arg("--print=cfg").env_remove("RUST_LOG");
    if let Some(ref s) = target {
        process.arg("--target").arg(s);
    }

    let output = match process.exec_with_output() {
        Ok(output) => output,
        Err(e) => return Err(e),
    };
    let output = str::from_utf8(&output.stdout).unwrap();
    let lines = output.lines();
    Ok(lines
        .map(Cfg::from_str)
        .collect::<std::result::Result<Vec<_>, cargo_platform::ParseError>>()?)
}

fn our_resolve<'a, 'cfg>(
    mut registry: PackageRegistry<'cfg>,
    workspace: &'a Workspace<'cfg>,
    features: &[String],
    all_features: bool,
    no_default_features: bool,
    no_dev_dependencies: bool,
) -> CargoResult<(PackageSet<'cfg>, Resolve)> {
    // there is bunch of slightly different ways to do it,
    // so I leave some dead code around, in case I want to
    // try the other ones, in some near future

    // this one will create a `Cargo.lock` file if it didn't exist before
    // good? not good? it also uses the registry to make it possible
    // the other methods
    // let mut registry = PackageRegistry::new(&workspace.config())?;
    let _lock = workspace.config().acquire_package_cache_lock()?;
    registry.lock_patches();
    let summaries: Vec<_> = workspace
        .members()
        .map(|m| {
            Ok((
                m.summary().to_owned(),
                ResolveOpts {
                    dev_deps: !no_dev_dependencies,
                    features: RequestedFeatures::CliFeatures(CliFeatures::from_command_line(
                        features,
                        all_features,
                        !no_default_features,
                    )?),
                },
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    let resolve = cargo::core::resolver::resolve(
        &summaries,
        workspace.root_replace(),
        &mut registry,
        &VersionPreferences::default(),
        None,
        false,
    )?;

    Ok((
        cargo::ops::get_resolved_packages(&resolve, registry)?,
        resolve,
    ))

    /* no_default_features doesn't work in this one
    let (packages, resolve) = ops::resolve_ws(workspace)?;

    let cli_features = CliFeatures {
        features: Rc::new(
            features
                .iter()
                .map(|s| FeatureValue::new(s.into()))
                .collect(),
        ),
        all_features,
        uses_default_features: !no_default_features,
    };

    let specs: Vec<_> = workspace
        .members()
        .map(|m| m.summary().package_id())
        .map(PackageIdSpec::from_package_id)
        .collect();

    let resolve = ops::resolve_with_previous(
        registry,
        workspace,
        &cli_features,
        if no_dev_dependencies {
            HasDevUnits::No
        } else {
            HasDevUnits::Yes
        },
        Some(&resolve),
        None,
        &specs,
        true,
    )?;

    Ok((packages, resolve))
        */

    /*
    // this method does not allow passing no_dev_dependencies
    let specs: Vec<_> = roots
        .map(|id| PackageIdSpec::from_package_id(id))
        .collect();

    ops::resolve_ws_precisely(
        workspace,
        features,
        all_features,
        no_default_features,
        &specs,
    )
    */

    /*
    // this does not update/create `Cargo.lock` AFAIU
    let method = Method::Required {
        dev_deps: !no_dev_dependencies,
        features: Rc::new(features.iter().map(|s| InternedString::new(s)).collect()),
        all_features,
        uses_default_features: !no_default_features,
    };

    let specs: Vec<_> = roots
        .map(|id| PackageIdSpec::from_package_id(id))
        .collect();

    ops::resolve_ws_with_method(workspace, method, &specs)
    */
}

fn build_graph<'a>(
    resolve: &'a Resolve,
    packages: &'a PackageSet<'_>,
    roots: impl Iterator<Item = PackageId>,
    target: Option<&str>,
    cfgs: &[Cfg],
) -> CargoResult<Graph> {
    let mut graph = Graph {
        graph: petgraph::Graph::new(),
        nodes: HashMap::new(),
    };

    let mut pending = vec![];
    for root in roots {
        let node = Node {
            id: root,
            metadata: packages.get_one(root)?.manifest().metadata().clone(),
        };
        graph.nodes.insert(root, graph.graph.add_node(node));
        pending.push(root);
    }

    while let Some(pkg_id) = pending.pop() {
        let idx = graph.nodes[&pkg_id];
        let pkg = packages.get_one(pkg_id)?;

        for raw_dep_id in resolve.deps_not_replaced(pkg_id) {
            let raw_dep_id = raw_dep_id.0;
            let it = pkg
                .dependencies()
                .iter()
                .filter(|d| d.matches_ignoring_source(raw_dep_id))
                .filter(|d| {
                    d.platform()
                        .and_then(|p| target.map(|t| p.matches(t, cfgs)))
                        .unwrap_or(true)
                });

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

/// Modifies the given config so that directory source replacements are removed, and references to them as well.
///
/// - For information on directory sources, [see here](https://doc.rust-lang.org/cargo/reference/source-replacement.html#directory-sources)
/// - For information on source replacement, [see here](https://doc.rust-lang.org/cargo/reference/config.html#source)
fn prune_directory_source_replacements(
    config: &mut HashMap<String, ConfigValue>,
) -> CargoResult<()> {
    if let Some(ConfigValue::Table(ref mut source_config, _)) = config.get_mut("source") {
        // To do the pruning, first, generate a graph of registry sources, where the node are the sources, and there is an edge if a source
        //  defines that it is replaced with another source.
        // Then, find the directory sources, and traverse the graph in reverse to find all the sources that are directory sources, or reference them
        //  directly or indirectly.
        // Then, the found sources can be removed from the config.
        let mut source_graph = petgraph::Graph::<String, ()>::new();
        let nodes = source_config
            .keys()
            .map(|source_key| (source_key, source_graph.add_node(source_key.clone())))
            .collect::<HashMap<_, _>>();

        source_config
            .iter()
            .for_each(|(source_name, source_config_entry)| {
                if let ConfigValue::Table(source_config_entry, _) = source_config_entry {
                    if let Some(ConfigValue::String(replacement, _)) =
                        source_config_entry.get("replace-with")
                    {
                        source_graph.add_edge(
                            *nodes.get(source_name).unwrap(),
                            *nodes.get(replacement).unwrap(),
                            (),
                        );
                    }
                }
            });
        source_graph.reverse();
        let source_entries_to_delete: HashSet<&String> = source_graph
            .externals(petgraph::Direction::Incoming)
            .filter(|leaf_node| {
                let leaf_source = &source_graph[*leaf_node];
                if let ConfigValue::Table(ref source_config_entry, _) = source_config[leaf_source] {
                    if let Some(ConfigValue::String(_, _)) = source_config_entry.get("directory") {
                        return true;
                    }
                }
                return false;
            })
            .flat_map(|leaf_node| {
                petgraph::visit::Walker::iter(
                    petgraph::visit::Bfs::new(&source_graph, leaf_node),
                    &source_graph,
                )
            })
            .map(|node| &source_graph[node])
            .collect();

        source_config.retain(|source_name, _| !source_entries_to_delete.contains(source_name));
    }
    Ok(())
}

/// A handle to the current Rust project
pub struct Repo {
    config: Config,
    cargo_opts: opts::CargoOpts,
    features_list: Vec<String>,
}

impl Repo {
    pub fn auto_open_cwd_default() -> Result<Self> {
        Self::auto_open_cwd(Default::default())
    }

    pub fn get_manifest_path(&self) -> Result<PathBuf> {
        Ok(if let Some(ref path) = self.cargo_opts.manifest_path {
            path.to_owned()
        } else {
            let cwd = env::current_dir()?;
            find_root_manifest_for_wd(&cwd)?
        })
    }

    pub fn auto_open_cwd(cargo_opts: opts::CargoOpts) -> Result<Self> {
        let mut config = Config::default()?;
        config.configure(
            0,
            /* quiet */ false,
            None,
            /* frozen: */ false,
            /* locked: */ true,
            /* offline: */ false,
            /* target dir */ &None,
            &cargo_opts.unstable_flags,
            &[],
        )?;

        config.load_values()?;
        prune_directory_source_replacements(config.values_mut()?)?;

        // how it used to be; can't find it anywhere anymore
        // let features_set =
        //     Method::split_features(&[cargo_opts.features.clone().unwrap_or_else(String::new)]);
        // let features_list = features_set.iter().map(|i| i.as_str().to_owned()).collect();
        let features_list = cargo_opts
            .features
            .clone()
            .unwrap_or_else(String::new)
            .split(',')
            .map(String::from)
            .filter(|s| !s.is_empty())
            .collect();

        Ok(Repo {
            config,
            features_list,
            cargo_opts,
        })
    }

    fn workspace(&self) -> Result<Workspace<'_>> {
        Workspace::new(&self.get_manifest_path()?, &self.config)
    }

    // TODO: Do we even need it? We should just always use a default/empty
    // registry or something? We don't have anything custom to add.
    fn registry(
        &self,
        source_ids: impl Iterator<Item = SourceId>,
    ) -> CargoResult<PackageRegistry<'_>> {
        let _lock = self.config.acquire_package_cache_lock()?;
        let mut registry = PackageRegistry::new(&self.config)?;
        registry.add_sources(source_ids)?;
        Ok(registry)
    }

    fn get_registry_from_workspace_members(&self) -> Result<(Workspace, PackageRegistry<'_>)> {
        let workspace = self.workspace()?;
        let registry = self.registry(
            workspace
                .members()
                .map(|m| m.summary().source_id().to_owned()),
        )?;
        Ok((workspace, registry))
    }

    pub fn get_dependency_graph(&self, roots: Vec<PackageId>) -> CargoResult<Graph> {
        let (workspace, registry) = self.get_registry_from_workspace_members()?;

        let (packages, resolve) = our_resolve(
            registry,
            &workspace,
            &self.features_list,
            self.cargo_opts.all_features,
            self.cargo_opts.no_default_features,
            self.cargo_opts.no_dev_dependencies,
        )?;

        let rustc = self.config.load_global_rustc(Some(&workspace))?;
        let host = rustc.host.to_string();

        let target = self
            .cargo_opts
            .target
            .as_ref()
            .map(|target| target.as_ref().unwrap_or(&host).as_str());

        let cfgs = get_cfgs(&rustc, target)?;
        let graph = build_graph(&resolve, &packages, roots.into_iter(), target, &cfgs)?;

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
            let _ = crates_io.get_downloads_count(&crate_.name(), crate_.version());
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
    ///
    /// TODO: This function doing downloads etc. is meh.
    /// Get rid of it.
    pub fn for_every_non_local_dep_crate(
        &self,
        mut f: impl FnMut(&Package) -> Result<()>,
    ) -> Result<()> {
        let workspace = self.workspace()?;

        // TODO: all pkgs instead
        let roots: Vec<_> = workspace
            .members()
            .map(|m| m.summary().package_id())
            .collect();

        let registry = self.registry(roots.iter().map(|pkgid| pkgid.source_id()))?;

        let (package_set, _resolve) = our_resolve(
            registry,
            &workspace,
            &self.features_list,
            self.cargo_opts.all_features,
            self.cargo_opts.no_default_features,
            self.cargo_opts.no_dev_dependencies,
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

            f(pkg)?;
        }

        Ok(())
    }

    /// Run `f` for every non-local dependency crate
    pub fn for_every_non_local_dep_crate_id(
        &self,
        mut f: impl FnMut(&PackageId) -> Result<()>,
    ) -> Result<()> {
        let workspace = self.workspace()?;

        // TODO: all pkgs instead
        let roots: Vec<_> = workspace
            .members()
            .map(|m| m.summary().package_id())
            .collect();

        let registry = self.registry(roots.iter().map(|pkgid| pkgid.source_id()))?;

        let (package_set, _resolve) = our_resolve(
            registry,
            &workspace,
            &self.features_list,
            self.cargo_opts.all_features,
            self.cargo_opts.no_default_features,
            self.cargo_opts.no_dev_dependencies,
        )?;

        for pkg_id in package_set.package_ids() {
            if !pkg_id.source_id().is_registry() {
                continue;
            }

            f(&pkg_id)?;
        }

        Ok(())
    }

    /*
    pub fn get_deps_package_set(&self) -> Result<PackageSet<'_>> {
        let workspace = self.workspace()?;
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
    */

    pub fn get_package_set(&self) -> Result<(PackageSet<'_>, Resolve)> {
        let (workspace, registry) = self.get_registry_from_workspace_members()?;

        our_resolve(
            registry,
            &workspace,
            &self.features_list,
            self.cargo_opts.all_features,
            self.cargo_opts.no_default_features,
            self.cargo_opts.no_dev_dependencies,
        )
    }

    pub fn find_dependency_pkg_id_by_selector(
        &self,
        name: &str,
        version: Option<&Version>,
    ) -> Result<Option<PackageId>> {
        let mut ret = vec![];

        self.for_every_non_local_dep_crate_id(|pkg_id| {
            if name == pkg_id.name().as_str()
                && (version.is_none() || version == Some(pkg_id.version()))
            {
                ret.push(pkg_id.to_owned());
            }
            Ok(())
        })?;

        match ret.len() {
            0 => Ok(None),
            1 => Ok(Some(ret[0])),
            n => bail!(
                "Ambiguous selection: {} matches found: {}",
                n,
                ret.iter()
                    .map(|pkgid| pkgid.version().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    pub fn get_crate(&self, pkg_id: &PackageId) -> Result<Package> {
        // We need to whitelist the crate, in case it was yanked
        let mut yanked_whitelist = HashSet::default();
        yanked_whitelist.insert(pkg_id.to_owned());
        let source = self.load_source_with_whitelist(yanked_whitelist)?;

        let mut source_map = SourceMap::new();
        source_map.insert(source);
        let package_set =
            cargo::core::PackageSet::new(&[pkg_id.to_owned()], source_map, &self.config)?;
        Ok(package_set.get_one(pkg_id.to_owned())?.to_owned())
    }

    pub fn find_independent_pkg_id_by_selector(
        &self,
        name: &str,
        version: Option<&Version>,
    ) -> Result<Option<PackageId>> {
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
            Dependency::parse(name, version_str.as_deref(), source.source_id())?;
        let _lock = self.config.acquire_package_cache_lock()?;
        source.query(&dependency_request, &mut |summary| summaries.push(summary))?;
        let summary = if let Some(version) = version {
            summaries.iter().find(|s| s.version() == version)
        } else {
            summaries.iter().max_by_key(|s| s.version())
        };

        Ok(summary.map(|s| s.package_id()))
    }

    pub fn find_pkgid(
        &self,
        name: &str,
        version: Option<&Version>,
        unrelated: bool,
    ) -> Result<PackageId> {
        if unrelated {
            Ok(
                self.find_independent_pkg_id_by_selector(name, version)?
                    .ok_or_else(|| format_err!("Could not find requested crate. Try updating cargo's registry index cache?"))?
                )
        } else {
            Ok(self.find_dependency_pkg_id_by_selector(name, version)?
                    .ok_or_else(|| format_err!("Could not find requested crate. Try `-u` if the crate is not a dependency."))?
                    )
        }
    }

    pub fn find_pkgid_by_crate_selector(&self, sel: &CrateSelector) -> Result<PackageId> {
        sel.ensure_name_given()?;

        let version = sel.version()?.cloned().map(Version::from);

        self.find_pkgid(sel.name.as_ref().unwrap(), version.as_ref(), sel.unrelated)
    }

    pub fn find_roots_by_crate_selector(&self, sel: &CrateSelector) -> Result<Vec<PackageId>> {
        if let Some(_name) = &sel.name {
            self.find_pkgid_by_crate_selector(sel).map(|i| vec![i])
        } else {
            Ok(self
                .workspace()?
                .members()
                .map(|m| m.package_id())
                .collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cargo::util::config::Definition;

    #[test]
    fn test_prune_directory_source_replacement() {
        // Test that:
        // {
        //     "source": {"crates-io": {"replace-with": my-vendor-source (from --config cli option)},
        //                "another-source": {"registry": path/to/registry (from --config cli option)},
        //                "my-vendor-source": {"directory": vendor (from --config cli option)}},
        // }
        // becomes:
        // {
        //     "source": {"another-source": {"registry": path/to/registry (from --config cli option)}},
        // }
        //
        // the "my-vendor-source" should get removed because it's a directory source replacement,
        // and "crates-io" should get removed because it referenced the removed "my-vendor-source"
        let crates_io_source_replacement = ConfigValue::Table(
            [(
                "replace-with".into(),
                ConfigValue::String("my-vendor-source".into(), Definition::Cli),
            )]
            .iter()
            .cloned()
            .collect(),
            Definition::Cli,
        );

        let directory_replacement = ConfigValue::Table(
            [(
                "directory".into(),
                ConfigValue::String("vendor".into(), Definition::Cli),
            )]
            .iter()
            .cloned()
            .collect(),
            Definition::Cli,
        );

        let registry_replacement = ConfigValue::Table(
            [(
                "registry".into(),
                ConfigValue::String("path/to/registry".into(), Definition::Cli),
            )]
            .iter()
            .cloned()
            .collect(),
            Definition::Cli,
        );

        let source_table = ConfigValue::Table(
            [
                ("crates-io".into(), crates_io_source_replacement),
                ("my-vendor-source".into(), directory_replacement),
                ("another-source".into(), registry_replacement.clone()),
            ]
            .iter()
            .cloned()
            .collect(),
            Definition::Cli,
        );

        let mut config_table = [("source".into(), source_table)].iter().cloned().collect();

        let expected_source_table = ConfigValue::Table(
            [("another-source".into(), registry_replacement)]
                .iter()
                .cloned()
                .collect(),
            Definition::Cli,
        );
        let expected_config_table = [("source".into(), expected_source_table)]
            .iter()
            .cloned()
            .collect();

        prune_directory_source_replacements(&mut config_table).unwrap();
        assert_eq!(config_table, expected_config_table);
    }

    #[test]
    fn test_prune_directory_source_replacement_nested() {
        // Test that:
        // {
        //     "source": {"another-source": {"registry": path/to/registry (from --config cli option)},
        //                "nested-vendor-source": {"directory": vendor (from --config cli option)},
        //                "my-vendor-source": {"replace-with": nested-vendor-source (from --config cli option)},
        //                "crates-io": {"replace-with": my-vendor-source (from --config cli option)}},
        // }
        // becomes:
        // {
        //     "source": {"another-source": {"registry": path/to/registry (from --config cli option)}},
        // }
        //
        // the "nested-vendor-source" should get removed because it's a directory source replacement,
        // and "my-vendor-source" should get removed because it referenced the removed "nested-vendor-source"
        // and "crates-io" should get removed because it referenced the removed "my-vendor-source"
        let crates_io_source_replacement = ConfigValue::Table(
            [(
                "replace-with".into(),
                ConfigValue::String("my-vendor-source".into(), Definition::Cli),
            )]
            .iter()
            .cloned()
            .collect(),
            Definition::Cli,
        );

        let nested_replacement = ConfigValue::Table(
            [(
                "replace-with".into(),
                ConfigValue::String("nested-vendor-source".into(), Definition::Cli),
            )]
            .iter()
            .cloned()
            .collect(),
            Definition::Cli,
        );

        let directory_replacement = ConfigValue::Table(
            [(
                "directory".into(),
                ConfigValue::String("vendor".into(), Definition::Cli),
            )]
            .iter()
            .cloned()
            .collect(),
            Definition::Cli,
        );

        let registry_replacement = ConfigValue::Table(
            [(
                "registry".into(),
                ConfigValue::String("path/to/registry".into(), Definition::Cli),
            )]
            .iter()
            .cloned()
            .collect(),
            Definition::Cli,
        );

        let source_table = ConfigValue::Table(
            [
                ("crates-io".into(), crates_io_source_replacement),
                ("my-vendor-source".into(), nested_replacement),
                ("nested-vendor-source".into(), directory_replacement),
                ("another-source".into(), registry_replacement.clone()),
            ]
            .iter()
            .cloned()
            .collect(),
            Definition::Cli,
        );

        let mut config_table = [("source".into(), source_table)].iter().cloned().collect();

        let expected_source_table = ConfigValue::Table(
            [("another-source".into(), registry_replacement)]
                .iter()
                .cloned()
                .collect(),
            Definition::Cli,
        );
        let expected_config_table = [("source".into(), expected_source_table)]
            .iter()
            .cloned()
            .collect();

        prune_directory_source_replacements(&mut config_table).unwrap();
        assert_eq!(config_table, expected_config_table);
    }
}
