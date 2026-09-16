-- Phase 5: real publishing engine — resumable uploads, crash recovery,
-- idempotency, metadata and provider execution.
--
-- Design decisions (see docs/publishing-engine.md for the full write-up):
--
-- * `publications` gains claim/lease columns rather than a separate
--   "claims" table — the claim IS the publication's own `status = 'uploading'`
--   transition (already a valid edge in the Phase 3 state machine), and a
--   lease is just "how do we know this claim is stale," which belongs on
--   the same row it protects.
-- * Attempt history and upload-session recovery state are new tables —
--   `publications` itself stays the coarse, user-facing record (section 9:
--   "do not overwrite all execution history in the Publication row").
-- * `publication_consent` has no `invalidated_at` column: a consent row
--   is valid if and only if its `approved_metadata_hash` still matches
--   the publication's *current* rendered metadata hash. Comparing hashes
--   at read time is simpler and less error-prone than remembering to flip
--   a flag on every metadata-editing code path (section 33).
-- * `provider_rate_state` is deliberately thin — real per-provider quota
--   numbers are not reliably exposed by any of the three providers, so
--   this only ever records "we were told to wait until X," never a fake
--   precise remaining-quota count (section 130).
-- * `templates` (Phase 1, `0001_init.sql`) is dropped rather than
--   extended: nothing ever wrote a row to it (no repository/service was
--   ever built against it), and section 25's precedence rules need
--   channel-level scoping and a title/description split it doesn't have.
--   Same reasoning as Phase 4's `platform_accounts` recreation in
--   `0005_platform_auth.sql`.

DROP TABLE IF EXISTS templates;

ALTER TABLE publications ADD COLUMN execution_key TEXT;
ALTER TABLE publications ADD COLUMN claim_token TEXT;
ALTER TABLE publications ADD COLUMN lease_expires_at TEXT;
ALTER TABLE publications ADD COLUMN rendered_metadata_json TEXT;

CREATE INDEX idx_publications_lease_expires_at ON publications(lease_expires_at)
    WHERE lease_expires_at IS NOT NULL;

-- One row per execution attempt (section 9). Never overwritten — a new
-- attempt is a new row, `attempt_number` increasing per publication.
CREATE TABLE publication_attempts (
    id TEXT PRIMARY KEY NOT NULL,
    publication_id TEXT NOT NULL REFERENCES publications(id) ON DELETE CASCADE,
    attempt_number INTEGER NOT NULL,
    provider TEXT NOT NULL CHECK (provider IN ('youtube', 'tiktok', 'kwai')),
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'succeeded', 'failed', 'cancelled')) DEFAULT 'pending',
    started_at TEXT,
    completed_at TEXT,
    bytes_total INTEGER,
    bytes_uploaded INTEGER,
    error_code TEXT,
    error_message TEXT,
    -- NULL until classified (section 70) — a `pending`/`running` attempt
    -- has no retry verdict yet.
    retryable INTEGER,
    remote_operation_id TEXT,
    created_at TEXT NOT NULL,
    UNIQUE(publication_id, attempt_number)
);
CREATE INDEX idx_publication_attempts_publication ON publication_attempts(publication_id, attempt_number DESC);

-- Recoverable provider upload state (section 10). Sensitive fields
-- (`remote_upload_url`, `remote_upload_token`) are local-only — never
-- transmitted anywhere beyond this SQLite file and the provider that
-- issued them — but are still never written to a log line (see
-- `domain::publishing::upload_session`'s redacting `Debug` impl).
CREATE TABLE upload_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    publication_id TEXT NOT NULL REFERENCES publications(id) ON DELETE CASCADE,
    attempt_id TEXT NOT NULL REFERENCES publication_attempts(id) ON DELETE CASCADE,
    provider TEXT NOT NULL CHECK (provider IN ('youtube', 'tiktok', 'kwai')),
    session_type TEXT NOT NULL,
    remote_session_id TEXT,
    remote_upload_url TEXT,
    remote_publish_id TEXT,
    remote_upload_token TEXT,
    bytes_total INTEGER,
    bytes_committed INTEGER NOT NULL DEFAULT 0,
    expires_at TEXT,
    state TEXT NOT NULL CHECK (state IN (
        'not_started', 'initialized', 'transferring', 'transferred',
        'remote_processing', 'remote_succeeded', 'remote_failed', 'remote_unknown'
    )) DEFAULT 'not_started',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_upload_sessions_publication ON upload_sessions(publication_id);
CREATE INDEX idx_upload_sessions_attempt ON upload_sessions(attempt_id);

-- TikTok express-consent proof (section 31-34). No `invalidated_at` —
-- see the file header. `approval_source` records how the user approved
-- (manual schedule, Add to Queue, bulk approval, Publish Now) so the
-- consent trail stays auditable.
CREATE TABLE publication_consent (
    id TEXT PRIMARY KEY NOT NULL,
    publication_id TEXT NOT NULL REFERENCES publications(id) ON DELETE CASCADE,
    provider TEXT NOT NULL CHECK (provider IN ('youtube', 'tiktok', 'kwai')),
    approved_at TEXT NOT NULL,
    approved_metadata_hash TEXT NOT NULL,
    approval_source TEXT NOT NULL CHECK (approval_source IN (
        'manual_schedule', 'add_to_queue', 'bulk_approval', 'publish_now'
    )),
    created_at TEXT NOT NULL
);
CREATE INDEX idx_publication_consent_publication ON publication_consent(publication_id, created_at DESC);

-- Metadata templates (section 22-26). `channel_id`/`platform` NULL means
-- "applies at the workspace default" / "applies to every platform" —
-- precedence resolution lives in application code
-- (`MetadataTemplateService`), not in SQL.
CREATE TABLE metadata_templates (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    channel_id TEXT REFERENCES channels(id) ON DELETE CASCADE,
    platform TEXT CHECK (platform IN ('youtube', 'tiktok', 'kwai')),
    kind TEXT NOT NULL CHECK (kind IN ('title', 'description')),
    template_text TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_metadata_templates_scope ON metadata_templates(workspace_id, channel_id, platform, kind);

-- Reusable hashtag groups (section 27) — same scoping rule as templates.
CREATE TABLE hashtag_sets (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    channel_id TEXT REFERENCES channels(id) ON DELETE CASCADE,
    platform TEXT CHECK (platform IN ('youtube', 'tiktok', 'kwai')),
    name TEXT NOT NULL,
    hashtags_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_hashtag_sets_scope ON hashtag_sets(workspace_id, channel_id, platform);

-- Lightweight rate-limit bookkeeping (section 78/101/129-130) — records
-- only what a provider actually told us (a Retry-After-derived
-- timestamp), never a synthesized quota number no provider exposes.
CREATE TABLE provider_rate_state (
    id TEXT PRIMARY KEY NOT NULL,
    platform_account_id TEXT NOT NULL REFERENCES platform_accounts(id) ON DELETE CASCADE,
    operation TEXT NOT NULL,
    retry_after TEXT,
    last_rate_limited_at TEXT,
    updated_at TEXT NOT NULL,
    UNIQUE(platform_account_id, operation)
);
