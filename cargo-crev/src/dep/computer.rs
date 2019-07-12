use crev_common::convert::OptionDeref;
use crev_lib;
use std::{
    collections::HashSet,
    default::Default,
    path::PathBuf,
};

use crate::prelude::*;
use crate::crates_io;
use crate::opts::*;
use crate::repo::*;
use crate::shared::*;
use crate::tokei;

use crate::dep::dep::*;

use crev_lib::{*, proofdb::*};

pub struct DepComputer<'a> {
    db: ProofDB,
    trust_set: TrustSet,
    ignore_list: HashSet<PathBuf>,
    crates_io: crates_io::Client,
    known_owners: HashSet<String>,
    requirements: crev_lib::VerificationRequirements,
    source: Box<dyn cargo::core::source::Source + 'a>,
    skip_verified: bool,
    skip_known_owners: bool,
}

impl<'a> DepComputer<'a> {

    pub fn new(repo: &'a Repo, args: &VerifyDeps) -> Result<DepComputer<'a>> {
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
        let source = repo.load_source()?;
        let skip_verified = args.skip_verified;
        let skip_known_owners = args.skip_known_owners;
        Ok(DepComputer {
            db,
            trust_set,
            ignore_list,
            crates_io,
            known_owners,
            requirements,
            source,
            skip_verified,
            skip_known_owners,
        })
    }

    fn try_compute(&mut self, row: &mut DepRow<'_>) -> Result<Option<Dep>> {
        if !row.pkg.root().exists() {
            self.source.download(row.pkg.package_id())?;
        }
        let crate_id = row.pkg.package_id();
        let name = crate_id.name().as_str().to_string();
        let version = crate_id.version();
        let crate_root = row.pkg.root();
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

        let loc = tokei::get_rust_line_count(crate_root).ok();
        let geiger_count = get_geiger_count(crate_root).ok();
        let has_custom_build = row.pkg.has_custom_build();

        let latest_trusted_version = self.db.find_latest_trusted_version(
            &self.trust_set,
            PROJECT_SOURCE_CRATES_IO,
            &name,
            &self.requirements,
        );
        Ok(Some(Dep {
            digest,
            name,
            version: version.clone(),
            latest_trusted_version,
            trust: result,
            reviews,
            downloads,
            owners,
            issues,
            loc,
            geiger_count,
            has_custom_build,
            unclean_digest,
            verified,
        }))
    }

    pub fn compute(&mut self, row: &mut DepRow<'_>) {
        row.computation_status = ComputationStatus::InProgress;
        match self.try_compute(row) {
            Ok(Some(dep)) => {
                row.computation_status = ComputationStatus::Ok{dep};
            }
            Ok(None) => {
                row.computation_status = ComputationStatus::Skipped;
            }
            Err(e) => {
                row.computation_status = ComputationStatus::Failed;
                println!("Computation Failed: {:?}", e);
            }
        }
    }
}
