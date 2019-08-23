use super::*;
use crate::crates_io;
use crate::repo::*;
use crate::shared::get_geiger_count;
use crev_common::convert::OptionDeref;
use crev_lib;
use crossbeam::{
    self,
    channel::{unbounded, Receiver},
};
use std::sync::Arc;
use std::{collections::HashSet, default::Default, path::PathBuf};

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
}

impl Scanner {
    pub fn new(args: &Verify) -> Result<Scanner> {
        let local = crev_lib::Local::auto_create_or_open()?;
        let db = local.load_db()?;
        let trust_set =
            if let Some(for_id) = local.get_for_id_from_str_opt(args.for_id.as_deref())? {
                db.calculate_trust_set(&for_id, &args.trust_params.clone().into())
            } else {
                crev_lib::proofdb::TrustSet::default()
            };
        let ignore_list = cargo_min_ignore_list();
        let crates_io = crates_io::Client::new(&local)?;
        let known_owners = read_known_owners_list().unwrap_or_else(|_| HashSet::new());
        let requirements = crev_lib::VerificationRequirements::from(args.requirements.clone());
        let skip_verified = args.skip_verified;
        let skip_known_owners = args.skip_known_owners;
        let repo = Repo::auto_open_cwd()?;
        let package_set = repo.get_deps_package_set()?;
        let pkg_ids = package_set.package_ids();
        let crates = package_set
            .get_many(pkg_ids)?
            .into_iter()
            .filter(|pkg| pkg.summary().source_id().is_registry())
            .map(|pkg| CrateInfo::from_pkg(pkg))
            .collect();
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
        })
    }

    pub fn total_crate_count(&self) -> usize {
        self.crates.len()
    }

    /// start computations on a new thread, and return
    /// - a channel receiver, to get new events
    /// - a channel sender, to ask for computation stop
    pub fn run(self) -> Receiver<CrateStats> {
        let (tx, rx) = unbounded();

        let pool = threadpool::Builder::new().build();
        for info in self.crates.clone().into_iter() {
            let mut self_clone = self.clone();
            let tx = tx.clone();
            pool.execute(move || {
                let details = self_clone.get_crate_details(&info);
                tx.send(CrateStats { info, details })
                    .expect("channel will be there waiting for the pool");
            });
        }

        rx
    }

    fn get_crate_details(&mut self, info: &CrateInfo) -> Result<Option<CrateDetails>> {
        let pkg_name = info.id.name();
        let pkg_version = info.id.version();
        info.download_if_needed()?;
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

        let version_reviews_count = self.db.get_package_review_count(
            PROJECT_SOURCE_CRATES_IO,
            Some(&info.id.name()),
            Some(&info.id.version()),
        );
        let total_reviews_count =
            self.db
                .get_package_review_count(PROJECT_SOURCE_CRATES_IO, Some(&pkg_name), None);
        let reviews = ReviewCount {
            version: version_reviews_count as u64,
            total: total_reviews_count as u64,
        };

        let downloads = match self.crates_io.get_downloads_count(&pkg_name, &pkg_version) {
            Ok((version, total)) => Some(DownloadCount { version, total }),
            Err(_) => None,
        };

        let owners = match self.crates_io.get_owners(&pkg_name) {
            Ok(owners) => {
                let total_owners_count = owners.len();
                let known_owners_count = owners
                    .iter()
                    .filter(|o| self.known_owners.contains(o.as_str()))
                    .count();
                if known_owners_count > 0 && self.skip_known_owners {
                    return Ok(None);
                }
                Some(TrustCount {
                    trusted: known_owners_count,
                    total: total_owners_count,
                })
            }
            Err(_) => None,
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
        let issues = TrustCount {
            trusted: issues_from_trusted.len(),
            total: issues_from_all.len(),
        };

        let loc = crate::tokei::get_rust_line_count(&info.root).ok();

        let latest_trusted_version = self.db.find_latest_trusted_version(
            &self.trust_set,
            PROJECT_SOURCE_CRATES_IO,
            &pkg_name,
            &self.requirements,
        );
        Ok(Some(CrateDetails {
            geiger_count,
            digest,
            latest_trusted_version,
            trust: result,
            reviews,
            downloads,
            owners,
            issues,
            loc,
            unclean_digest,
            verified,
        }))
    }

    /*
    /// do the computation
    fn compute_all(mut self, tx_events: Sender<Result<ComputationEvent>>) -> Result<()> {
        let status = TableComputationStatus::New;
        tx_events
            .send(Ok(ComputationEvent::from_status(status)))
            .unwrap();

        let mut rows: Vec<DepRow> = pkg_ids
            .iter()
            .filter(|pkg| pkg.summary().source_id().is_registry())
            .map(|pkg| DepRow::from(pkg))
            .collect();

        // computing in parallel (using rayon) all the geiger things
        // (which are costly)
        let (tx_geiger, rx_geiger) = unbounded();
        let mut progress = Progress {
            done: 0,
            total: rows.len(),
        };

        // we could probably avoid this thread and send the computation event from the closure
        // given to rayon but sending the correct progress value would involve a mutex, I think
        let tx_geiger_events = tx_events.clone();
        thread::spawn(move || {
            loop {
                if rx_geiger.recv().is_err() {
                    // TODO log
                    break;
                }
                progress.done += 1;
                let status = TableComputationStatus::ComputingGeiger { progress };
                tx_geiger_events
                    .send(Ok(ComputationEvent::from_status(status)))
                    .unwrap();
                if progress.is_complete() {
                    break;
                }
            }
        });
        rows.par_iter_mut().for_each(|row| {
            row.download_if_needed().unwrap();
            row.count_geiger();
            tx_geiger.send(true).unwrap();
        });

        rows.sort();

        // doing the rest of the computation
        progress.done = 0;
        let status = TableComputationStatus::ComputingTrust { progress };
        tx_events
            .send(Ok(ComputationEvent::from_status(status)))
            .unwrap();
        for row in rows.drain(..) {
            let dep = self.compute_dep(row);
            progress.done += 1;
            let status = TableComputationStatus::ComputingTrust { progress };
            let event = ComputationEvent {
                computation_status: status,
                finished_dep: Some(dep),
            };
            tx_events.send(Ok(event)).unwrap();
        }

        // all done
        let status = TableComputationStatus::Done;
        tx_events
            .send(Ok(ComputationEvent::from_status(status)))
            .unwrap();
        Ok(())
    }

    fn compute_dep(&mut self, row: DepRow) -> DependencyStats {
        let name = row.id.name().as_str().to_string();
        let version = row.id.version().clone();
        let computation_status = match self.try_compute_dep(&row) {
            Ok(Some(computed_dep)) => DepComputationStatus::Ok { computed_dep },
            Ok(None) => DepComputationStatus::Skipped,
            Err(_e) => {
                //println!("Computation Failed: {:?}", e);
                DepComputationStatus::Failed
            }
        };
        DependencyStats {
            name,
            version,
            computation_status,
            root: row.root,
            geiger_count: row.geiger_count,
            has_custom_build: row.has_custom_build,
        }
    }

    fn try_compute_dep(&mut self, row: &DepRow) -> Result<Option<ComputedDep>> {
        let crate_id = row.id;
        let name = crate_id.name().as_str().to_string();
        let version = crate_id.version();
        let crate_root = &row.root;
        let digest = crev_lib::get_dir_digest(&crate_root, &self.ignore_list)?;
        let unclean_digest = !is_digest_clean(&self.db, &name, &version, &digest);
        let result = self
            .db
            .verify_package_digest(&digest, &self.trust_set, &self.requirements);
        let verified = result.is_verified();
        if verified && self.skip_verified {
            return Ok(None);
        }

        let version_reviews_count =
            self.db
                .get_package_review_count(PROJECT_SOURCE_CRATES_IO, Some(&name), Some(&version));
        let total_reviews_count =
            self.db
                .get_package_review_count(PROJECT_SOURCE_CRATES_IO, Some(&name), None);
        let reviews = CrateCounts {
            version: version_reviews_count as u64,
            total: total_reviews_count as u64,
        };

        let downloads = match self.crates_io.get_downloads_count(&name, &version) {
            Ok((version, total)) => Some(CrateCounts { version, total }),
            Err(_) => None,
        };

        let owners = match self.crates_io.get_owners(&name) {
            Ok(owners) => {
                let total_owners_count = owners.len();
                let known_owners_count = owners
                    .iter()
                    .filter(|o| self.known_owners.contains(o.as_str()))
                    .count();
                if known_owners_count > 0 && self.skip_known_owners {
                    return Ok(None);
                }
                Some(TrustCount {
                    trusted: known_owners_count,
                    total: total_owners_count,
                })
            }
            Err(_) => None,
        };

        let issues_from_trusted = self.db.get_open_issues_for_version(
            PROJECT_SOURCE_CRATES_IO,
            &name,
            version,
            &self.trust_set,
            self.requirements.trust_level.into(),
        );
        let issues_from_all = self.db.get_open_issues_for_version(
            PROJECT_SOURCE_CRATES_IO,
            &name,
            version,
            &self.trust_set,
            crev_data::Level::None.into(),
        );
        let issues = TrustCount {
            trusted: issues_from_trusted.len(),
            total: issues_from_all.len(),
        };

        let loc = tokei::get_rust_line_count(&row.root).ok();

        let latest_trusted_version = self.db.find_latest_trusted_version(
            &self.trust_set,
            PROJECT_SOURCE_CRATES_IO,
            &name,
            &self.requirements,
        );
        Ok(Some(ComputedDep {
            digest,
            latest_trusted_version,
            trust: result,
            reviews,
            downloads,
            owners,
            issues,
            loc,
            unclean_digest,
            verified,
        }))
    }
    */
}
