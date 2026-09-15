use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::domain::duplicate_match::DuplicateMatch;
use crate::domain::media_error::MediaError;
use crate::domain::ports::hashing::{ContentHashService, PerceptualHashService};
use crate::domain::ports::media_service::{MediaProbe, MediaProbeService, ThumbnailService};
use crate::domain::ports::repositories::{DuplicateMatchRepository, VideoRepository};
use crate::domain::video::Video;
use crate::domain::video_status::{AvailabilityStatus, ValidationStatus};
use crate::infrastructure::filesystem::{extension_lowercase, is_supported_video_extension};
use crate::platform::paths::AppPaths;

/// A near-duplicate match below this similarity is not worth recording —
/// most unrelated thumbnails still share ~50-60% of dHash bits by chance.
const NEAR_DUPLICATE_THRESHOLD: f64 = 0.90;
/// How many of the workspace's most recent videos to compare a new
/// perceptual hash against (section 28 — bounded, not O(library size)).
const NEAR_DUPLICATE_COMPARISON_WINDOW: i64 = 500;

/// What happened when [`MediaIngestionService::ingest_path`] ran. This is
/// the *only* entry point that ever creates a [`Video`] row — manual
/// import, drag-and-drop and the folder watcher all converge here
/// (section 13), so there is exactly one place duplicate/validation/
/// thumbnail logic can diverge.
pub enum IngestOutcome {
    Created(Video),
    /// An identical file (same `content_hash`) already exists in the
    /// library, and its recorded file still exists on disk. Per section
    /// 30, XP FLOW does not create a second record for this by default.
    Duplicate {
        existing: Video,
    },
    /// An identical file was found, but the existing row's file no longer
    /// exists at its recorded path — this is almost certainly the same
    /// file having been moved/renamed (section 32). The existing row's
    /// `file_path` is updated in place rather than creating a new one.
    Moved {
        updated: Video,
    },
    /// This exact path is already indexed (e.g. reconciliation revisiting
    /// a file the watcher already ingested) — a idempotent no-op.
    AlreadyIndexed {
        existing: Video,
    },
    Rejected(MediaError),
}

pub struct MediaIngestionService {
    video_repo: Arc<dyn VideoRepository>,
    duplicate_repo: Arc<dyn DuplicateMatchRepository>,
    probe_service: Arc<dyn MediaProbeService>,
    thumbnail_service: Arc<dyn ThumbnailService>,
    hash_service: Arc<dyn ContentHashService>,
    perceptual_service: Arc<dyn PerceptualHashService>,
    paths: AppPaths,
}

