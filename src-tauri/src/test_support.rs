//! Shared test-only fakes and fixtures (section 91/92 of the Phase 2
//! brief). FFmpeg/FFprobe are not guaranteed to be installed wherever
//! these tests run, so `MediaIngestionService`'s ingestion pipeline is
//! exercised here against fake `MediaProbeService`/`ThumbnailService`/
//! `ContentHashService`/`PerceptualHashService` implementations instead of
//! the real FFmpeg-backed ones — exactly what those ports exist for.
//! `cargo test` never shells out to a media tool.
#![cfg(test)]

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use async_trait::async_trait;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::domain::media_error::MediaError;
use crate::domain::ports::hashing::{ContentHashService, PerceptualHashService};
use crate::domain::ports::media_service::{MediaProbe, MediaProbeService, ThumbnailService};

pub fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("xpflow-test-{label}-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

pub async fn temp_pool(label: &str) -> SqlitePool {
    let dir = temp_dir(label);
    crate::persistence::db::init_pool(&dir.join("test.db"))
        .await
        .unwrap()
}

/// Inserts the minimum parent rows the `videos` table's foreign keys
/// require (a workspace and a video source), returning their ids.
pub async fn seed_workspace_and_source(pool: &SqlitePool) -> (Uuid, Uuid) {
    let workspace_id = Uuid::new_v4();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO workspaces (id, name, created_at, updated_at) VALUES (?, ?, ?, ?)")
        .bind(workspace_id.to_string())
        .bind("Test Workspace")
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .unwrap();

    let source_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO video_sources (id, workspace_id, name, source_type, folder_path, enabled, recursive, watch_enabled, created_at, updated_at) \
         VALUES (?, ?, 'Manual Import', 'manual_import', NULL, 1, 0, 0, ?, ?)",
    )
    .bind(source_id.to_string())
    .bind(workspace_id.to_string())
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();

    (workspace_id, source_id)
}

/// Inserts a channel row for the given workspace, returning its id.
pub async fn seed_channel(pool: &SqlitePool, workspace_id: Uuid, name: &str) -> Uuid {
    let channel_id = Uuid::new_v4();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO channels (id, workspace_id, name, created_at, updated_at) VALUES (?, ?, ?, ?, ?)")
        .bind(channel_id.to_string())
        .bind(workspace_id.to_string())
        .bind(name)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .unwrap();
    channel_id
}

/// Inserts a minimal, valid `videos` row (bypassing the ingestion
/// pipeline entirely) for tests that only care about queue/scheduler
/// behavior downstream of "a video exists".
pub async fn seed_video(
    pool: &SqlitePool,
    workspace_id: Uuid,
    source_id: Uuid,
    channel_id: Option<Uuid>,
    title: &str,
) -> Uuid {
    let video = crate::domain::video::Video::new(
        workspace_id,
        source_id,
        channel_id,
        format!("{title}.mp4"),
        title,
        format!("/tmp/{title}.mp4"),
        1024,
        "mp4",
    );
    let repo = crate::infrastructure::repositories::SqliteVideoRepository::new(pool.clone());
    use crate::domain::ports::repositories::VideoRepository;
    repo.create(&video).await.unwrap();
    video.id
}

/// Overwrites the workspace's timezone directly (bypassing
/// `WorkspaceService::update_timezone`'s validation) for tests that need a
/// specific IANA zone already in place.
pub async fn set_workspace_timezone(pool: &SqlitePool, workspace_id: Uuid, timezone: &str) {
    sqlx::query("UPDATE workspaces SET timezone = ? WHERE id = ?")
        .bind(timezone)
        .bind(workspace_id.to_string())
        .execute(pool)
        .await
        .unwrap();
}

/// Writes a tiny fake "video" file (real content doesn't matter — the
/// fake probe/hash services below never actually decode it).
pub fn write_fake_video(dir: &Path, filename: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(filename);
    std::fs::write(&path, bytes).unwrap();
    path
}

/// Inserts a minimal, valid `publications` row (Imported status) for
/// Phase 5 tests that need a real publication_id to satisfy a foreign
/// key — the queue/scheduler seams this bypasses aren't what's under
/// test in those cases.
pub async fn seed_publication(
    pool: &SqlitePool,
    workspace_id: Uuid,
    channel_id: Uuid,
    video_id: Uuid,
) -> Uuid {
    use crate::domain::platform::Platform;
    use crate::domain::ports::repositories::PublicationRepository;
    use crate::domain::publication::Publication;
    use crate::domain::video_status::VideoPriority;

    let publication = Publication::new(
        workspace_id,
        video_id,
        channel_id,
        Platform::YouTube,
        "Test publication",
        VideoPriority::Normal,
    );
    let repo = crate::infrastructure::repositories::SqlitePublicationRepository::new(pool.clone());
    repo.create(&publication).await.unwrap();
    publication.id
}

