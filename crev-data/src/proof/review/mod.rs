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
pub enum Recommendation {
    Negative,
    Neutral,
    Positivie,
}

impl Default for Recommendation {
    fn default() -> Self {
        Recommendation::Neutral
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
    pub quality: Level,
    #[builder(default = "Default::default()")]
    pub recommendation: Recommendation,
}

impl Default for Review {
    fn default() -> Self {
        Review {
            thoroughness: Level::Low,
            understanding: Level::Medium,
            quality: Level::Medium,
            recommendation: Recommendation::Neutral,
        }
    }
}

impl Review {
    pub fn new_default_trust() -> Self {
        Default::default()
    }
    pub fn new_default_distrust() -> Self {
        Review {
            thoroughness: Level::Low,
            understanding: Level::Medium,
            quality: Level::None,
            recommendation: Recommendation::Negative,
        }
    }
}