impl MediaIngestionService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        video_repo: Arc<dyn VideoRepository>,
        duplicate_repo: Arc<dyn DuplicateMatchRepository>,
        probe_service: Arc<dyn MediaProbeService>,
        thumbnail_service: Arc<dyn ThumbnailService>,
        hash_service: Arc<dyn ContentHashService>,
        perceptual_service: Arc<dyn PerceptualHashService>,
        paths: AppPaths,
    ) -> Self {
        Self {
            video_repo,
            duplicate_repo,
            probe_service,
            thumbnail_service,
            hash_service,
            perceptual_service,
            paths,
        }
    }

    /// Ingests one already-stable file (the caller — `JobRunner` — is
    /// responsible for file-stability waiting; this service assumes the
    /// file is finished being written).
    pub async fn ingest_path(
        &self,
        workspace_id: Uuid,
        source_id: Uuid,
        channel_id: Option<Uuid>,
        path: &Path,
    ) -> IngestOutcome {
        let path_string = path.to_string_lossy().to_string();

        let Some(extension) = extension_lowercase(path) else {
            return IngestOutcome::Rejected(MediaError::UnsupportedFormat {
                extension: "(none)".to_string(),
            });
        };
        if !is_supported_video_extension(path) {
            return IngestOutcome::Rejected(MediaError::UnsupportedFormat { extension });
        }

        let metadata = match tokio::fs::metadata(path).await {
            Ok(m) => m,
            Err(_) => {
                return IngestOutcome::Rejected(MediaError::FileNotFound { path: path_string })
            }
        };

        // Idempotency (section 58): this exact path is already indexed.
        if let Ok(Some(existing)) = self
            .video_repo
            .get_by_path(workspace_id, &path_string)
            .await
        {
            return IngestOutcome::AlreadyIndexed { existing };
        }

        let probe = match self.probe_service.probe(path).await {
            Ok(probe) => probe,
            Err(err) => return IngestOutcome::Rejected(err),
        };

        let content_hash = match self.hash_service.hash_file(path).await {
            Ok(hash) => hash,
            Err(err) => return IngestOutcome::Rejected(err),
        };

        if let Ok(Some(existing)) = self
            .video_repo
            .get_by_hash(workspace_id, &content_hash)
            .await
        {
            return self
                .resolve_hash_collision(existing, path, &path_string)
                .await;
        }

        let validation_status = classify_validation(&probe);
        let video_id = Uuid::new_v4();

        let thumbnail_path = self.paths.thumbnail_path(video_id);
        let thumbnail_ok = self
            .thumbnail_service
            .generate(path, &thumbnail_path, probe.duration_ms)
            .await
            .is_ok();

        let perceptual_hash = if thumbnail_ok {
            self.perceptual_service
                .hash_image(&thumbnail_path)
                .await
                .ok()
        } else {
            None
        };

        let original_filename = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path_string.clone());
        let display_title = derive_display_title(&original_filename);

        let mut video = Video::new(
            workspace_id,
            source_id,
            channel_id,
            original_filename,
            display_title,
            path_string,
            metadata.len() as i64,
            extension,
        );
        video.id = video_id;
        video.duration_ms = probe.duration_ms;
        video.width = probe.width;
        video.height = probe.height;
        video.fps = probe.fps;
        video.video_codec = probe.video_codec;
        video.audio_codec = probe.audio_codec;
        video.bitrate = probe.bitrate;
        video.has_audio = Some(probe.has_audio);
        video.content_hash = Some(content_hash);
        video.perceptual_hash = perceptual_hash.clone();
        video.thumbnail_path = thumbnail_ok.then(|| thumbnail_path.display().to_string());
        video.validation_status = validation_status;
        video.availability_status = AvailabilityStatus::Available;

        if let Err(err) = self.video_repo.create(&video).await {
            return IngestOutcome::Rejected(MediaError::Unreadable {
                path: video.file_path.clone(),
                detail: err.to_string(),
            });
        }

        if let Some(hash) = perceptual_hash {
            self.record_near_duplicates(workspace_id, video.id, &hash)
                .await;
        }

        IngestOutcome::Created(video)
    }

    async fn resolve_hash_collision(
        &self,
        existing: Video,
        new_path: &Path,
        new_path_string: &str,
    ) -> IngestOutcome {
        let existing_path_exists = Path::new(&existing.file_path).is_file();

        if existing_path_exists {
            return IngestOutcome::Duplicate { existing };
        }

        let mut updated = existing;
        updated.file_path = new_path_string.to_string();
        updated.availability_status = AvailabilityStatus::Available;
        updated.last_seen_at = Utc::now();
        updated.updated_at = Utc::now();

        if self.video_repo.update(&updated).await.is_err() {
            return IngestOutcome::Rejected(MediaError::Unreadable {
                path: new_path.display().to_string(),
                detail: "failed to update moved video record".to_string(),
            });
        }

        IngestOutcome::Moved { updated }
    }

    async fn record_near_duplicates(
        &self,
        workspace_id: Uuid,
        video_id: Uuid,
        perceptual_hash: &str,
    ) {
        let Ok(recent) = self
            .video_repo
            .list_recent_perceptual_hashes(workspace_id, NEAR_DUPLICATE_COMPARISON_WINDOW)
            .await
        else {
            return;
        };

        for (other_id, other_hash) in recent {
            if other_id == video_id {
                continue;
            }
            let similarity = self
                .perceptual_service
                .similarity(perceptual_hash, &other_hash);
            if similarity >= NEAR_DUPLICATE_THRESHOLD {
                let duplicate = DuplicateMatch::possible(video_id, other_id, similarity);
                let _ = self.duplicate_repo.create(&duplicate).await;
            }
        }
    }

    pub async fn regenerate_thumbnail(&self, video: &Video) -> Result<PathBuf, MediaError> {
        let thumbnail_path = self.paths.thumbnail_path(video.id);
        self.thumbnail_service
            .generate(
                Path::new(&video.file_path),
                &thumbnail_path,
                video.duration_ms,
            )
            .await?;
        Ok(thumbnail_path)
    }

    pub async fn revalidate(&self, video: &mut Video) -> Result<(), MediaError> {
        if !Path::new(&video.file_path).is_file() {
            video.availability_status = AvailabilityStatus::Missing;
            video.validation_status = ValidationStatus::Missing;
            return Ok(());
        }

        let probe = self
            .probe_service
            .probe(Path::new(&video.file_path))
            .await?;
        video.validation_status = classify_validation(&probe);
        video.duration_ms = probe.duration_ms;
        video.width = probe.width;
        video.height = probe.height;
        video.fps = probe.fps;
        video.video_codec = probe.video_codec;
        video.audio_codec = probe.audio_codec;
        video.bitrate = probe.bitrate;
        video.has_audio = Some(probe.has_audio);
        video.availability_status = AvailabilityStatus::Available;
        Ok(())
    }
}

