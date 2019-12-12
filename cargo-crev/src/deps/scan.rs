use crate::{
    crates_io,
    deps::{
        AccumulativeCrateDetails, CountWithTotal, CrateDetails, CrateInfo, CrateStats, OwnerSetSet,
    },
    opts::{CargoOpts, CrateSelector, CrateVerify},
    prelude::*,
    repo::Repo,
    shared::{
        cargo_full_ignore_list, cargo_min_ignore_list, get_geiger_count, is_digest_clean,
        read_known_owners_list, PROJECT_SOURCE_CRATES_IO,
    },
};
use cargo::core::PackageId;
use crev_common::convert::OptionDeref;
use crev_data::proof::{self, CommonOps};
use crev_lib;
use crev_lib::VerificationStatus;
use crossbeam::{
    self,
    channel::{unbounded, Receiver},
};
use std::{
    collections::{HashMap, HashSet},
    default::Default,
    path::PathBuf,
    sync::{atomic, Arc, Mutex},
};

use crev_lib::proofdb::*;

/// Dependency scaner
///
/// Offloads dependency scanning to concurrent worker threads.
//
// I know the code here is a mess.
#[derive(Clone)]
pub struct Scanner {
    db: Arc<ProofDB>,
    trust_set: TrustSet,
    min_ignore_list: HashSet<PathBuf>,
    full_ignore_list: HashSet<PathBuf>,
    crates_io: Arc<crates_io::Client>,
    known_owners: HashSet<String>,
    requirements: crev_lib::VerificationRequirements,
    recursive: bool,
    crate_info_by_id: HashMap<PackageId, CrateInfo>,
    // all the packages that we might need to potentially analyse
    all_crates_ids: Vec<PackageId>,
    // packages that we will have to return to the caller
    selected_crates_ids: HashSet<PackageId>,
    cargo_opts: CargoOpts,
    graph: Arc<crate::repo::Graph>,
    crate_details_by_id: Arc<Mutex<HashMap<PackageId, CrateDetails>>>,
    pub roots: Vec<cargo::core::PackageId>,
}

