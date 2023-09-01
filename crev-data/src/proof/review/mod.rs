use crate::level::Level;
pub use code::*;
use derive_builder::Builder;
pub use package::Draft;
pub use package::*;
use serde::{Deserialize, Serialize};
use std::default::Default;

pub mod code;
pub mod package;

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Rating {
    #[serde(alias = "dangerous")] // for backward compat with some previous versions
    Negative,
    #[default]
    Neutral,
    Positive,
    Strong,
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
        Review::new_none()
    }
}

impl Review {
    #[must_use]
    pub fn new_positive() -> Self {
        Review {
            thoroughness: Level::Low,
            understanding: Level::Medium,
            rating: Rating::Positive,
        }
    }

    #[must_use]
    pub fn new_negative() -> Self {
        Review {
            thoroughness: Level::Low,
            understanding: Level::Medium,
            rating: Rating::Negative,
        }
    }
    #[must_use]
    pub fn new_none() -> Self {
        Review {
            thoroughness: Level::None,
            understanding: Level::None,
            rating: Rating::Neutral,
        }
    }

    #[must_use]
    pub fn is_none(&self) -> bool {
        *self == Self::new_none()
    }
}
