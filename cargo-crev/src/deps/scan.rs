use crate::{
    crates_io,
    deps::{
        AccumulativeCrateDetails, CountWithTotal, CrateDetails, CrateInfo, CrateStats, OwnerSetSet,
    },
    opts::{CargoOpts, CrateSelector, CrateVerify},
    prelude::*,
    repo::Repo,
    shared::{
        cargo_full_ignore_list, cargo_min_ignore_list, get_crate_digest_mismatches,
        get_geiger_count, read_known_owners_list, PROJECT_SOURCE_CRATES_IO,
    },
};
use cargo::core::PackageId;
use crev_data::proof::{self, CommonOps};
use crev_lib::{self, VerificationStatus};
use crev_wot::{self, *};
use crossbeam::{self, channel::unbounded};
use std::{
    collections::{HashMap, HashSet},
    default::Default,
    path::PathBuf,
    sync::{
        atomic::{self, AtomicBool, Ordering},
        Arc, Mutex,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct RequiredDetails {
    pub geiger: bool,
    pub owners: bool,
    pub downloads: bool,
    pub loc: bool,
}

impl RequiredDetails {
    pub fn none() -> Self {
        Self {
            geiger: false,
            owners: false,
            downloads: false,
            loc: false,
        }
    }
}

/// Dependency scaner
///
/// Offloads dependency scanning to concurrent worker threads.
//
// I know the code here is a mess.
#[derive(Clone)]
pub struct Scanner {
    pub db: Arc<ProofDB>,
    pub trust_set: TrustSet,
    /// True if trust_set is not empty
    has_trusted_ids: bool,
    min_ignore_list: fnv::FnvHashSet<PathBuf>,
    full_ignore_list: fnv::FnvHashSet<PathBuf>,
    local: Arc<crev_lib::Local>,
    known_owners: HashSet<String>,
    requirements: crev_lib::VerificationRequirements,
    recursive: bool,
    crate_info_by_id: HashMap<PackageId, CrateInfo>,
    // all the packages that we might need to potentially analyse
    pub all_crates_ids: Vec<PackageId>,
    // packages that we will have to return to the caller
    selected_crates_ids: HashSet<PackageId>,
    cargo_opts: CargoOpts,
    graph: Arc<crate::repo::Graph>,
    crate_details_by_id: Arc<Mutex<HashMap<PackageId, CrateDetails>>>,
    pub roots: Vec<cargo::core::PackageId>,
}

// Something in (presumably) in the C bindings we're using is unsound and will SIGSEGV
// if the threads are still running while the main thread terminated. To prevent that
// we wrap all handles in this struct that will `join` them on `drop`.
pub struct ScannerHandle {
    threads: Vec<std::thread::JoinHandle<()>>,
    canceled_flag: Arc<AtomicBool>,
    ready_rx: crossbeam::channel::Receiver<CrateStats>,
}

impl Iterator for ScannerHandle {
    type Item = CrateStats;

    fn next(&mut self) -> Option<Self::Item> {
        self.ready_rx.recv().ok()
    }
}

impl Drop for ScannerHandle {
    fn drop(&mut self) {
        self.canceled_flag.store(true, Ordering::SeqCst);
        self.threads
            .drain(..)
            .for_each(|h| h.join().expect("deps scanner thread panicked"));
    }
}

impl Scanner {
    pub fn new(root_crate: CrateSelector, args: &CrateVerify) -> Result<Scanner> {
        let local = crev_lib::Local::auto_create_or_open()?;
        let db = local.load_db()?;
        let trust_set = local.trust_set_for_id(
            args.common.for_id.as_deref(),
            &args.common.trust_params.clone().into(),
            &db,
        )?;
        let min_ignore_list = cargo_min_ignore_list();
        let full_ignore_list = cargo_full_ignore_list(false);
        let known_owners = read_known_owners_list().unwrap_or_else(|_| HashSet::new());
        let requirements =
            crev_lib::VerificationRequirements::from(args.common.requirements.clone());
        let repo = Repo::auto_open_cwd(args.common.cargo_opts.clone())?;

        if root_crate.unrelated {
            // we would have to create a ephemeral workspace, etc.
            bail!("Unrealated crates are currently not supported");
        }

        let roots = repo.find_roots_by_crate_selector(&root_crate)?;
        let roots_set: HashSet<_> = roots.iter().cloned().collect();

        let (all_pkgs_set, _resolve) = repo.get_package_set()?;

        let graph = repo.get_dependency_graph(roots.clone())?;

        let all_pkgs_ids = graph.get_all_pkg_ids();

        let crate_info_by_id: HashMap<PackageId, CrateInfo> = all_pkgs_set
            .get_many(all_pkgs_ids)?
            .into_iter()
            .map(|pkg| (pkg.package_id(), CrateInfo::from_pkg(pkg)))
            .collect();

        let all_crates_ids = crate_info_by_id.keys().cloned().collect();

        let selected_crates_ids = crate_info_by_id
            .iter()
            .filter_map(|(id, _crate_info)| {
                if !args.skip_indirect
                    || roots_set.contains(id)
                    || graph
                        .get_reverse_dependencies_of(*id)
                        .any(|r_dep| roots.contains(&r_dep))
                {
                    Some(id)
                } else {
                    None
                }
            })
            .cloned()
            .collect();

        let has_trusted_ids = trust_set.trusted_ids().next().is_some();

        Ok(Scanner {
            db: Arc::new(db),
            trust_set,
            has_trusted_ids,
            min_ignore_list,
            full_ignore_list,
            local: Arc::new(local),
            known_owners,
            requirements,
            recursive: args.recursive,
            crate_info_by_id,
            all_crates_ids,
            selected_crates_ids,
            cargo_opts: args.common.cargo_opts.clone(),
            graph: Arc::new(graph),
            crate_details_by_id: Default::default(),
            roots,
        })
    }

    fn crates_io(&self) -> Result<crates_io::Client> {
        crates_io::Client::new(&self.local)
    }

    pub fn selected_crate_count(&self) -> usize {
        self.selected_crates_ids.len()
    }

    /// start computations on a new thread
    pub fn run(self, required_details: &RequiredDetails) -> ScannerHandle {
        if !self.has_trusted_ids {
            eprintln!("There are no trusted Ids. There is nothing to verify against.\nUse `cargo crev trust` to add trusted reviewers");
        }

        // TODO: instead of properly traversing the graph
        // to be able to calculate recursive stats,
        // we use pending channel to postpone working
        // on crates that need their dependencies to be
        // analyzed first
        let (ready_tx, ready_rx) = crossbeam::channel::unbounded();
        let (pending_tx, pending_rx) = unbounded();

        let total_crates_len = self.selected_crate_count();
        for id in self.all_crates_ids.clone().into_iter() {
            pending_tx.send(id).unwrap();
        }

        // we share the loop-back pending tx, so we can drop
        // it once for all the worker threads, after we hit
        // the terminating condition
        let pending_tx = Arc::new(Mutex::new(Some(pending_tx)));
        let canceled_flag = Arc::new(AtomicBool::new(false));

        if total_crates_len == 0 {
            return ScannerHandle {
                threads: vec![],
                canceled_flag,
                ready_rx,
            };
        }

        let ready_tx_count = Arc::new(atomic::AtomicUsize::new(0));
        let threads: Vec<_> = (0..num_cpus::get())
            .map(|_| {
                let pending_rx = pending_rx.clone();
                let pending_tx = pending_tx.clone();
                let ready_tx = ready_tx.clone();
                let ready_tx_count = ready_tx_count.clone();
                let mut self_clone = self.clone();
                let ready_tx_count = ready_tx_count.clone();
                let required_details = *required_details;
                std::thread::spawn({
                    let canceled_flag = canceled_flag.clone();
                    move || {
                        pending_rx.into_iter().for_each(move |pkg_id: PackageId| {
                            if canceled_flag.load(Ordering::SeqCst) {
                                *pending_tx.lock().unwrap() = None;
                                return;
                            }

                            {
                                let graph = &self_clone.graph;
                                let crate_details_by_id =
                                    self_clone.crate_details_by_id.lock().unwrap();

                                for dep_pkg_id in graph.get_dependencies_of(pkg_id) {
                                    if !crate_details_by_id.contains_key(&dep_pkg_id) {
                                        if let Some(pending_tx) =
                                            pending_tx.lock().unwrap().as_mut()
                                        {
                                            pending_tx.send(pkg_id).unwrap();
                                        }
                                        return;
                                    }
                                }
                            }

                            let info = self_clone.crate_info_by_id[&pkg_id].to_owned();

                            let details = self_clone
                                .get_crate_details(&info, &required_details)
                                .expect("Unable to scan crate");
                            {
                                let mut crate_details_by_id =
                                    self_clone.crate_details_by_id.lock().unwrap();
                                crate_details_by_id.insert(info.id, details.clone());
                            }

                            if self_clone.selected_crates_ids.contains(&pkg_id) {
                                let stats = CrateStats { info, details };

                                // ignore any problems if the receiver decided not to listen anymore
                                let _ = ready_tx.send(stats);

                                if ready_tx_count.fetch_add(1, atomic::Ordering::SeqCst) + 1
                                    == total_crates_len
                                {
                                    // we processed all the crates, let all the workers terminate
                                    *pending_tx.lock().unwrap() = None;
                                }
                            }
                        });
                    }
                })
            })
            .collect();

        ScannerHandle {
            threads,
            canceled_flag,
            ready_rx,
        }
    }

    fn get_crate_details(
        &mut self,
        info: &CrateInfo,
        required_details: &RequiredDetails,
    ) -> Result<CrateDetails> {
        let pkg_name = info.id.name();
        let proof_pkg_id = proof::PackageId {
            source: "https://crates.io".into(),
            name: pkg_name.to_string(),
        };

        let pkg_version = info.id.version();
        info.download_if_needed(self.cargo_opts.clone())?;
        let geiger_count = if required_details.geiger {
            get_geiger_count(&info.root).ok()
        } else {
            None
        };
        let is_local_source_code = !info.id.source_id().is_registry();
        let ignore_list = if is_local_source_code {
            &self.min_ignore_list
        } else {
            &self.full_ignore_list
        };
        let digest = if !is_local_source_code {
            Some(crev_lib::get_dir_digest(&info.root, ignore_list)?)
        } else {
            None
        };
        let digest_mismatches = digest
            .as_ref()
            .map(|digest| get_crate_digest_mismatches(&self.db, &pkg_name, pkg_version, digest))
            .unwrap_or(vec![]);
        let verification_result = if let Some(digest) = digest.as_ref() {
            crev_lib::verify_package_digest(digest, &self.trust_set, &self.requirements, &self.db)
        } else {
            VerificationStatus::Local
        };
        let verified = verification_result.is_verified();

        let pkg_name = info.id.name().to_string();

        let version_reviews: Vec<_> = self
            .db
            .get_package_reviews_for_package(
                PROJECT_SOURCE_CRATES_IO,
                Some(&pkg_name),
                Some(info.id.version()),
            )
            .collect();

        let version_reviews_count = version_reviews.len();
        let total_reviews_count =
            self.db
                .get_package_review_count(PROJECT_SOURCE_CRATES_IO, Some(&pkg_name), None);
        let version_review_count = CountWithTotal {
            count: version_reviews_count as u64,
            total: total_reviews_count as u64,
        };

        let crates_io = self.crates_io()?;

        let downloads = if required_details.downloads {
            crates_io.get_downloads_count(&pkg_name, pkg_version).ok()
        } else {
            None
        };

        let owner_list = if required_details.owners {
            crates_io.get_owners(&pkg_name).ok()
        } else {
            None
        };
        let known_owners = owner_list.as_ref().map(|owner_list| {
            let total_owners_count = owner_list.len();
            let known_owners_count = owner_list
                .iter()
                .filter(|o| self.known_owners.contains(o.as_str()))
                .count();
            CountWithTotal {
                count: known_owners_count as u64,
                total: total_owners_count as u64,
            }
        });

        let issues_from_trusted = self.db.get_open_issues_for_version(
            PROJECT_SOURCE_CRATES_IO,
            &pkg_name,
            pkg_version,
            &self.trust_set,
            self.requirements.trust_level.into(),
        );

        let issues_from_all = self.db.get_open_issues_for_version(
            PROJECT_SOURCE_CRATES_IO,
            &pkg_name,
            pkg_version,
            &self.trust_set,
            crev_data::Level::None.into(),
        );

        let issues = CountWithTotal {
            count: issues_from_trusted.len() as u64,
            total: issues_from_all.len() as u64,
        };

        let loc = if required_details.loc {
            crate::tokei::get_rust_line_count(&info.root).ok()
        } else {
            None
        };

        let latest_trusted_version = crev_lib::find_latest_trusted_version(
            &self.trust_set,
            PROJECT_SOURCE_CRATES_IO,
            &pkg_name,
            &self.requirements,
            &self.db,
        );

        let is_unmaintained = self
            .db
            .get_pkg_flags(&proof_pkg_id)
            .any(|(id, flags)| self.trust_set.is_trusted(id) && flags.unmaintained);

        let owner_set = OwnerSetSet::new(info.id, owner_list.into_iter().flatten());

        let accumulative_own = AccumulativeCrateDetails {
            has_trusted_ids: self.has_trusted_ids,
            trust: verification_result,
            trusted_issues: issues,
            geiger_count,
            loc: loc.map(|l| l as u64),
            verified,
            has_custom_build: info.has_custom_build,
            is_unmaintained,
            owner_set,
            is_local_source_code,
        };

        let mut accumulative_recursive = accumulative_own.clone();

        {
            let crate_details_by_id = self.crate_details_by_id.lock().expect("lock works");

            for dep_pkg_id in self
                .graph
                .get_recursive_dependencies_of(info.id)
                .into_iter()
            {
                accumulative_recursive = accumulative_recursive
                    + crate_details_by_id
                        .get(&dep_pkg_id)
                        .expect("dependency already calculated")
                        .accumulative_own
                        .clone()
            }
        }

        Ok(CrateDetails {
            digest,
            trusted_reviewers: version_reviews
                .into_iter()
                .map(|pkg_review| pkg_review.from().to_owned())
                .filter(|id| {
                    self.trust_set.get_effective_trust_level(&id.id)
                        >= self.requirements.trust_level.into()
                })
                .collect(),
            latest_trusted_version,
            version_reviews: version_review_count,
            downloads,
            known_owners,
            digest_mismatches,
            leftpad_idx: downloads
                .and_then(|d| d.recent.checked_div(accumulative_own.loc.unwrap_or(0)))
                .unwrap_or(0),
            accumulative: if self.recursive {
                accumulative_recursive.clone()
            } else {
                accumulative_own.clone()
            },
            accumulative_own,
            accumulative_recursive,
            dependencies: self
                .graph
                .get_dependencies_of(info.id)
                .map(|c| crate::cargo_pkg_id_to_crev_pkg_id(&c))
                .collect(),
            rev_dependencies: self
                .graph
                .get_reverse_dependencies_of(info.id)
                .into_iter()
                .map(|c| crate::cargo_pkg_id_to_crev_pkg_id(&c))
                .collect(),
        })
    }
}
