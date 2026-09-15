use std::path::Path;

use async_trait::async_trait;

use crate::domain::media_error::MediaError;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolStatus {
    pub available: bool,
    pub path: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MediaToolchainStatus {
    pub ffmpeg: ToolStatus,
    pub ffprobe: ToolStatus,
}

/// Contract for detecting the local media toolchain (FFmpeg/FFprobe).
/// Phase 1 only reports availability so Settings/About can be honest about
/// what XP FLOW can currently do; the transcoding pipeline itself is future
/// work (see section 46 of the Phase 1 brief).
#[async_trait]
pub trait MediaService: Send + Sync {
    async fn detect(&self) -> MediaToolchainStatus;
}

/// Structured FFprobe output (section 18) — the frontend never sees raw
/// FFprobe CLI/JSON text, only this typed shape.
#[derive(Debug, Clone, Default)]
pub struct MediaProbe {
    pub duration_ms: Option<i64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub fps: Option<f64>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub bitrate: Option<i64>,
    pub has_audio: bool,
    pub container_format: Option<String>,
}

/// Real FFprobe integration (section 18/19). Implemented by
/// `infrastructure::media::FfprobeMediaProbeService`, which shells out to
/// `ffprobe -print_format json -show_format -show_streams` and parses the
/// JSON into [`MediaProbe`].
#[async_trait]
pub trait MediaProbeService: Send + Sync {
    async fn probe(&self, path: &Path) -> Result<MediaProbe, MediaError>;
}

/// Thumbnail extraction (section 24). Implemented by shelling out to
/// `ffmpeg -ss <t> -i <input> -frames:v 1 <output.jpg>`.
#[async_trait]
pub trait ThumbnailService: Send + Sync {
    async fn generate(
        &self,
        video_path: &Path,
        output_path: &Path,
        duration_ms: Option<i64>,
    ) -> Result<(), MediaError>;
}
