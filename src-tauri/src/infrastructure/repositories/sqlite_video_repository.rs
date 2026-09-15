use async_trait::async_trait;
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};
use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::ports::repositories::VideoRepository;
use crate::domain::video::Video;
use crate::domain::video_query::{
    ChannelFilter, DuplicateFilter, VideoLibrarySummary, VideoListQuery, VideoPage, VideoSort,
};
use crate::domain::video_status::{
    AvailabilityStatus, Orientation, ValidationStatus, VideoPriority,
};

use super::parse_dt;

pub struct SqliteVideoRepository {
    pool: SqlitePool,
}

impl SqliteVideoRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn map_repo_err(err: sqlx::Error) -> DomainError {
    DomainError::Repository(err.to_string())
}

const SELECT_COLUMNS: &str = "id, workspace_id, channel_id, source_id, original_filename, display_title, \
     file_path, file_size_bytes, extension, duration_ms, width, height, fps, video_codec, audio_codec, \
     bitrate, has_audio, content_hash, perceptual_hash, thumbnail_path, validation_status, \
     availability_status, duplicate_of, priority, notes, archived, created_at, imported_at, last_seen_at, updated_at";

fn row_to_video(row: &sqlx::sqlite::SqliteRow) -> Result<Video, DomainError> {
    let channel_id: Option<String> = row.try_get("channel_id").map_err(map_repo_err)?;
    let duplicate_of: Option<String> = row.try_get("duplicate_of").map_err(map_repo_err)?;
    let has_audio: Option<i64> = row.try_get("has_audio").map_err(map_repo_err)?;

    Ok(Video {
        id: parse_uuid(&row.try_get::<String, _>("id").map_err(map_repo_err)?),
        workspace_id: parse_uuid(
            &row.try_get::<String, _>("workspace_id")
                .map_err(map_repo_err)?,
        ),
        channel_id: channel_id.map(|s| parse_uuid(&s)),
        source_id: parse_uuid(
            &row.try_get::<String, _>("source_id")
                .map_err(map_repo_err)?,
        ),
        original_filename: row.try_get("original_filename").map_err(map_repo_err)?,
        display_title: row.try_get("display_title").map_err(map_repo_err)?,
        file_path: row.try_get("file_path").map_err(map_repo_err)?,
        file_size_bytes: row.try_get("file_size_bytes").map_err(map_repo_err)?,
        extension: row.try_get("extension").map_err(map_repo_err)?,
        duration_ms: row.try_get("duration_ms").map_err(map_repo_err)?,
        width: row.try_get("width").map_err(map_repo_err)?,
        height: row.try_get("height").map_err(map_repo_err)?,
        fps: row.try_get("fps").map_err(map_repo_err)?,
        video_codec: row.try_get("video_codec").map_err(map_repo_err)?,
        audio_codec: row.try_get("audio_codec").map_err(map_repo_err)?,
        bitrate: row.try_get("bitrate").map_err(map_repo_err)?,
        has_audio: has_audio.map(|v| v != 0),
        content_hash: row.try_get("content_hash").map_err(map_repo_err)?,
        perceptual_hash: row.try_get("perceptual_hash").map_err(map_repo_err)?,
        thumbnail_path: row.try_get("thumbnail_path").map_err(map_repo_err)?,
        validation_status: row
            .try_get::<String, _>("validation_status")
            .map_err(map_repo_err)?
            .parse::<ValidationStatus>()
            .map_err(DomainError::Validation)?,
        availability_status: row
            .try_get::<String, _>("availability_status")
            .map_err(map_repo_err)?
            .parse::<AvailabilityStatus>()
            .map_err(DomainError::Validation)?,
        duplicate_of: duplicate_of.map(|s| parse_uuid(&s)),
        priority: row
            .try_get::<String, _>("priority")
            .map_err(map_repo_err)?
            .parse::<VideoPriority>()
            .map_err(DomainError::Validation)?,
        notes: row.try_get("notes").map_err(map_repo_err)?,
        archived: row.try_get::<i64, _>("archived").map_err(map_repo_err)? != 0,
        created_at: parse_dt(
            &row.try_get::<String, _>("created_at")
                .map_err(map_repo_err)?,
        ),
        imported_at: parse_dt(
            &row.try_get::<String, _>("imported_at")
                .map_err(map_repo_err)?,
        ),
        last_seen_at: parse_dt(
            &row.try_get::<String, _>("last_seen_at")
                .map_err(map_repo_err)?,
        ),
        updated_at: parse_dt(
            &row.try_get::<String, _>("updated_at")
                .map_err(map_repo_err)?,
        ),
    })
}

