use Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    None,
    Low,
    Medium,
    High,
}

impl Default for Level {
    fn default() -> Self {
        Level::Low
    }
}

impl Level {
    #[allow(unused)]
    fn as_str(&self) -> &str {
        use self::Level::*;
        match self {
            None => "none",
            Low => "low",
            Medium => "medium",
            High => "high",
        }
    }
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
