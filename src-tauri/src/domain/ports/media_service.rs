use async_trait::async_trait;

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
