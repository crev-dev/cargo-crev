use semver::Version;
use std::path::PathBuf;

use crev_data::*;
use crev_lib::*;

#[derive(Clone, Copy, Debug)]
pub struct Progress {
    pub done: usize,
    pub total: usize,
}
impl Progress {
    pub fn is_complete(&self) -> bool {
        self.done >= self.total
    }
}

/// Current state/progress of the computation
///  of the repo's dependencies
#[derive(Clone, Copy, Debug)]
pub enum TableComputationStatus {
    New,
    ComputingGeiger {
        progress: Progress,
    },
    ComputingTrust {
        progress: Progress,
    },
    Done, // might be a crash, too
}
impl TableComputationStatus {
    /// Are we in the initial phase during which there's no dep ?
    pub fn is_before_deps(&self) -> bool {
        match self {
            TableComputationStatus::New => true,
            TableComputationStatus::ComputingGeiger{ progress:_ } => true,
            _ => false,
        }
    }
}

/// Events are the output of the computer, this is
///  where the computed data are to be read.
pub struct ComputationEvent {
    pub computation_status: TableComputationStatus,
    pub finished_dep: Option<Dep>,
}
impl ComputationEvent {
    pub fn from_status(computation_status: TableComputationStatus) -> Self {
        Self {
            computation_status,
            finished_dep: None,
        }
    }
}

/// Status of the computation of a dependency. If it's OK contains
///  the computed dependency.
pub enum DepComputationStatus {
    Ok {
        computed_dep: ComputedDep,
    },
    Skipped, // imply it's verified and args ask for skipping of verified
    Failed,
}

pub struct CrateCounts {
    pub version: u64,
    pub total: u64,
}
pub struct TrustCount {
    pub trusted: usize, // or "known" in case of crate owners
    pub total: usize,
}

/// The computed content for a dep. One field should be one
///  cell in the displayed dep table
pub struct ComputedDep {
    pub digest: Digest,
    pub latest_trusted_version: Option<Version>,
    pub trust: VerificationStatus,
    pub reviews: CrateCounts,
    pub downloads: Option<CrateCounts>,
    pub owners: Option<TrustCount>,
    pub issues: TrustCount,
    pub loc: Option<usize>,
    pub unclean_digest: bool,
    pub verified: bool,
}

/// A dependency, as returned by the computer. It may
///  contain (depending on success/slipping) the computed
///  dep.
pub struct Dep {
    pub name: String,
    pub version: Version,
    pub computation_status: DepComputationStatus,
    pub root: PathBuf,
    pub geiger_count: Option<u64>,
    pub has_custom_build: bool,
}

impl Dep {
    pub fn is_digest_unclean(&self) -> bool {
        match &self.computation_status {
            DepComputationStatus::Ok{computed_dep} => computed_dep.unclean_digest,
            _ => false,
        }
    }
    pub fn computed(&self) -> Option<&ComputedDep> {
        match &self.computation_status {
            DepComputationStatus::Ok{computed_dep} => Some(&computed_dep),
            _ => None,
        }
    }
}

pub fn latest_trusted_version_string(
    base_version: &Version,
    latest_trusted_version: &Option<Version>,
) -> String {
    if let Some(latest_trusted_version) = latest_trusted_version {
        format!(
            "{}{}",
            if base_version < latest_trusted_version {
                "↑"
            } else if latest_trusted_version < base_version {
                "↓"
            } else {
                "="
            },
            &latest_trusted_version,
        )
    } else {
        "".to_owned()

    }
}
