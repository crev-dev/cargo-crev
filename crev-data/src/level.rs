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
        use Level::*;
        f.write_str(match self {
            None => "none",
            Low => "low",
            Medium => "medium",
            High => "high",
        })
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Can't convert string to Level")]
pub struct FromStrErr;

impl std::str::FromStr for Level {
    type Err = FromStrErr;

    fn from_str(s: &str) -> std::result::Result<Level, FromStrErr> {
        Ok(match s {
            "none" => Level::None,
            "low" => Level::Low,
            "medium" => Level::Medium,
            "high" => Level::High,
            _ => return Err(FromStrErr),
        })
    }
}
