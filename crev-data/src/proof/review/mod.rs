use crate::level::Level;
use std::default::Default;

pub mod code;
pub mod package;

pub use self::{code::*, package::*};

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
    Strong,
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
    pub fn new_none() -> Self {
        Review {
            thoroughness: Level::None,
            understanding: Level::None,
            rating: Rating::Neutral,
        }
    }
}
