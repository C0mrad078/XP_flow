use async_trait::async_trait;
use tokio::process::Command;

use crate::domain::ports::media_service::{MediaService, MediaToolchainStatus, ToolStatus};

/// Detects the local FFmpeg/FFprobe toolchain by shelling out to
/// `<tool> -version`. This is detection only — no transcoding happens here
/// (see section 46 of the Phase 1 brief). Future builds may bundle
/// platform-specific binaries; this service is the seam that decides
/// "system PATH" vs. "bundled binary" without callers caring which.
pub struct FfmpegMediaService;

impl FfmpegMediaService {
    pub fn new() -> Self {
        Self
    }

    async fn probe(binary: &str) -> ToolStatus {
        match Command::new(binary).arg("-version").output().await {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let version = stdout.lines().next().map(str::to_string);
                ToolStatus {
                    available: true,
                    path: which(binary),
                    version,
                }
            }
            _ => ToolStatus {
                available: false,
                path: None,
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

/// Minimal, dependency-free `which`: checks each `PATH` entry for an
/// executable with this name (`.exe` on Windows).
fn which(binary: &str) -> Option<String> {
    let path_var = std::env::var_os("PATH")?;
    let exe_name = if cfg!(windows) {
        format!("{binary}.exe")
    } else {
        binary.to_string()
    };

    std::env::split_paths(&path_var)
        .map(|dir| dir.join(&exe_name))
        .find(|candidate| candidate.is_file())
        .map(|p| p.display().to_string())
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
