use crate::{
    crates_io,
    deps::{
        AccumulativeCrateDetails, CountWithTotal, CrateDetails, CrateInfo, CrateStats, OwnerSetSet,
    },
    opts::{CargoOpts, CrateVerify},
    prelude::*,
    repo::Repo,
    shared::{
        cargo_min_ignore_list, get_geiger_count, is_digest_clean, read_known_owners_list,
        PROJECT_SOURCE_CRATES_IO,
    },
};
use crev_common::convert::OptionDeref;
use crev_lib;
use crossbeam::{
    self,
    channel::{unbounded, Receiver},
};
use std::{
    collections::{HashMap, HashSet},
    default::Default,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crev_lib::proofdb::*;

#[derive(Clone)]
pub struct Scanner {
    db: Arc<ProofDB>,
    trust_set: TrustSet,
    ignore_list: HashSet<PathBuf>,
    crates_io: Arc<crates_io::Client>,
    known_owners: HashSet<String>,
    requirements: crev_lib::VerificationRequirements,
    skip_verified: bool,
    skip_known_owners: bool,
    crates: Vec<CrateInfo>,
    cargo_opts: CargoOpts,
    graph: Option<Arc<crate::repo::Graph>>,
    ready_details: Arc<Mutex<HashMap<cargo::core::PackageId, Option<CrateDetails>>>>,
}

impl Scanner {
    pub fn new(args: &CrateVerify) -> Result<Scanner> {
        let local = crev_lib::Local::auto_create_or_open()?;
        let db = local.load_db()?;
        let trust_set = if let Some(for_id) =
            local.get_for_id_from_str_opt(OptionDeref::as_deref(&args.common.for_id))?
        {
            db.calculate_trust_set(&for_id, &args.common.trust_params.clone().into())
        } else {
            crev_lib::proofdb::TrustSet::default()
        };
        let ignore_list = cargo_min_ignore_list();
        let crates_io = crates_io::Client::new(&local)?;
        let known_owners = read_known_owners_list().unwrap_or_else(|_| HashSet::new());
        let requirements =
            crev_lib::VerificationRequirements::from(args.common.requirements.clone());
        let skip_verified = args.skip_verified;
        let skip_known_owners = args.skip_known_owners;
        let repo = Repo::auto_open_cwd(args.common.cargo_opts.clone())?;

        if args.common.crate_.unrelated {
            bail!("Unrealated crates are currently not supported");
        }

        let roots = repo.find_roots_by_crate_selector(&args.common.crate_)?;

        let package_set = repo.get_package_set()?;

        let graph = repo.get_dependency_graph(roots)?;

        let pkg_ids = graph.get_all_pkg_ids();

        let crates: Vec<_> = package_set
            .get_many(pkg_ids)?
            .into_iter()
            .filter(|pkg| pkg.summary().source_id().is_registry())
            .map(|pkg| CrateInfo::from_pkg(pkg))
            .collect();

        let graph = if args.recursive {
            Some(Arc::new(graph))
        } else {
            None
        };

        Ok(Scanner {
            db: Arc::new(db),
            trust_set,
            ignore_list,
            crates_io: Arc::new(crates_io),
            known_owners,
            requirements,
            skip_verified,
            skip_known_owners,
            crates,
            cargo_opts: args.common.cargo_opts.clone(),
            graph,
            ready_details: Default::default(),
        })
    }

    pub fn total_crate_count(&self) -> usize {
        self.crates.len()
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

        let total_crates_len = self.crates.len();
        for info in self.crates.clone().into_iter() {
            pending_tx.send(info).unwrap();
        }

        // we share the loop-back pending tx, so we can drop
        // it once for all the worker threads, after we hit
        // the terminating condition
        let pending_tx = Arc::new(Mutex::new(Some(pending_tx)));

        for _ in 0..num_cpus::get() {
            let pending_rx = pending_rx.clone();
            let pending_tx = pending_tx.clone();
            let ready_tx = ready_tx.clone();
            let mut self_clone = self.clone();
            std::thread::spawn(move || {
                pending_rx
                    .into_iter()
                    .map(move |info: CrateInfo| {
                        if let Some(ref graph) = self_clone.graph {
                            let ready_details = self_clone.ready_details.lock().unwrap();

                            for dep_pkg_id in graph.get_dependencies_of(info.id) {
                                if !ready_details.contains_key(&dep_pkg_id) {
                                    pending_tx
                                        .lock()
                                        .unwrap()
                                        .as_mut()
                                        .unwrap()
                                        .send(info)
                                        .unwrap();
                                    return;
                                }
                            }
                        }

                        let details = self_clone.get_crate_details(&info);
                        let mut ready_details = self_clone.ready_details.lock().unwrap();
                        ready_details
                            .insert(info.id, details.as_ref().ok().and_then(|d| d.clone()));
                        if ready_details.len() == total_crates_len {
                            // we processed all the crates, let all the workers terminate
                            *pending_tx.lock().unwrap() = None;
                        }

                        drop(ready_details);
                        ready_tx
                            .send(CrateStats { info, details })
                            .expect("channel will be there waiting for the pool");
                    })
                    .count()
            });
        }

        ready_rx
    }

    fn get_crate_details(&mut self, info: &CrateInfo) -> Result<Option<CrateDetails>> {
        let pkg_name = info.id.name();
        let pkg_version = info.id.version();
        info.download_if_needed(self.cargo_opts.clone())?;
        let geiger_count = get_geiger_count(&info.root).ok();
        let digest = crev_lib::get_dir_digest(&info.root, &self.ignore_list)?;
        let unclean_digest = !is_digest_clean(&self.db, &pkg_name, &pkg_version, &digest);
        let result = self
            .db
            .verify_package_digest(&digest, &self.trust_set, &self.requirements);
        let verified = result.is_verified();
        if verified && self.skip_verified {
            return Ok(None);
        }

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

        let (known_owners, owner_list) = match self.crates_io.get_owners(&pkg_name) {
            Ok(owners) => {
                let total_owners_count = owners.len();
                let known_owners_count = owners
                    .iter()
                    .filter(|o| self.known_owners.contains(o.as_str()))
                    .count();
                if known_owners_count > 0 && self.skip_known_owners {
                    return Ok(None);
                }
                (
                    Some(CountWithTotal {
                        count: known_owners_count as u64,
                        total: total_owners_count as u64,
                    }),
                    Some(owners),
                )
            }
            Err(_) => (None, None),
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

        let owner_set = OwnerSetSet::new(info.id, owner_list.unwrap_or_else(|| vec![]));

        let accumulative_own = AccumulativeCrateDetails {
            trust: result,
            trusted_issues: issues,
            geiger_count,
            loc,
            verified,
            has_custom_build: info.has_custom_build,
            owner_set,
        };

        let mut accumulative = accumulative_own.clone();

        if let Some(ref graph) = self.graph {
            let ready_details = self.ready_details.lock().expect("lock works");

            for dep_pkg_id in graph.get_recursive_dependencies_of(info.id).into_iter() {
                match ready_details
                    .get(&dep_pkg_id)
                    .expect("dependency already calculated")
                {
                    Some(dep_details) => {
                        accumulative = accumulative + dep_details.accumulative_own.clone()
                    }
                    None => bail!("Dependency {} failed", dep_pkg_id),
                }
            }
        }

        Ok(Some(CrateDetails {
            digest,
            trusted_reviewers: version_reviews
                .into_iter()
                .map(|pkg_review| pkg_review.from.to_owned())
                .collect(),
            latest_trusted_version,
            version_reviews: version_review_count,
            version_downloads,
            known_owners,
            unclean_digest,
            accumulative_own,
            accumulative,
        }))
    }
}
