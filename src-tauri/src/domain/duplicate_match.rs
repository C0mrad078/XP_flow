use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Exact duplicates (identical `content_hash`) are never persisted as a
/// second [`super::video::Video`] row at all (section 30 — "do not create
/// another record by default"); a `DuplicateMatch` exists specifically for
/// the fuzzier case: two *different* video rows that a perceptual-hash
/// comparison thinks are probably the same clip (section 28/29). The user
/// decides what to do with those; XP FLOW never discards a file over a
/// `Possible` match on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchType {
    Exact,
    Possible,
}

impl MatchType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MatchType::Exact => "exact",
            MatchType::Possible => "possible",
        }
    }
}

impl std::str::FromStr for MatchType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "exact" => Ok(MatchType::Exact),
            "possible" => Ok(MatchType::Possible),
            other => Err(format!("unknown match type: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateMatch {
    pub id: Uuid,
    pub video_id: Uuid,
    pub matched_video_id: Uuid,
    /// 0.0..=1.0 confidence the two clips are the same content.
    pub similarity: f64,
    pub match_type: MatchType,
    pub created_at: DateTime<Utc>,
}

impl DuplicateMatch {
    pub fn possible(video_id: Uuid, matched_video_id: Uuid, similarity: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            video_id,
            matched_video_id,
            similarity,
            match_type: MatchType::Possible,
            created_at: Utc::now(),
        }
    }
}
