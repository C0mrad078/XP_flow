-- Auth Broker's own minimal persistence (section 22). Deliberately not
-- the XP FLOW desktop database — this service knows nothing about videos,
-- channels, queues or schedules, only what authentication itself requires.

-- Tracks every authorization attempt across both brokered providers
-- (section 41). Serves two shapes:
--   * Kwai: a row is created by POST /v1/auth/kwai/start (broker owns the
--     whole flow, including its own PKCE pair) and completed when Kwai's
--     redirect reaches this broker's own callback endpoint.
--   * TikTok: the desktop already captured the authorization code itself
--     (via its own loopback listener) and supplies `id` as a client-
--     generated correlation id on POST /v1/auth/tiktok/exchange — the row
--     is created and immediately completed in that one call, and its
--     presence is what makes a replayed exchange request rejected
--     (section 12: "state tokens must be single-use").
CREATE TABLE oauth_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    platform TEXT NOT NULL CHECK (platform IN ('tiktok', 'kwai')),
    workspace_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    state TEXT NOT NULL,
    -- Encrypted at rest (section 84) even though its exposure window is
    -- short — it is still key material that can complete an OAuth
    -- exchange on this session's behalf.
    code_verifier_encrypted TEXT,
    redirect_uri TEXT,
    status TEXT NOT NULL CHECK (status IN ('pending', 'completed', 'failed', 'expired')) DEFAULT 'pending',
    connection_id TEXT REFERENCES connections(id) ON DELETE SET NULL,
    error_code TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    consumed_at TEXT
);
CREATE INDEX idx_oauth_sessions_status ON oauth_sessions(status);
CREATE INDEX idx_oauth_sessions_expires_at ON oauth_sessions(expires_at);

-- One row per real connected provider account (section 22's
-- `auth_connections`). Token material is AES-256-GCM encrypted at rest
-- (section 84) under `AUTH_BROKER_MASTER_KEY` — never plaintext.
CREATE TABLE connections (
    id TEXT PRIMARY KEY NOT NULL,
    platform TEXT NOT NULL CHECK (platform IN ('tiktok', 'kwai')),
    workspace_id TEXT NOT NULL,
    provider_account_id TEXT NOT NULL,
    display_name TEXT,
    username_or_handle TEXT,
    avatar_url TEXT,
    granted_scopes_json TEXT NOT NULL DEFAULT '[]',
    access_token_encrypted TEXT NOT NULL,
    refresh_token_encrypted TEXT,
    access_expires_at TEXT,
    refresh_expires_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    revoked_at TEXT
);
CREATE INDEX idx_connections_workspace_id ON connections(workspace_id);
-- Section 53's identity-uniqueness rule, enforced broker-side too (the
-- broker is the source of truth for which real accounts are connected).
CREATE UNIQUE INDEX idx_connections_identity
    ON connections(workspace_id, platform, provider_account_id)
    WHERE revoked_at IS NULL;
