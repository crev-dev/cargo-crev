//! Activities track user actions to help verified
//! multi-step flows, and confirm user intention.
//!
//! Eg. when user reviews a package we record details
//! and  we  can warn them if they attempt to create
//! a proof review which they haven't previously reviewed.
use crev_common::{
    self,
    serde::{as_rfc3339_fixed, from_rfc3339_fixed},
};
use crev_data::Version;
use serde::{Deserialize, Serialize};

pub type Date = chrono::DateTime<chrono::FixedOffset>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ReviewMode {
    Full,
    Differential,
}

impl ReviewMode {
    #[must_use]
    pub fn is_diff(self) -> bool {
        self == ReviewMode::Differential
    }

    #[must_use]
    pub fn is_full(self) -> bool {
        self == ReviewMode::Full
    }

    #[must_use]
    pub fn from_diff_flag(diff: bool) -> Self {
        if diff {
            ReviewMode::Differential
        } else {
            ReviewMode::Full
        }
    }
}

/// Which review is the most recent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestReviewActivity {
    pub source: String,
    pub name: String,
    pub version: Version,
    pub diff_base: Option<Version>,
}

/// Record of an in-progress review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewActivity {
    #[serde(
        serialize_with = "as_rfc3339_fixed",
        deserialize_with = "from_rfc3339_fixed"
    )]
    pub timestamp: Date,
    pub diff_base: Option<Version>,
}

impl ReviewActivity {
    #[must_use]
    pub fn new(diff_base: Option<Version>) -> Self {
        Self {
            timestamp: crev_common::now(),
            diff_base,
        }
    }

    #[must_use]
    pub fn to_review_mode(&self) -> ReviewMode {
        if self.diff_base.is_some() {
            ReviewMode::Differential
        } else {
            ReviewMode::Full
        }
    }
}
