use async_trait::async_trait;
use tokio::process::Command;

use crate::domain::ports::media_service::{MediaService, MediaToolchainStatus, ToolStatus};

use super::resolver::resolve_binary;

/// Detects the local FFmpeg/FFprobe toolchain by resolving each binary
/// (see `resolver::resolve_binary`, section 20) and shelling out to
/// `<tool> -version`. Detection only — no transcoding happens here
/// (section 46 of the Phase 1 brief; section 19 of Phase 2).
pub struct FfmpegMediaService;

impl FfmpegMediaService {
    pub fn new() -> Self {
        Self
    }

    async fn probe(binary: &str) -> ToolStatus {
        let Some(path) = resolve_binary(binary) else {
            return ToolStatus {
                available: false,
                path: None,
                version: None,
            };
        };

        match Command::new(&path).arg("-version").output().await {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let version = stdout.lines().next().map(str::to_string);
                ToolStatus {
                    available: true,
                    path: Some(path.display().to_string()),
                    version,
                }
            }
            _ => ToolStatus {
                available: false,
                path: Some(path.display().to_string()),
                version: None,
            },
        }
    }
}

impl Default for FfmpegMediaService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MediaService for FfmpegMediaService {
    async fn detect(&self) -> MediaToolchainStatus {
        MediaToolchainStatus {
            ffmpeg: Self::probe("ffmpeg").await,
            ffprobe: Self::probe("ffprobe").await,
        }
    }
}