fn parse_uuid(s: &str) -> Uuid {
    Uuid::parse_str(s).unwrap_or_default()
}

#[async_trait]
impl VideoRepository for SqliteVideoRepository {
    async fn create(&self, video: &Video) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO videos (id, workspace_id, channel_id, source_id, original_filename, display_title, \
             file_path, file_size_bytes, extension, duration_ms, width, height, fps, video_codec, audio_codec, \
             bitrate, has_audio, content_hash, perceptual_hash, thumbnail_path, validation_status, \
             availability_status, duplicate_of, priority, notes, archived, created_at, imported_at, last_seen_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(video.id.to_string())
        .bind(video.workspace_id.to_string())
        .bind(video.channel_id.map(|id| id.to_string()))
        .bind(video.source_id.to_string())
        .bind(&video.original_filename)
        .bind(&video.display_title)
        .bind(&video.file_path)
        .bind(video.file_size_bytes)
        .bind(&video.extension)
        .bind(video.duration_ms)
        .bind(video.width)
        .bind(video.height)
        .bind(video.fps)
        .bind(&video.video_codec)
        .bind(&video.audio_codec)
        .bind(video.bitrate)
        .bind(video.has_audio.map(|b| b as i64))
        .bind(&video.content_hash)
        .bind(&video.perceptual_hash)
        .bind(&video.thumbnail_path)
        .bind(video.validation_status.as_str())
        .bind(video.availability_status.as_str())
        .bind(video.duplicate_of.map(|id| id.to_string()))
        .bind(video.priority.as_str())
        .bind(&video.notes)
        .bind(video.archived as i64)
        .bind(video.created_at.to_rfc3339())
        .bind(video.imported_at.to_rfc3339())
        .bind(video.last_seen_at.to_rfc3339())
        .bind(video.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn update(&self, video: &Video) -> DomainResult<()> {
        sqlx::query(
            "UPDATE videos SET channel_id = ?, display_title = ?, file_path = ?, file_size_bytes = ?, \
             duration_ms = ?, width = ?, height = ?, fps = ?, video_codec = ?, audio_codec = ?, bitrate = ?, \
             has_audio = ?, content_hash = ?, perceptual_hash = ?, thumbnail_path = ?, validation_status = ?, \
             availability_status = ?, duplicate_of = ?, priority = ?, notes = ?, archived = ?, \
             last_seen_at = ?, updated_at = ? WHERE id = ?",
        )
        .bind(video.channel_id.map(|id| id.to_string()))
        .bind(&video.display_title)
        .bind(&video.file_path)
        .bind(video.file_size_bytes)
        .bind(video.duration_ms)
        .bind(video.width)
        .bind(video.height)
        .bind(video.fps)
        .bind(&video.video_codec)
        .bind(&video.audio_codec)
        .bind(video.bitrate)
        .bind(video.has_audio.map(|b| b as i64))
        .bind(&video.content_hash)
        .bind(&video.perceptual_hash)
        .bind(&video.thumbnail_path)
        .bind(video.validation_status.as_str())
        .bind(video.availability_status.as_str())
        .bind(video.duplicate_of.map(|id| id.to_string()))
        .bind(video.priority.as_str())
        .bind(&video.notes)
        .bind(video.archived as i64)
        .bind(video.last_seen_at.to_rfc3339())
        .bind(video.updated_at.to_rfc3339())
        .bind(video.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_repo_err)?;
        Ok(())
    }

    async fn get(&self, id: Uuid) -> DomainResult<Option<Video>> {
        let row = sqlx::query(&format!("SELECT {SELECT_COLUMNS} FROM videos WHERE id = ?"))
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_repo_err)?;
        row.as_ref().map(row_to_video).transpose()
    }

    async fn get_by_hash(
        &self,
        workspace_id: Uuid,
        content_hash: &str,
    ) -> DomainResult<Option<Video>> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM videos WHERE workspace_id = ? AND content_hash = ? LIMIT 1"
        ))
        .bind(workspace_id.to_string())
        .bind(content_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_repo_err)?;
        row.as_ref().map(row_to_video).transpose()
    }

    async fn get_by_path(
        &self,
        workspace_id: Uuid,
        file_path: &str,
    ) -> DomainResult<Option<Video>> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM videos WHERE workspace_id = ? AND file_path = ? LIMIT 1"
        ))
        .bind(workspace_id.to_string())
        .bind(file_path)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_repo_err)?;
        row.as_ref().map(row_to_video).transpose()
    }

    async fn list_for_workspace(&self, workspace_id: Uuid) -> DomainResult<Vec<Video>> {
        let rows = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM videos WHERE workspace_id = ? ORDER BY imported_at DESC"
        ))
        .bind(workspace_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;
        rows.iter().map(row_to_video).collect()
    }

    async fn list_paths_for_source(&self, source_id: Uuid) -> DomainResult<Vec<(Uuid, String)>> {
        let rows =
            sqlx::query("SELECT id, file_path FROM videos WHERE source_id = ? AND archived = 0")
                .bind(source_id.to_string())
                .fetch_all(&self.pool)
                .await
                .map_err(map_repo_err)?;

        rows.iter()
            .map(|row| {
                Ok((
                    parse_uuid(&row.try_get::<String, _>("id").map_err(map_repo_err)?),
                    row.try_get::<String, _>("file_path")
                        .map_err(map_repo_err)?,
                ))
            })
            .collect()
    }

    async fn list_recent_perceptual_hashes(
        &self,
        workspace_id: Uuid,
        limit: i64,
    ) -> DomainResult<Vec<(Uuid, String)>> {
        let rows = sqlx::query(
            "SELECT id, perceptual_hash FROM videos \
             WHERE workspace_id = ? AND perceptual_hash IS NOT NULL AND archived = 0 \
             ORDER BY imported_at DESC LIMIT ?",
        )
        .bind(workspace_id.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_repo_err)?;

        rows.iter()
            .map(|row| {
                Ok((
                    parse_uuid(&row.try_get::<String, _>("id").map_err(map_repo_err)?),
                    row.try_get::<String, _>("perceptual_hash")
                        .map_err(map_repo_err)?,
                ))
            })
            .collect()
    }

    async fn list_paginated(&self, query: &VideoListQuery) -> DomainResult<VideoPage> {
        let mut count_builder: QueryBuilder<Sqlite> =
            QueryBuilder::new("SELECT COUNT(*) FROM videos WHERE 1 = 1");
        apply_filters(&mut count_builder, query);
        let total: i64 = count_builder
            .build_query_scalar()
            .fetch_one(&self.pool)
            .await
            .map_err(map_repo_err)?;

        let mut builder: QueryBuilder<Sqlite> =
            QueryBuilder::new(format!("SELECT {SELECT_COLUMNS} FROM videos WHERE 1 = 1"));
        apply_filters(&mut builder, query);

        let order_by = match query.sort {
            VideoSort::NewestImported => " ORDER BY imported_at DESC",
            VideoSort::OldestImported => " ORDER BY imported_at ASC",
            VideoSort::Filename => " ORDER BY original_filename ASC",
            VideoSort::Duration => " ORDER BY duration_ms DESC",
            VideoSort::FileSize => " ORDER BY file_size_bytes DESC",
            VideoSort::Channel => " ORDER BY channel_id ASC, imported_at DESC",
        };
        builder.push(order_by);
        builder.push(" LIMIT ").push_bind(query.page_size);
        builder
            .push(" OFFSET ")
            .push_bind(query.page * query.page_size);

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(map_repo_err)?;
        let items = rows
            .iter()
            .map(row_to_video)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(VideoPage {
            items,
            total,
            page: query.page,
            page_size: query.page_size,
        })
    }

    async fn summary(&self, workspace_id: Uuid) -> DomainResult<VideoLibrarySummary> {
        let row = sqlx::query(
            "SELECT \
                COUNT(*) FILTER (WHERE archived = 0) AS total, \
                COUNT(*) FILTER (WHERE archived = 0 AND validation_status = 'valid' AND availability_status = 'available') AS ready, \
                COUNT(*) FILTER (WHERE archived = 0 AND validation_status IN ('pending', 'validating')) AS processing, \
                COUNT(*) FILTER (WHERE archived = 0 AND id IN (SELECT video_id FROM duplicate_matches)) AS duplicates, \
                COUNT(*) FILTER (WHERE archived = 0 AND validation_status IN ('invalid', 'unsupported', 'corrupted')) AS invalid, \
                COUNT(*) FILTER (WHERE archived = 0 AND availability_status != 'available') AS missing, \
                COUNT(*) FILTER (WHERE archived = 1) AS archived, \
                COUNT(*) FILTER (WHERE archived = 0 AND channel_id IS NULL) AS unassigned \
             FROM videos WHERE workspace_id = ?",
        )
        .bind(workspace_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(map_repo_err)?;

        Ok(VideoLibrarySummary {
            total: row.try_get("total").map_err(map_repo_err)?,
            ready: row.try_get("ready").map_err(map_repo_err)?,
            processing: row.try_get("processing").map_err(map_repo_err)?,
            duplicates: row.try_get("duplicates").map_err(map_repo_err)?,
            invalid: row.try_get("invalid").map_err(map_repo_err)?,
            missing: row.try_get("missing").map_err(map_repo_err)?,
            archived: row.try_get("archived").map_err(map_repo_err)?,
            unassigned: row.try_get("unassigned").map_err(map_repo_err)?,
        })
    }

    async fn delete(&self, id: Uuid) -> DomainResult<()> {
        sqlx::query("DELETE FROM videos WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(map_repo_err)?;
        Ok(())
    }
}

fn apply_filters(builder: &mut QueryBuilder<Sqlite>, query: &VideoListQuery) {
    builder
        .push(" AND workspace_id = ")
        .push_bind(query.workspace_id.to_string());

    if !query.include_archived {
        builder.push(" AND archived = 0");
    }

    if let Some(search) = query.search.as_deref().filter(|s| !s.trim().is_empty()) {
        let pattern = format!("%{}%", search.trim().replace('%', "\\%"));
        builder
            .push(" AND (display_title LIKE ")
            .push_bind(pattern.clone());
        builder
            .push(" ESCAPE '\\' OR original_filename LIKE ")
            .push_bind(pattern.clone());
        builder
            .push(" ESCAPE '\\' OR notes LIKE ")
            .push_bind(pattern);
        builder.push(" ESCAPE '\\')");
    }

    match query.channel {
        Some(ChannelFilter::Any(id)) => {
            builder.push(" AND channel_id = ").push_bind(id.to_string());
        }
        Some(ChannelFilter::Unassigned) => {
            builder.push(" AND channel_id IS NULL");
        }
        None => {}
    }

    if let Some(source_id) = query.source_id {
        builder
            .push(" AND source_id = ")
            .push_bind(source_id.to_string());
    }
    if let Some(status) = query.validation_status {
        builder
            .push(" AND validation_status = ")
            .push_bind(status.as_str());
    }
    if let Some(status) = query.availability_status {
        builder
            .push(" AND availability_status = ")
            .push_bind(status.as_str());
    }
    if let Some(priority) = query.priority {
        builder
            .push(" AND priority = ")
            .push_bind(priority.as_str());
    }
    if let Some(orientation) = query.orientation {
        match orientation {
            Orientation::Vertical => {
                builder.push(" AND width IS NOT NULL AND height IS NOT NULL AND width < height")
            }
            Orientation::Square => {
                builder.push(" AND width IS NOT NULL AND height IS NOT NULL AND width = height")
            }
            Orientation::Landscape => {
                builder.push(" AND width IS NOT NULL AND height IS NOT NULL AND width > height")
            }
        };
    }
    if let Some(DuplicateFilter::PossibleDuplicates) = query.duplicate {
        builder.push(" AND id IN (SELECT video_id FROM duplicate_matches)");
    }
}
