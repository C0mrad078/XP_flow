-- Phase 3: persistent queue engine, scheduler, calendar, priority system.
--
-- `publications`, `queue_items` and `schedule_slots` are replaced (not
-- ALTERed), same justification as migration 0002: their Phase 1 shape
-- never carried real operational data (no publishing pipeline existed to
-- populate them), and the Phase 3 shape differs enough (workspace_id,
-- priority, locked on Publication; a slimmer QueueItem; optional platform
-- on ScheduleSlot) that a clean recreate is clearer than ALTER churn.
-- `jobs` is likewise recreated only to extend its `job_type` CHECK list
-- (SQLite cannot ALTER a CHECK constraint in place); no job history is
-- considered durable product data at this stage.
--
-- `workspaces`, `channels` and `platform_accounts` may already hold real
-- rows from manual testing, so they are ALTERed in place instead.

DROP TABLE IF EXISTS schedule_slots;
DROP TABLE IF EXISTS queue_items;
DROP TABLE IF EXISTS publications;
DROP TABLE IF EXISTS jobs;

ALTER TABLE workspaces ADD COLUMN timezone TEXT NOT NULL DEFAULT 'UTC';

ALTER TABLE channels ADD COLUMN status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'paused'));

ALTER TABLE platform_accounts ADD COLUMN default_target INTEGER NOT NULL DEFAULT 0;

CREATE TABLE publications (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
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
    priority TEXT NOT NULL CHECK (priority IN ('low', 'normal', 'high', 'urgent')) DEFAULT 'normal',
    locked INTEGER NOT NULL DEFAULT 0,
    scheduled_at TEXT,
    published_at TEXT,
    remote_id TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_publications_workspace_id ON publications(workspace_id);
CREATE INDEX idx_publications_video_id ON publications(video_id);
CREATE INDEX idx_publications_channel_id ON publications(channel_id);
CREATE INDEX idx_publications_status ON publications(status);
CREATE INDEX idx_publications_scheduled_at ON publications(scheduled_at);

-- Section 50/51: at most one non-terminal publication per (video, channel,
-- platform). `cancelled`/`archived`/`duplicate` are excluded so a video can
-- be re-queued to the same channel/platform after cancellation or archival;
-- `published` is deliberately *included* (i.e. still blocks) so the same
-- video cannot be silently re-added to the same target while it is still
-- sitting in `published` awaiting archival.
CREATE UNIQUE INDEX idx_publications_active_dedupe
    ON publications(video_id, channel_id, platform)
    WHERE status NOT IN ('cancelled', 'archived', 'duplicate');

-- Section 27/28/51: no two `Scheduled` publications may occupy the exact
-- same instant on the same channel — the database-level backstop beneath
-- the application-level slot-conflict check, so a concurrent
-- auto-schedule/drag-reschedule race can never double-book a slot.
CREATE UNIQUE INDEX idx_publications_channel_scheduled_dedupe
    ON publications(channel_id, scheduled_at)
    WHERE status = 'scheduled';

CREATE TABLE queue_items (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    publication_id TEXT NOT NULL REFERENCES publications(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (publication_id)
);
CREATE INDEX idx_queue_items_workspace_id ON queue_items(workspace_id);

CREATE TABLE schedule_slots (
    id TEXT PRIMARY KEY NOT NULL,
    channel_id TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    -- NULL = channel-default slot (applies to any platform without its own
    -- platform-specific slots on that weekday); non-NULL = platform-specific.
    platform TEXT CHECK (platform IN ('youtube', 'tiktok', 'kwai')),
    day_of_week INTEGER NOT NULL CHECK (day_of_week BETWEEN 0 AND 6),
    time_of_day TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_schedule_slots_channel_id ON schedule_slots(channel_id);

-- Section 27/28: no two active slots at the same channel/platform-bucket/
-- day/time. SQLite treats each NULL as distinct for UNIQUE purposes, so
-- this does not prevent a channel-default (NULL) slot and a
-- platform-specific slot from sharing a day/time — that is intentional,
-- they represent different rules.
CREATE UNIQUE INDEX idx_schedule_slots_dedupe
    ON schedule_slots(channel_id, platform, day_of_week, time_of_day)
    WHERE is_active = 1;

-- One-off "skip this date" overrides (section 17). See
-- domain::schedule_exception for why only `skip` is modeled in Phase 3.
CREATE TABLE schedule_exceptions (
    id TEXT PRIMARY KEY NOT NULL,
    channel_id TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    date TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('skip')),
    reason TEXT,
    created_at TEXT NOT NULL,
    UNIQUE (channel_id, date)
);
CREATE INDEX idx_schedule_exceptions_channel_id ON schedule_exceptions(channel_id);

CREATE TABLE jobs (
    id TEXT PRIMARY KEY NOT NULL,
    job_type TEXT NOT NULL CHECK (job_type IN (
        'validate_video', 'publish_video', 'collect_metrics', 'fetch_comments',
        'generate_thumbnail', 'backup', 'cleanup', 'scan_folder', 'ingest_video', 'reconcile_source',
        'auto_schedule', 'rebuild_schedule', 'fill_schedule_gaps', 'queue_reconciliation'
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
