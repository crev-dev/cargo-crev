use crate::{level::Level, proof};
use std::default::Default;

pub mod code;
pub mod project;

pub use self::{code::*, project::*};

pub trait Common: super::ContentCommon {
    fn review(&self) -> &Review;
}

/// Information about review result (score)
#[derive(Clone, Debug, Serialize, Deserialize, Builder)]
pub struct Review {
    #[builder(default = "Default::default()")]
    pub thoroughness: Level,
    #[builder(default = "Default::default()")]
    pub understanding: Level,
    #[builder(default = "Default::default()")]
    pub trust: Level,
    #[builder(default = "proof::default_distrust_level()")]
    #[serde(
        skip_serializing_if = "proof::equals_default_distrust_level",
        default = "proof::default_distrust_level"
    )]
    pub distrust: Level,
}

impl Default for Review {
    fn default() -> Self {
        Review {
            thoroughness: Level::Low,
            understanding: Level::Medium,
            trust: Level::Medium,
            distrust: Level::None,
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
            trust: Level::None,
            distrust: Level::Medium,
        }
    }
}
