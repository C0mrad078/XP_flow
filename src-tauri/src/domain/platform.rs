use serde::{Deserialize, Serialize};
use std::fmt;

/// The short-form video platforms XP FLOW can eventually publish to.
///
/// This is a closed set on purpose: adding a platform is a deliberate,
/// reviewed change (new connector, new UI badge, new domain rules), not
/// something that should be possible by passing an arbitrary string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    YouTube,
    TikTok,
    Kwai,
}

impl Platform {
    pub const ALL: [Platform; 3] = [Platform::YouTube, Platform::TikTok, Platform::Kwai];

    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::YouTube => "youtube",
            Platform::TikTok => "tiktok",
            Platform::Kwai => "kwai",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Platform::YouTube => "YouTube",
            Platform::TikTok => "TikTok",
            Platform::Kwai => "Kwai",
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Platform {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "youtube" => Ok(Platform::YouTube),
            "tiktok" => Ok(Platform::TikTok),
            "kwai" => Ok(Platform::Kwai),
            other => Err(format!("unknown platform: {other}")),
        }
    }
}
