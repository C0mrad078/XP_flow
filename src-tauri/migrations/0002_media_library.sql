-- Phase 2: media library, folder sources, duplicate detection, jobs.
--
-- videos/video_sources are replaced (not ALTERed) because their Phase 1
-- shape ("VideoSource" = one row per video's origin) and Phase 2 shape
-- ("VideoSource" = a watched folder configuration, per section 8 of the
-- brief) are different enough that ALTER TABLE churn would be harder to
-- read than a clean recreate — and no production data exists yet to
-- migrate. SQLite does not enforce foreign-key referential integrity
-- during DDL, so dropping the referenced `videos` table here is safe even
-- though `publications.video_id` still names it.

DROP TABLE IF EXISTS video_sources;
DROP TABLE IF EXISTS videos;

CREATE TABLE video_sources (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK (source_type IN ('cutpro_folder', 'manual_import', 'watch_folder')),
    folder_path TEXT,
    channel_id TEXT REFERENCES channels(id) ON DELETE SET NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    recursive INTEGER NOT NULL DEFAULT 0,
    watch_enabled INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_scan_at TEXT,
    last_error TEXT
);
CREATE INDEX idx_video_sources_workspace_id ON video_sources(workspace_id);
CREATE INDEX idx_video_sources_channel_id ON video_sources(channel_id);

CREATE TABLE videos (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    channel_id TEXT REFERENCES channels(id) ON DELETE SET NULL,
    source_id TEXT NOT NULL REFERENCES video_sources(id) ON DELETE RESTRICT,

    original_filename TEXT NOT NULL,
    display_title TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_size_bytes INTEGER NOT NULL,
    extension TEXT NOT NULL,

    duration_ms INTEGER,
    width INTEGER,
    height INTEGER,
    fps REAL,
    video_codec TEXT,
    audio_codec TEXT,
    bitrate INTEGER,
    has_audio INTEGER,

    content_hash TEXT,
    perceptual_hash TEXT,
    thumbnail_path TEXT,

    validation_status TEXT NOT NULL CHECK (validation_status IN (
        'pending', 'validating', 'valid', 'invalid', 'unsupported', 'corrupted', 'missing'
    )) DEFAULT 'pending',
    availability_status TEXT NOT NULL CHECK (availability_status IN (
        'available', 'missing', 'moved', 'offline_volume', 'permission_denied'
    )) DEFAULT 'available',
    duplicate_of TEXT REFERENCES videos(id) ON DELETE SET NULL,
    priority TEXT NOT NULL CHECK (priority IN ('low', 'normal', 'high', 'urgent')) DEFAULT 'normal',
    notes TEXT,
    archived INTEGER NOT NULL DEFAULT 0,

    created_at TEXT NOT NULL,
    imported_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,

    UNIQUE (workspace_id, file_path)
);
CREATE INDEX idx_videos_workspace_id ON videos(workspace_id);
CREATE INDEX idx_videos_channel_id ON videos(channel_id);
CREATE INDEX idx_videos_source_id ON videos(source_id);
CREATE INDEX idx_videos_content_hash ON videos(content_hash);
CREATE INDEX idx_videos_validation_status ON videos(validation_status);
CREATE INDEX idx_videos_availability_status ON videos(availability_status);
CREATE INDEX idx_videos_imported_at ON videos(imported_at);
CREATE INDEX idx_videos_archived ON videos(archived);

-- Near-duplicate matches only (exact duplicates are never persisted as a
-- second video row — see domain::duplicate_match).
CREATE TABLE duplicate_matches (
    id TEXT PRIMARY KEY NOT NULL,
    video_id TEXT NOT NULL REFERENCES videos(id) ON DELETE CASCADE,
    matched_video_id TEXT NOT NULL REFERENCES videos(id) ON DELETE CASCADE,
    similarity REAL NOT NULL,
    match_type TEXT NOT NULL CHECK (match_type IN ('exact', 'possible')),
    created_at TEXT NOT NULL,
    UNIQUE (video_id, matched_video_id)
);
CREATE INDEX idx_duplicate_matches_video_id ON duplicate_matches(video_id);

-- Background job persistence (sections 57/58/84/85). The partial unique
-- index is what makes enqueueing idempotent: only one pending/running job
-- may exist per dedupe_key at a time.
CREATE TABLE jobs (
    id TEXT PRIMARY KEY NOT NULL,
    job_type TEXT NOT NULL CHECK (job_type IN (
        'validate_video', 'publish_video', 'collect_metrics', 'fetch_comments',
        'generate_thumbnail', 'backup', 'cleanup', 'scan_folder', 'ingest_video', 'reconcile_source'
    )),
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'succeeded', 'failed', 'cancelled')) DEFAULT 'pending',
    payload_json TEXT NOT NULL DEFAULT '{}',
    dedupe_key TEXT,
    created_at TEXT NOT NULL,
    started_at TEXT,
    completed_at TEXT,
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT
);
CREATE INDEX idx_jobs_status ON jobs(status);
CREATE UNIQUE INDEX idx_jobs_dedupe_active ON jobs(dedupe_key)
    WHERE dedupe_key IS NOT NULL AND status IN ('pending', 'running');
