-- XP FLOW foundational schema (Phase 1).
-- IDs are UUID v4 stored as TEXT. Timestamps are ISO-8601 UTC strings
-- (converted to the user's local timezone only at the presentation layer).

CREATE TABLE workspaces (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE channels (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    niche TEXT,
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_channels_workspace_id ON channels(workspace_id);

CREATE TABLE platform_accounts (
    id TEXT PRIMARY KEY NOT NULL,
    channel_id TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    platform TEXT NOT NULL CHECK (platform IN ('youtube', 'tiktok', 'kwai')),
    display_name TEXT,
    status TEXT NOT NULL CHECK (status IN ('not_connected', 'connected', 'auth_expired', 'error')) DEFAULT 'not_connected',
    external_account_id TEXT,
    connected_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (channel_id, platform)
);
CREATE INDEX idx_platform_accounts_channel_id ON platform_accounts(channel_id);

CREATE TABLE videos (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    duration_seconds INTEGER,
    file_path TEXT NOT NULL,
    file_size_bytes INTEGER,
    checksum TEXT,
    imported_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_videos_workspace_id ON videos(workspace_id);

CREATE TABLE video_sources (
    id TEXT PRIMARY KEY NOT NULL,
    video_id TEXT NOT NULL REFERENCES videos(id) ON DELETE CASCADE,
    source_type TEXT NOT NULL CHECK (source_type IN ('local_file', 'cut_pro_export', 'manual_upload')),
    origin_path TEXT NOT NULL,
    metadata_json TEXT,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_video_sources_video_id ON video_sources(video_id);

CREATE TABLE publications (
    id TEXT PRIMARY KEY NOT NULL,
    video_id TEXT NOT NULL REFERENCES videos(id) ON DELETE CASCADE,
    channel_id TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    platform_account_id TEXT REFERENCES platform_accounts(id) ON DELETE SET NULL,
    platform TEXT NOT NULL CHECK (platform IN ('youtube', 'tiktok', 'kwai')),
    status TEXT NOT NULL CHECK (status IN (
        'imported', 'validating', 'ready', 'queued', 'scheduled', 'uploading', 'processing', 'published',
        'failed', 'retry_wait', 'auth_required', 'rate_limited', 'blocked', 'paused', 'cancelled', 'archived', 'duplicate'
    )) DEFAULT 'imported',
    title TEXT NOT NULL,
    description TEXT,
    hashtags_json TEXT NOT NULL DEFAULT '[]',
    scheduled_at TEXT,
    published_at TEXT,
    remote_id TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_publications_video_id ON publications(video_id);
CREATE INDEX idx_publications_channel_id ON publications(channel_id);
CREATE INDEX idx_publications_status ON publications(status);

CREATE TABLE queue_items (
    id TEXT PRIMARY KEY NOT NULL,
    publication_id TEXT NOT NULL REFERENCES publications(id) ON DELETE CASCADE,
    priority INTEGER NOT NULL DEFAULT 0,
    position INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_queue_items_publication_id ON queue_items(publication_id);

CREATE TABLE schedule_slots (
    id TEXT PRIMARY KEY NOT NULL,
    channel_id TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    platform TEXT NOT NULL CHECK (platform IN ('youtube', 'tiktok', 'kwai')),
    day_of_week INTEGER NOT NULL CHECK (day_of_week BETWEEN 0 AND 6),
    time_of_day TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_schedule_slots_channel_id ON schedule_slots(channel_id);

CREATE TABLE templates (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    title_template TEXT,
    description_template TEXT,
    hashtags_json TEXT NOT NULL DEFAULT '[]',
    platform TEXT CHECK (platform IN ('youtube', 'tiktok', 'kwai')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_templates_workspace_id ON templates(workspace_id);

CREATE TABLE activity_events (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT REFERENCES workspaces(id) ON DELETE CASCADE,
    category TEXT NOT NULL CHECK (category IN ('system', 'content', 'publication', 'platform', 'warning', 'error')),
    level TEXT NOT NULL CHECK (level IN ('info', 'success', 'warning', 'error')),
    message TEXT NOT NULL,
    metadata_json TEXT,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_activity_events_created_at ON activity_events(created_at);

CREATE TABLE notifications (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT REFERENCES workspaces(id) ON DELETE CASCADE,
    type TEXT NOT NULL CHECK (type IN ('info', 'success', 'warning', 'error')),
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    read INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_notifications_read ON notifications(read);

-- Single-row-per-key settings store; keeps future settings additions from
-- requiring a migration each time.
CREATE TABLE app_settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
