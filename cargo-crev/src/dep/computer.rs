use rayon::prelude::*;
use cargo::core::{
    package::{Package},
    package_id::PackageId,
};
use crev_common::convert::OptionDeref;
use crev_lib;
use std::{
    collections::HashSet,
    default::Default,
    path::PathBuf,
    thread,
};
use crossbeam:: {
    self,
    channel::{Sender, Receiver, unbounded},
};

use crate::prelude::*;
use crate::crates_io;
use crate::opts::*;
use crate::repo::*;
use crate::shared::*;
use crate::tokei;
use crate::dep::dep::*;

use crev_lib::{*, proofdb::*};

/// a structure internally used by the computer
struct DepRow {
    id: PackageId, // contains the name, version
    pub root: PathBuf,
    pub geiger_count: Option<u64>,
    pub has_custom_build: bool,
}
impl DepRow {
    pub fn from(pkg: & Package) -> Self {
        let id = pkg.package_id();
        let root = pkg.root().to_path_buf();
        let has_custom_build = pkg.has_custom_build();
        DepRow {
            id,
            root,
            has_custom_build,
            geiger_count: None,
        }
    }

    pub fn download_if_needed(
        &mut self,
    ) -> Result<()> {
        if !self.root.exists() {
            let repo = Repo::auto_open_cwd()?;
            let mut source = repo.load_source()?;
            source.download(self.id)?;
        }
        Ok(())
    }

    pub fn count_geiger(
        &mut self,
    ) {
        debug_assert!(self.root.exists());
        self.geiger_count = get_geiger_count(&self.root).ok();
    }


}

/// manages analysis of a crate dependency.
pub struct DepComputer {
    db: ProofDB,
    trust_set: TrustSet,
    ignore_list: HashSet<PathBuf>,
    crates_io: crates_io::Client,
    known_owners: HashSet<String>,
    requirements: crev_lib::VerificationRequirements,
    skip_verified: bool,
    skip_known_owners: bool,
}


impl DepComputer {

    pub fn new(
        args: &Verify,
    ) -> Result<DepComputer> {
        let local = crev_lib::Local::auto_create_or_open()?;
        let db = local.load_db()?;
        let trust_set = if let Some(for_id) = local.get_for_id_from_str_opt(args.for_id.as_deref())? {
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
        Ok(DepComputer {
            db,
            trust_set,
            ignore_list,
            crates_io,
            known_owners,
            requirements,
            skip_verified,
            skip_known_owners,
        })
    }

    /// start computations on a new thread, and return
    /// - a channel receiver, to get new events
    /// - a channel sender, to ask for computation stop
    pub fn run_computation(self) -> Receiver<ComputationEvent> {
        let (tx_events, rx_events) = unbounded();
        thread::spawn(move || {
            match self.compute_all(tx_events) {
                Ok(()) => {
                    //println!("OK - computation done"); // we need a better logging
                }
                Err(_e) => {
                    //println!("NOT OK: {:?}", e);
                }
            }
        });
        rx_events
    }

    /// do the computation
    fn compute_all(mut self, tx_events: Sender<ComputationEvent>) -> Result<()> {
        let status = TableComputationStatus::New;
        tx_events.send(ComputationEvent::from_status(status)).unwrap();

        let repo = Repo::auto_open_cwd()?;
        let package_set = repo.non_local_dep_crates()?;
        let pkgs = package_set.get_many(package_set.package_ids())?;
        let mut rows: Vec<DepRow> = pkgs.iter()
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
        thread::spawn(move|| {
            loop {
                if rx_geiger.recv().is_err() {
                    // TODO log
                    break;
                }
                progress.done += 1;
                let status = TableComputationStatus::ComputingGeiger { progress };
                tx_geiger_events.send(ComputationEvent::from_status(status)).unwrap();
                if progress.is_complete() {
                    break;
                }
            }
        });
        rows
            .par_iter_mut()
            .for_each(|row| {
                row.download_if_needed().unwrap();
                row.count_geiger();
                tx_geiger.send(true).unwrap();
            });

        // doing the rest of the computation
        progress.done = 0;
        let status = TableComputationStatus::ComputingTrust { progress };
        tx_events.send(ComputationEvent::from_status(status)).unwrap();
        for row in rows.drain(..) {
            let dep = self.compute_dep(row);
            progress.done += 1;
            let status = TableComputationStatus::ComputingTrust{ progress };
            let event = ComputationEvent {
                computation_status: status,
                finished_dep: Some(dep),
            };
            tx_events.send(event).unwrap();
        }

        // all done
        let status = TableComputationStatus::Done;
        tx_events.send(ComputationEvent::from_status(status)).unwrap();
        Ok(())
    }

    fn compute_dep(
        &mut self,
        row: DepRow,
    ) -> Dep {
        let name = row.id.name().as_str().to_string();
        let version = row.id.version().clone();
        let computation_status = match self.try_compute_dep(&row) {
            Ok(Some(computed_dep)) => {
                DepComputationStatus::Ok{
                    computed_dep,
                }
            }
            Ok(None) => {
                DepComputationStatus::Skipped
            }
            Err(_e) => {
                //println!("Computation Failed: {:?}", e);
                DepComputationStatus::Failed
            }
        };
        Dep {
            name,
            version,
            computation_status,
            root: row.root,
            geiger_count: row.geiger_count,
            has_custom_build: row.has_custom_build,
        }
    }

    fn try_compute_dep(
        &mut self,
        row: &DepRow,
    ) -> Result<Option<ComputedDep>> {
        let crate_id = row.id;
        let name = crate_id.name().as_str().to_string();
        let version = crate_id.version();
        let crate_root = &row.root;
        let digest = crev_lib::get_dir_digest(&crate_root, &self.ignore_list)?;
        let unclean_digest = !is_digest_clean(
            &self.db, &name, &version, &digest
        );
        let result = self.db.verify_package_digest(&digest, &self.trust_set, &self.requirements);
        let verified = result.is_verified();
        if verified && self.skip_verified {
            return Ok(None);
        }

        let version_reviews_count = self.db.get_package_review_count(
            PROJECT_SOURCE_CRATES_IO,
            Some(&name),
            Some(&version),
        );
        let total_reviews_count = self.db.get_package_review_count(
            PROJECT_SOURCE_CRATES_IO,
            Some(&name),
            None,
        );
        let reviews = CrateCounts {
            version: version_reviews_count as u64,
            total: total_reviews_count as u64,
        };

        let downloads = match self.crates_io.get_downloads_count(&name, &version) {
            Ok((version, total)) => Some(CrateCounts{ version, total }),
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
                Some(TrustCount{
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

}


