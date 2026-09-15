-- Phase 4: real platform authentication, account management, OAuth
-- lifecycle and connector foundation.
--
-- `platform_accounts` is recreated (not ALTERed): its Phase 1/3 shape
-- (`external_account_id`, a 4-state `status`, a hard `UNIQUE(channel_id,
-- platform)`) never held real connected-account data (no OAuth flow has
-- ever run), and the Phase 4 shape is different enough — a typed 8-state
-- lifecycle, cached capabilities, token-expiry bookkeeping, and a
-- corrected uniqueness rule — that ALTERing in place would be harder to
-- read than a clean recreate. See domain::platform_account and
-- docs/platform-authentication.md for the full shape rationale.
--
-- Design decision (section 52/53/55): the old `UNIQUE(channel_id,
-- platform)` restricted a channel to exactly one account per platform,
-- which was already in tension with Phase 3's own `default_target`/
-- "default account for this channel+platform" mechanism (which only makes
-- sense if more than one can exist). The real invariant the brief asks
-- for is identity-uniqueness — the same real provider account must not be
-- connected twice in one workspace — not a per-channel cardinality limit,
-- so that is what the new partial unique index enforces instead.
DROP TABLE IF EXISTS platform_accounts;

CREATE TABLE platform_accounts (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    channel_id TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    platform TEXT NOT NULL CHECK (platform IN ('youtube', 'tiktok', 'kwai')),

    provider_account_id TEXT,
    provider_connection_id TEXT,

    display_name TEXT,
    username_or_handle TEXT,
    avatar_url TEXT,

    status TEXT NOT NULL CHECK (status IN (
        'not_configured', 'connecting', 'connected', 'refreshing',
        'permission_missing', 'reauth_required', 'revoked', 'error'
    )) DEFAULT 'not_configured',
    granted_scopes_json TEXT NOT NULL DEFAULT '[]',
    capabilities_json TEXT NOT NULL DEFAULT '[]',

    default_target INTEGER NOT NULL DEFAULT 0,

    access_expires_at TEXT,
    refresh_expires_at TEXT,
    connected_at TEXT,
    last_validated_at TEXT,
    last_refreshed_at TEXT,
    last_error_code TEXT,
    last_error_message TEXT,

    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_platform_accounts_workspace_id ON platform_accounts(workspace_id);
CREATE INDEX idx_platform_accounts_channel_id ON platform_accounts(channel_id);
CREATE INDEX idx_platform_accounts_status ON platform_accounts(status);

-- Section 53: a real provider account may only be connected once per
-- workspace. NULL provider_account_id (never-connected rows) are exempt —
-- SQLite already treats each NULL as distinct in a UNIQUE index.
CREATE UNIQUE INDEX idx_platform_accounts_identity
    ON platform_accounts(workspace_id, platform, provider_account_id)
    WHERE provider_account_id IS NOT NULL;
