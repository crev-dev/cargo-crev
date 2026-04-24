use std::default::Default;

pub use code::*;
use derive_builder::Builder;
pub use package::{Draft, *};
use serde::{Deserialize, Serialize};

use crate::level::Level;

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

/// Information about an LLM agent that performed the review.
///
/// Presence of this field in a review proof (code or package)
/// indicates that the review was conducted by (or with assistance of)
/// an LLM agent.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmAgentInfo {
    /// Model identifier (e.g. "claude-opus-4-6", "gpt-4o")
    pub model: String,
    /// Model version string, if available (e.g. "2026-04-01")
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        rename = "model-version"
    )]
    pub model_version: Option<String>,
    /// Whether a human guided the agent during the review (interactive
    /// session), as opposed to a fully autonomous review.
    #[serde(rename = "human-guided", default)]
    pub human_guided: bool,
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
