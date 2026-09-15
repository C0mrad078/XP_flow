use std::path::Path;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::process::Command;

use crate::domain::media_error::MediaError;
use crate::domain::ports::media_service::{MediaProbe, MediaProbeService};

use super::resolver::resolve_binary;

/// Real FFprobe integration (section 18): shells out to
/// `ffprobe -print_format json -show_format -show_streams` and parses the
/// JSON into a typed [`MediaProbe`] — the frontend/application layer never
/// sees FFprobe's raw output.
pub struct FfprobeMediaProbeService;

impl FfprobeMediaProbeService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FfprobeMediaProbeService {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct RawProbe {
    #[serde(default)]
    streams: Vec<RawStream>,
    #[serde(default)]
    format: RawFormat,
}

#[derive(Debug, Default, Deserialize)]
struct RawStream {
    codec_type: String,
    codec_name: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
    r_frame_rate: Option<String>,
    bit_rate: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RawFormat {
    duration: Option<String>,
    bit_rate: Option<String>,
    format_name: Option<String>,
}

fn parse_frame_rate(raw: &str) -> Option<f64> {
    let (num, den) = raw.split_once('/')?;
    let num: f64 = num.parse().ok()?;
    let den: f64 = den.parse().ok()?;
    if den == 0.0 {
        None
    } else {
        Some(num / den)
    }
}

#[async_trait]
impl MediaProbeService for FfprobeMediaProbeService {
    async fn probe(&self, path: &Path) -> Result<MediaProbe, MediaError> {
        if !path.is_file() {
            return Err(MediaError::FileNotFound {
                path: path.display().to_string(),
            });
        }

        let binary = resolve_binary("ffprobe").ok_or_else(|| MediaError::FfprobeFailed {
            path: path.display().to_string(),
            detail: "ffprobe is not installed or could not be found".to_string(),
        })?;

        let output = Command::new(&binary)
            // Section 99 quality review: if this task is dropped mid-flight
            // (e.g. an abrupt app shutdown), don't leave ffprobe running.
            .kill_on_drop(true)
            .args([
                "-v",
                "error",
                "-print_format",
                "json",
                "-show_format",
                "-show_streams",
            ])
            .arg(path)
            .output()
            .await
            .map_err(|e| MediaError::FfprobeFailed {
                path: path.display().to_string(),
                detail: e.to_string(),
            })?;

        if !output.status.success() {
            return Err(MediaError::CorruptedVideo {
                path: path.display().to_string(),
            });
        }

        let raw: RawProbe =
            serde_json::from_slice(&output.stdout).map_err(|e| MediaError::FfprobeFailed {
                path: path.display().to_string(),
                detail: format!("could not parse ffprobe output: {e}"),
            })?;

        let video_stream = raw.streams.iter().find(|s| s.codec_type == "video");
        let audio_stream = raw.streams.iter().find(|s| s.codec_type == "audio");

        let duration_ms = raw
            .format
            .duration
            .as_deref()
            .and_then(|s| s.parse::<f64>().ok())
            .map(|secs| (secs * 1000.0).round() as i64);

        let bitrate = raw
            .format
            .bit_rate
            .as_deref()
            .or(video_stream.and_then(|s| s.bit_rate.as_deref()))
            .and_then(|s| s.parse::<i64>().ok());

        Ok(MediaProbe {
            duration_ms,
            width: video_stream.and_then(|s| s.width),
            height: video_stream.and_then(|s| s.height),
            fps: video_stream
                .and_then(|s| s.r_frame_rate.as_deref())
                .and_then(parse_frame_rate),
            video_codec: video_stream.and_then(|s| s.codec_name.clone()),
            audio_codec: audio_stream.and_then(|s| s.codec_name.clone()),
            bitrate,
            has_audio: audio_stream.is_some(),
            container_format: raw.format.format_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fractional_frame_rate() {
        assert_eq!(parse_frame_rate("30000/1001"), Some(30000.0 / 1001.0));
        assert_eq!(parse_frame_rate("30/1"), Some(30.0));
        assert_eq!(parse_frame_rate("30/0"), None);
        assert_eq!(parse_frame_rate("not-a-rate"), None);
    }
}