impl Scanner {
    pub fn new(root_crate: CrateSelector, args: &CrateVerify) -> Result<Scanner> {
        let local = crev_lib::Local::auto_create_or_open()?;
        let db = local.load_db()?;
        let trust_set = if let Some(for_id) =
            local.get_for_id_from_str_opt(OptionDeref::as_deref(&args.common.for_id))?
        {
            db.calculate_trust_set(&for_id, &args.common.trust_params.clone().into())
        } else {
            // when running without an id (explicit, or current), just use an empty trust set
            crev_lib::proofdb::TrustSet::default()
        };
        let min_ignore_list = cargo_min_ignore_list();
        let full_ignore_list = cargo_full_ignore_list(false);
        let crates_io = crates_io::Client::new(&local)?;
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

        Ok(Scanner {
            db: Arc::new(db),
            trust_set,
            min_ignore_list,
            full_ignore_list,
            crates_io: Arc::new(crates_io),
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

    pub fn selected_crate_count(&self) -> usize {
        self.selected_crates_ids.len()
    }

    /// start computations on a new thread
    pub fn run(self) -> Receiver<CrateStats> {
        let (ready_tx, ready_rx) = unbounded();
        // instead of properly traversing the graph
        // to be able to calculate recursive stats,
        // we use pending channel to postpone working
        // on crates that need their dependencies to be
        // analyzed first
        let (pending_tx, pending_rx) = unbounded();

        let total_crates_len = self.selected_crate_count();
        for id in self.all_crates_ids.clone().into_iter() {
            pending_tx.send(id).unwrap();
        }

        // we share the loop-back pending tx, so we can drop
        // it once for all the worker threads, after we hit
        // the terminating condition
        let pending_tx = Arc::new(Mutex::new(Some(pending_tx)));

        if total_crates_len == 0 {
            return ready_rx;
        }

        let ready_tx_count = Arc::new(atomic::AtomicUsize::new(0));
        for _ in 0..num_cpus::get() {
            let pending_rx = pending_rx.clone();
            let pending_tx = pending_tx.clone();
            let ready_tx = ready_tx.clone();
            let ready_tx_count = ready_tx_count.clone();
            let mut self_clone = self.clone();
            let ready_tx_count = ready_tx_count.clone();
            let ready_tx_count_clone = ready_tx_count.clone();
            std::thread::spawn({
                move || {
                    pending_rx.into_iter().for_each(move |pkg_id: PackageId| {
                        {
                            let graph = &self_clone.graph;
                            let crate_details_by_id =
                                self_clone.crate_details_by_id.lock().unwrap();

                            for dep_pkg_id in graph.get_dependencies_of(pkg_id) {
                                if !crate_details_by_id.contains_key(&dep_pkg_id) {
                                    if let Some(pending_tx) = pending_tx.lock().unwrap().as_mut() {
                                        pending_tx.send(pkg_id).unwrap();
                                    }
                                    return;
                                }
                            }
                        }

                        let info = self_clone.crate_info_by_id[&pkg_id].to_owned();

                        let details = self_clone
                            .get_crate_details(&info)
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

                    assert_eq!(
                        ready_tx_count_clone.load(atomic::Ordering::SeqCst),
                        total_crates_len
                    );
                }
            });
        }

        ready_rx
    }

    fn get_crate_details(&mut self, info: &CrateInfo) -> Result<CrateDetails> {
        let pkg_name = info.id.name();
        let proof_pkg_id = proof::PackageId {
            source: "https://crates.io".into(),
            name: pkg_name.to_string(),
        };

        let pkg_version = info.id.version();
        info.download_if_needed(self.cargo_opts.clone())?;
        let geiger_count = get_geiger_count(&info.root).ok();
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
        let unclean_digest = digest
            .as_ref()
            .map(|digest| !is_digest_clean(&self.db, &pkg_name, &pkg_version, &digest))
            .unwrap_or(false);
        let verification_result = if let Some(digest) = digest.as_ref() {
            self.db
                .verify_package_digest(&digest, &self.trust_set, &self.requirements)
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
                Some(&info.id.version()),
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

        let version_downloads = match self.crates_io.get_downloads_count(&pkg_name, &pkg_version) {
            Ok((version, total)) => Some(CountWithTotal {
                count: version,
                total,
            }),
            Err(_) => None,
        };

        let owner_list = self.crates_io.get_owners(&pkg_name)?;
        let total_owners_count = owner_list.len();
        let known_owners_count = owner_list
            .iter()
            .filter(|o| self.known_owners.contains(o.as_str()))
            .count();
        let known_owners = CountWithTotal {
            count: known_owners_count as u64,
            total: total_owners_count as u64,
        };

        let issues_from_trusted = self.db.get_open_issues_for_version(
            PROJECT_SOURCE_CRATES_IO,
            &pkg_name,
            &pkg_version,
            &self.trust_set,
            self.requirements.trust_level.into(),
        );

        let issues_from_all = self.db.get_open_issues_for_version(
            PROJECT_SOURCE_CRATES_IO,
            &pkg_name,
            &pkg_version,
            &self.trust_set,
            crev_data::Level::None.into(),
        );

        let issues = CountWithTotal {
            count: issues_from_trusted.len() as u64,
            total: issues_from_all.len() as u64,
        };

        let loc = crate::tokei::get_rust_line_count(&info.root).ok();

        let latest_trusted_version = self.db.find_latest_trusted_version(
            &self.trust_set,
            PROJECT_SOURCE_CRATES_IO,
            &pkg_name,
            &self.requirements,
        );

        let is_unmaintained = self
            .db
            .get_pkg_flags(&proof_pkg_id)
            .any(|(id, flags)| self.trust_set.contains_trusted(id) && flags.unmaintained);

        let owner_set = OwnerSetSet::new(info.id, owner_list);

        let accumulative_own = AccumulativeCrateDetails {
            trust: verification_result,
            trusted_issues: issues,
            geiger_count,
            loc,
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
            version_downloads,
            known_owners,
            unclean_digest,
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