/// Inserts a minimal `platform_accounts` row for tests that need a real
/// platform_account_id to satisfy a foreign key.
pub async fn seed_platform_account(
    pool: &SqlitePool,
    workspace_id: Uuid,
    channel_id: Uuid,
) -> Uuid {
    let account = crate::domain::platform_account::PlatformAccount::new(
        workspace_id,
        channel_id,
        crate::domain::platform::Platform::YouTube,
    );
    use crate::domain::ports::repositories::PlatformAccountRepository;
    let repo =
        crate::infrastructure::repositories::SqlitePlatformAccountRepository::new(pool.clone());
    repo.create(&account).await.unwrap();
    account.id
}

/// Returns a fixed, valid-looking probe result for every file — configure
/// per-path overrides via `set_probe` for invalid/corrupted cases.
pub struct FakeProbeService {
    overrides: Mutex<std::collections::HashMap<PathBuf, Result<MediaProbe, String>>>,
}

impl Default for FakeProbeService {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeProbeService {
    pub fn new() -> Self {
        Self {
            overrides: Mutex::new(std::collections::HashMap::new()),
        }
    }

    pub fn set_probe(&self, path: &Path, probe: MediaProbe) {
        self.overrides
            .lock()
            .unwrap()
            .insert(path.to_path_buf(), Ok(probe));
    }
}

fn default_probe() -> MediaProbe {
    MediaProbe {
        duration_ms: Some(42_000),
        width: Some(1080),
        height: Some(1920),
        fps: Some(30.0),
        video_codec: Some("h264".to_string()),
        audio_codec: Some("aac".to_string()),
        bitrate: Some(4_000_000),
        has_audio: true,
        container_format: Some("mov,mp4,m4a,3gp,3g2,mj2".to_string()),
    }
}

#[async_trait]
impl MediaProbeService for FakeProbeService {
    async fn probe(&self, path: &Path) -> Result<MediaProbe, MediaError> {
        if !path.is_file() {
            return Err(MediaError::FileNotFound {
                path: path.display().to_string(),
            });
        }
        match self.overrides.lock().unwrap().get(path) {
            Some(Ok(probe)) => Ok(probe.clone()),
            Some(Err(_)) => Err(MediaError::CorruptedVideo {
                path: path.display().to_string(),
            }),
            None => Ok(default_probe()),
        }
    }
}

pub struct FakeThumbnailService;

#[async_trait]
impl ThumbnailService for FakeThumbnailService {
    async fn generate(
        &self,
        _video_path: &Path,
        output_path: &Path,
        _duration_ms: Option<i64>,
    ) -> Result<(), MediaError> {
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(output_path, b"fake-thumbnail").map_err(|e| MediaError::ThumbnailFailed {
            path: output_path.display().to_string(),
            detail: e.to_string(),
        })
    }
}

/// Hashes by *file path* rather than content, so tests can control exact
/// duplicate/near-duplicate scenarios deterministically by writing files
/// with the hash they want baked in via `hash_for`.
pub struct FakeHashService {
    forced: Mutex<std::collections::HashMap<PathBuf, String>>,
}

impl Default for FakeHashService {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeHashService {
    pub fn new() -> Self {
        Self {
            forced: Mutex::new(std::collections::HashMap::new()),
        }
    }

    pub fn hash_for(&self, path: &Path, hash: impl Into<String>) {
        self.forced
            .lock()
            .unwrap()
            .insert(path.to_path_buf(), hash.into());
    }
}

#[async_trait]
impl ContentHashService for FakeHashService {
    async fn hash_file(&self, path: &Path) -> Result<String, MediaError> {
        if let Some(hash) = self.forced.lock().unwrap().get(path) {
            return Ok(hash.clone());
        }
        // Default: unique per path so unrelated files never collide by accident.
        Ok(format!(
            "{:x}",
            md5_like(path.display().to_string().as_bytes())
        ))
    }
}

/// Not a real hash algorithm — just a fast, deterministic path->string
/// mixer so unrelated fixture files get distinct default hashes.
fn md5_like(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 1469598103934665603;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

pub struct FakePerceptualHashService;

#[async_trait]
impl PerceptualHashService for FakePerceptualHashService {
    async fn hash_image(&self, _image_path: &Path) -> Result<String, MediaError> {
        Ok("0000000000000000".to_string())
    }

    fn similarity(&self, hash_a: &str, hash_b: &str) -> f64 {
        if hash_a == hash_b {
            1.0
        } else {
            0.0
        }
    }
}
