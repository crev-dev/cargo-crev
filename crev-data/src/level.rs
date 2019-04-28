use crate::Result;
use failure::bail;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    None,
    Low,
    Medium,
    High,
}

impl Default for Level {
    fn default() -> Self {
        Level::Medium
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Level::*;
        f.write_str(match self {
            None => "none",
            Low => "low",
            Medium => "medium",
            High => "high",
        })
    }
}

impl Level {
    #[allow(unused)]
    fn from_str(s: &str) -> Result<Level> {
        Ok(match s {
            "none" => Level::None,
            "low" => Level::Low,
            "medium" => Level::Medium,
            "high" => Level::High,
            _ => bail!("Unknown level: {}", s),
        })
    }
}
