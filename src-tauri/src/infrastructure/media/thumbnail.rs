use std::path::Path;

use async_trait::async_trait;
use tokio::process::Command;

use crate::domain::media_error::MediaError;
use crate::domain::ports::media_service::ThumbnailService;

use super::resolver::resolve_binary;

/// Extracts one representative JPEG frame per video via FFmpeg (section
/// 24). Picks a timestamp ~10% into the clip (clamped to 1..=5s) rather
/// than frame 0, which is very often solid black on freshly cut exports.
pub struct FfmpegThumbnailService;

impl FfmpegThumbnailService {
    pub fn new() -> Self {
        Self
    }

    fn pick_timestamp_seconds(duration_ms: Option<i64>) -> f64 {
        match duration_ms {
            Some(ms) if ms > 0 => {
                let ten_percent = (ms as f64 / 1000.0) * 0.1;
                ten_percent.clamp(1.0, 5.0).min(ms as f64 / 1000.0)
            }
            _ => 0.0,
        }
    }
}

impl Default for FfmpegThumbnailService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ThumbnailService for FfmpegThumbnailService {
    async fn generate(
        &self,
        video_path: &Path,
        output_path: &Path,
        duration_ms: Option<i64>,
    ) -> Result<(), MediaError> {
        let binary = resolve_binary("ffmpeg").ok_or_else(|| MediaError::ThumbnailFailed {
            path: video_path.display().to_string(),
            detail: "ffmpeg is not installed or could not be found".to_string(),
        })?;

        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| MediaError::ThumbnailFailed {
                    path: video_path.display().to_string(),
                    detail: e.to_string(),
                })?;
        }

        let timestamp = Self::pick_timestamp_seconds(duration_ms);

        let output = Command::new(&binary)
            // Section 99 quality review: don't leave ffmpeg running if this
            // task is dropped mid-flight.
            .kill_on_drop(true)
            .args(["-y", "-ss"])
            .arg(format!("{timestamp:.3}"))
            .arg("-i")
            .arg(video_path)
            .args(["-frames:v", "1", "-vf", "scale=480:-1", "-q:v", "3"])
            .arg(output_path)
            .output()
            .await
            .map_err(|e| MediaError::ThumbnailFailed {
                path: video_path.display().to_string(),
                detail: e.to_string(),
            })?;

        if !output.status.success() || !output_path.is_file() {
            return Err(MediaError::ThumbnailFailed {
                path: video_path.display().to_string(),
                detail: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_a_sane_timestamp_for_short_and_long_clips() {
        assert_eq!(FfmpegThumbnailService::pick_timestamp_seconds(None), 0.0);
        assert_eq!(FfmpegThumbnailService::pick_timestamp_seconds(Some(0)), 0.0);
        // 2s clip: 10% is 0.2s, clamped up to 1s, but capped at the clip length.
        assert_eq!(
            FfmpegThumbnailService::pick_timestamp_seconds(Some(2_000)),
            1.0
        );
        // 42s clip: 10% is 4.2s.
        assert!((FfmpegThumbnailService::pick_timestamp_seconds(Some(42_000)) - 4.2).abs() < 0.01);
        // Very long clip: clamped to 5s, never later.
        assert_eq!(
            FfmpegThumbnailService::pick_timestamp_seconds(Some(600_000)),
            5.0
        );
    }
}
