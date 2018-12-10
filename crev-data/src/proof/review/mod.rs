use crate::level::Level;
use std::default::Default;

pub mod code;
pub mod project;

pub use self::{code::*, project::*};

pub trait Common: super::ContentCommon {
    fn review(&self) -> &Review;
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Rating {
    Dangerous,
    Negative,
    Neutral,
    Positive,
    Superb,
}

impl Default for Rating {
    fn default() -> Self {
        Rating::Neutral
    }
}

/// Information about review result
#[derive(Clone, Debug, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct Review {
    #[builder(default = "Default::default()")]
    pub thoroughness: Level,
    #[builder(default = "Default::default()")]
    pub understanding: Level,
    #[builder(default = "Default::default()")]
    pub rating: Rating,
}

impl Default for Review {
    fn default() -> Self {
        Review {
            thoroughness: Level::Low,
            understanding: Level::Medium,
            rating: Rating::Positive,
        }
    }
}

impl Review {
    pub fn new_positive() -> Self {
        Default::default()
    }
    pub fn new_negative() -> Self {
        Review {
            thoroughness: Level::Low,
            understanding: Level::Medium,
            rating: Rating::Negative,
        }
    }
}
