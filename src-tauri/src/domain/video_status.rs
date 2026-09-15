use serde::{Deserialize, Serialize};

/// Media validation state — separate from [`super::publication::PublicationStatus`]
/// on purpose (section 6 of the Phase 2 brief): whether a *file* is a
/// playable, well-formed video has nothing to do with whether it has been
/// published anywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Pending,
    Validating,
    Valid,
    Invalid,
    Unsupported,
    Corrupted,
    Missing,
}

impl ValidationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ValidationStatus::Pending => "pending",
            ValidationStatus::Validating => "validating",
            ValidationStatus::Valid => "valid",
            ValidationStatus::Invalid => "invalid",
            ValidationStatus::Unsupported => "unsupported",
            ValidationStatus::Corrupted => "corrupted",
            ValidationStatus::Missing => "missing",
        }
    }
}

impl std::str::FromStr for ValidationStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "pending" => ValidationStatus::Pending,
            "validating" => ValidationStatus::Validating,
            "valid" => ValidationStatus::Valid,
            "invalid" => ValidationStatus::Invalid,
            "unsupported" => ValidationStatus::Unsupported,
            "corrupted" => ValidationStatus::Corrupted,
            "missing" => ValidationStatus::Missing,
            other => return Err(format!("unknown validation status: {other}")),
        })
    }
}

/// Whether the video's underlying file can currently be found on disk
/// (section 7). A missing/offline file must never delete the database
/// record — this status is how XP FLOW represents "the file isn't here
/// right now" without losing history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityStatus {
    Available,
    Missing,
    Moved,
    OfflineVolume,
    PermissionDenied,
}

impl AvailabilityStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AvailabilityStatus::Available => "available",
            AvailabilityStatus::Missing => "missing",
            AvailabilityStatus::Moved => "moved",
            AvailabilityStatus::OfflineVolume => "offline_volume",
            AvailabilityStatus::PermissionDenied => "permission_denied",
        }
    }
}

impl std::str::FromStr for AvailabilityStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "available" => AvailabilityStatus::Available,
            "missing" => AvailabilityStatus::Missing,
            "moved" => AvailabilityStatus::Moved,
            "offline_volume" => AvailabilityStatus::OfflineVolume,
            "permission_denied" => AvailabilityStatus::PermissionDenied,
            other => return Err(format!("unknown availability status: {other}")),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VideoPriority {
    Low,
    #[default]
    Normal,
    High,
    Urgent,
}

impl VideoPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            VideoPriority::Low => "low",
            VideoPriority::Normal => "normal",
            VideoPriority::High => "high",
            VideoPriority::Urgent => "urgent",
        }
    }
}

impl std::str::FromStr for VideoPriority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "low" => VideoPriority::Low,
            "normal" => VideoPriority::Normal,
            "high" => VideoPriority::High,
            "urgent" => VideoPriority::Urgent,
            other => return Err(format!("unknown priority: {other}")),
        })
    }
}

/// Orientation is always derived from `width`/`height` (never stored —
/// section 5: "avoid storing derived display strings when they can be
/// calculated safely").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    Vertical,
    Square,
    Landscape,
}

impl Orientation {
    pub fn from_dimensions(width: i32, height: i32) -> Option<Self> {
        if width <= 0 || height <= 0 {
            return None;
        }
        Some(match width.cmp(&height) {
            std::cmp::Ordering::Less => Orientation::Vertical,
            std::cmp::Ordering::Equal => Orientation::Square,
            std::cmp::Ordering::Greater => Orientation::Landscape,
        })
    }
}

/// Non-blocking quality signals (section 22: "not every warning should
/// make the video invalid"). Always derived from stored numeric fields at
/// read time, never persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoWarning {
    MissingAudio,
    LowResolution,
    ZeroBitrate,
    ExtremelyShort,
}