fn classify_validation(probe: &MediaProbe) -> ValidationStatus {
    let has_dimensions = probe.width.is_some_and(|w| w > 0) && probe.height.is_some_and(|h| h > 0);
    let has_duration = probe.duration_ms.is_some_and(|d| d > 0);

    if has_dimensions && has_duration {
        ValidationStatus::Valid
    } else {
        ValidationStatus::Invalid
    }
}

/// Best-effort, deliberately simple filename -> title heuristic (section
/// 80): underscores/hyphens become spaces, whitespace is collapsed, and
/// only the very first letter is capitalized (sentence case) — there is no
/// attempt at proper-noun detection or NLP. `original_filename` is always
/// preserved separately, and `display_title` remains user-editable.
fn derive_display_title(filename: &str) -> String {
    let stem = Path::new(filename)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| filename.to_string());

    let normalized = stem
        .chars()
        .map(|c| if c == '_' || c == '-' { ' ' } else { c })
        .collect::<String>();

    let collapsed = normalized.split_whitespace().collect::<Vec<_>>().join(" ");

    let mut chars = collapsed.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => filename.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_a_readable_title_from_a_snake_case_filename() {
        assert_eq!(
            derive_display_title("neymar_fala_sobre_messi_001.mp4"),
            "Neymar fala sobre messi 001"
        );
    }

    #[test]
    fn collapses_repeated_separators_and_whitespace() {
        assert_eq!(
            derive_display_title("clip__final---v2.mov"),
            "Clip final v2"
        );
    }

    #[test]
    fn preserves_already_readable_titles() {
        assert_eq!(derive_display_title("Q3 Highlights.mkv"), "Q3 Highlights");
    }

    #[test]
    fn classifies_a_clean_probe_as_valid() {
        let probe = MediaProbe {
            duration_ms: Some(42_000),
            width: Some(1080),
            height: Some(1920),
            ..Default::default()
        };
        assert_eq!(classify_validation(&probe), ValidationStatus::Valid);
    }

    #[test]
    fn classifies_a_zero_duration_probe_as_invalid() {
        let probe = MediaProbe {
            duration_ms: Some(0),
            width: Some(1080),
            height: Some(1920),
            ..Default::default()
        };
        assert_eq!(classify_validation(&probe), ValidationStatus::Invalid);
    }

    #[test]
    fn classifies_a_probe_with_no_video_stream_as_invalid() {
        let probe = MediaProbe {
            duration_ms: Some(42_000),
            width: None,
            height: None,
            ..Default::default()
        };
        assert_eq!(classify_validation(&probe), ValidationStatus::Invalid);
    }
}
