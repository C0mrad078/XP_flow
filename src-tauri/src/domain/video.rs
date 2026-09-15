use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::video_status::{
    AvailabilityStatus, Orientation, ValidationStatus, VideoPriority, VideoWarning,
};

/// A `Video` is the source media asset imported into XP FLOW. It is
/// deliberately separate from [`super::publication::Publication`]: one video
/// can produce many publications (one per platform, or repeated posts).
///
/// Phase 2 expands this considerably (section 5): technical metadata comes
/// from real FFprobe analysis, `validation_status`/`availability_status`
/// track the health of the *file* independently of anything downstream,
/// and `content_hash`/`duplicate_of` back exact-duplicate detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Video {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub channel_id: Option<Uuid>,
    pub source_id: Uuid,

    pub original_filename: String,
    pub display_title: String,
    pub file_path: String,
    pub file_size_bytes: i64,
    pub extension: String,

    pub duration_ms: Option<i64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub fps: Option<f64>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub bitrate: Option<i64>,
    pub has_audio: Option<bool>,

    pub content_hash: Option<String>,
    pub perceptual_hash: Option<String>,
    pub thumbnail_path: Option<String>,

    pub validation_status: ValidationStatus,
    pub availability_status: AvailabilityStatus,
    pub duplicate_of: Option<Uuid>,
    pub priority: VideoPriority,
    pub notes: Option<String>,
    pub archived: bool,

    pub created_at: DateTime<Utc>,
    pub imported_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Video {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace_id: Uuid,
        source_id: Uuid,
        channel_id: Option<Uuid>,
        original_filename: impl Into<String>,
        display_title: impl Into<String>,
        file_path: impl Into<String>,
        file_size_bytes: i64,
        extension: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            channel_id,
            source_id,
            original_filename: original_filename.into(),
            display_title: display_title.into(),
            file_path: file_path.into(),
            file_size_bytes,
            extension: extension.into(),
            duration_ms: None,
            width: None,
            height: None,
            fps: None,
            video_codec: None,
            audio_codec: None,
            bitrate: None,
            has_audio: None,
            content_hash: None,
            perceptual_hash: None,
            thumbnail_path: None,
            validation_status: ValidationStatus::Pending,
            availability_status: AvailabilityStatus::Available,
            duplicate_of: None,
            priority: VideoPriority::Normal,
            notes: None,
            archived: false,
            created_at: now,
            imported_at: now,
            last_seen_at: now,
            updated_at: now,
        }
    }

    /// Derived, never stored (section 23/5).
    pub fn orientation(&self) -> Option<Orientation> {
        match (self.width, self.height) {
            (Some(w), Some(h)) => Orientation::from_dimensions(w, h),
            _ => None,
        }
    }

    pub fn aspect_ratio_label(&self) -> Option<String> {
        let (w, h) = (self.width?, self.height?);
        if w <= 0 || h <= 0 {
            return None;
        }
        let divisor = gcd(w, h);
        Some(format!("{}:{}", w / divisor, h / divisor))
    }

    /// Non-blocking quality signals, computed from stored fields (section 22).
    pub fn warnings(&self) -> Vec<VideoWarning> {
        let mut warnings = Vec::new();

        if self.has_audio == Some(false) {
            warnings.push(VideoWarning::MissingAudio);
        }
        if let (Some(w), Some(h)) = (self.width, self.height) {
            if w < 320 || h < 320 {
                warnings.push(VideoWarning::LowResolution);
            }
        }
        if self.bitrate == Some(0) {
            warnings.push(VideoWarning::ZeroBitrate);
        }
        if let Some(duration_ms) = self.duration_ms {
            if duration_ms > 0 && duration_ms < 1000 {
                warnings.push(VideoWarning::ExtremelyShort);
            }
        }

        warnings
    }
}

fn gcd(a: i32, b: i32) -> i32 {
    if b == 0 {
        a.max(1)
    } else {
        gcd(b, a % b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Video {
        Video::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            None,
            "clip_001.mp4",
            "Clip 001",
            "/videos/clip_001.mp4",
            1024,
            "mp4",
        )
    }

    #[test]
    fn orientation_from_dimensions() {
        let mut video = sample();
        video.width = Some(1080);
        video.height = Some(1920);
        assert_eq!(video.orientation(), Some(Orientation::Vertical));

        video.width = Some(1920);
        video.height = Some(1080);
        assert_eq!(video.orientation(), Some(Orientation::Landscape));

        video.width = Some(1080);
        video.height = Some(1080);
        assert_eq!(video.orientation(), Some(Orientation::Square));
    }

    #[test]
    fn orientation_is_none_without_dimensions() {
        assert_eq!(sample().orientation(), None);
    }

    #[test]
    fn aspect_ratio_label_reduces_to_lowest_terms() {
        let mut video = sample();
        video.width = Some(1920);
        video.height = Some(1080);
        assert_eq!(video.aspect_ratio_label().as_deref(), Some("16:9"));
    }

    #[test]
    fn warnings_flag_missing_audio_and_low_resolution() {
        let mut video = sample();
        video.has_audio = Some(false);
        video.width = Some(200);
        video.height = Some(200);
        video.duration_ms = Some(500);
        video.bitrate = Some(0);

        let warnings = video.warnings();
        assert!(warnings.contains(&VideoWarning::MissingAudio));
        assert!(warnings.contains(&VideoWarning::LowResolution));
        assert!(warnings.contains(&VideoWarning::ZeroBitrate));
        assert!(warnings.contains(&VideoWarning::ExtremelyShort));
    }

    #[test]
    fn healthy_video_has_no_warnings() {
        let mut video = sample();
        video.has_audio = Some(true);
        video.width = Some(1080);
        video.height = Some(1920);
        video.duration_ms = Some(42_000);
        video.bitrate = Some(4_000_000);

        assert!(video.warnings().is_empty());
    }
}
