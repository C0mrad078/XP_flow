# Platform authentication (Phase 4)

How XP FLOW connects, validates, refreshes and disconnects real YouTube, TikTok and Kwai accounts. Real video
publishing is **not** part of this — see [Deferred to Phase 5](#deferred-to-phase-5) at the bottom.

## 1. Why this looks the way it does

XP FLOW stays local-first: all content, queue, schedule and database data lives on the user's machine, no cloud
backend required. OAuth breaks that cleanly for two of the three platforms — TikTok and Kwai's confidential
token exchange needs a `client_secret` that a distributed desktop binary cannot hold safely (anyone can extract a
string from a shipped executable). Google's own threat model for installed apps treats YouTube differently, so
YouTube never touches a broker at all.

The result is a **two-port split**, not one uniform OAuth client:

- **`PlatformAuthProvider`** — provider-specific. One `authenticate(session, cancel_rx) -> Result<ConnectedIdentity, AuthError>`
  call per provider; the internals are allowed to differ completely (see §3).
- **`PlatformConnector`** — uniform once connected. `validate_connection`, `refresh_connection`, `disconnect`,
  `get_profile`, `store_local_credential`, `acquire_access_token` — same shape for all three providers once a
  `PlatformAccount` exists. Account lifecycle only — actual publishing goes through the separate
  `PlatformPublisher` port Phase 5 added (`docs/publishing-engine.md`), not through this trait.

## 2. Domain model

`domain::platform_account::PlatformAccount` — no OAuth secret ever lives on this row:

| Field                                                                             | Purpose                                                                                     |
| --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `provider_account_id`                                                             | The real provider's stable account id — what identity-uniqueness is keyed on                |
| `provider_connection_id`                                                          | Opaque broker-side connection id (TikTok/Kwai only; `None` for YouTube)                     |
| `display_name`, `username_or_handle`, `avatar_url`                                | Profile display only                                                                        |
| `status`                                                                          | `PlatformAccountStatus` — see §4                                                            |
| `granted_scopes`, `capabilities`                                                  | Raw provider scopes, and the derived `Capability` set (see `docs/provider-capabilities.md`) |
| `access_expires_at`, `refresh_expires_at`                                         | Token lifetime, used by the refresh sweep and by `ConnectionHealth`                         |
| `last_validated_at`, `last_refreshed_at`, `last_error_code`, `last_error_message` | Operational history                                                                         |

Raw token material lives elsewhere: YouTube's access/refresh tokens go into the OS keychain via `SecureStorage`
(`infrastructure::secure_storage`), keyed `xpflow.youtube.credential.{account_id}`. TikTok/Kwai tokens never leave
the Auth Broker's own encrypted SQLite — the desktop only ever holds `provider_connection_id`, an opaque reference.

**Identity safety.** The same real provider account cannot be connected twice in a workspace — enforced at both
layers: `PlatformAuthService::finish_connect` checks `find_by_provider_identity` before persisting, and
`idx_platform_accounts_identity` (a partial unique index on `(workspace_id, platform, provider_account_id) WHERE
provider_account_id IS NOT NULL`) is the database-level backstop. Reconnecting an existing row with a _different_
real account fails closed with `AccountIdentityMismatch` unless the caller explicitly passes
`allow_identity_change: true` — never a silent identity swap. A `Revoked` row (from a prior disconnect) is not
treated as "connected" for this check — a fresh "Connect" for the same real account revives that row and moves it
to whichever channel the user picked, rather than erroring or creating a duplicate.

## 3. Per-provider OAuth transport

### YouTube — desktop-only, direct

Authorization Code + PKCE (S256), system browser, loopback redirect. `infrastructure::auth::LoopbackListener` binds
`127.0.0.1:0` (OS-assigned port, **never** `0.0.0.0`), runs a single-request Axum server, and shuts down after one
callback or a timeout. XP FLOW talks directly to Google's real endpoints
(`accounts.google.com/o/oauth2/v2/auth`, `oauth2.googleapis.com/token`, `oauth2.googleapis.com/revoke`,
`openidconnect.googleapis.com/v1/userinfo`, `www.googleapis.com/youtube/v3/channels`) — no broker involvement,
because Google does not treat an installed app's `client_secret` as confidential.

### TikTok — desktop captures the code, broker exchanges it

Desktop Login Kit OAuth + PKCE, desktop's own loopback listener captures the authorization code, then forwards
`{session_id, workspace_id, code, code_verifier, redirect_uri}` to the broker's `POST /v1/auth/tiktok/exchange` for
the confidential code-for-token exchange. TikTok's PKCE strategy is a separate type
(`domain::oauth::pkce::TikTokPkceStrategy`) from Google's even though both currently implement the same RFC 7636
S256 transform — deliberately not assumed identical, since nothing guarantees a provider's documented behavior
stays that way.

### Kwai — entirely broker-owned

The desktop never sees a Kwai authorization code. It calls the broker's `POST /v1/auth/kwai/start` (gets back
`{session_id, authorize_url}`), opens the browser, then polls `GET /v1/auth/sessions/:id` every 2 seconds (bounded
to a 300-second timeout) until the session reaches `Completed`/`Failed`/`Expired`. Kwai's own OAuth redirect lands
on the broker's `GET /v1/auth/kwai/callback` — an endpoint registered with Kwai, not a desktop loopback port.

## 4. Status, health and capabilities — three different signals

These are deliberately not the same thing:

- **`PlatformAccountStatus`** (persisted) — the account row's actual lifecycle:
  `NotConfigured → Connecting → Connected ⇄ Refreshing`, plus `PermissionMissing`, `ReauthRequired`, `Revoked`,
  `Error`.
- **`ConnectionHealth`** (derived, never persisted — `domain::connection_health::derive_connection_health`) —
  `Healthy` / `TokenExpiring` / `PermissionMissing` / `RefreshRequired` / `Disconnected` / `ProviderError`. Computed
  fresh from `status` + `access_expires_at` every time it's needed, same "derive, don't store" discipline as
  Phase 3's publication-overdue detection. `TOKEN_EXPIRING_BUFFER` is 24 hours. Mirrored client-side in
  `src/types/platform-auth.ts::deriveConnectionHealth` rather than round-tripped over IPC for every row rendered.
- **`AuthFlowState`** (in-memory, live progress only) — `OpeningBrowser → WaitingForAuthorization →
VerifyingAccount → SavingConnection → Connected | Failed{code, message}`. This is what a connect/reconnect
  dialog polls; it has no relationship to the persisted `status` of an account that already exists.
- **`Capability`** (derived from `granted_scopes` via `domain::capability::map_scopes_to_capabilities`) —
  `ReadProfile`, `UploadVideo`, `ReadVideoStatus`, `ReadMetrics`, `ReadComments`, `WriteComments`. See
  `docs/provider-capabilities.md` for the full scope-to-capability table. **Default requested scopes never grant
  `UploadVideo`** — tested (`capability::tests::default_requested_scopes_never_include_publishing`) — connecting an
  account in this phase never silently acquires publish rights.

## 5. The connect/reconnect flow

A connect attempt can take as long as the user takes to finish authorizing in their browser, needs to expose live
progress, and needs to be cancellable — none of which fits a single blocking Tauri command. So
`PlatformAuthService::begin_connect`/`begin_reconnect` return a `session_id` immediately and spawn a detached
`tokio::spawn` task that drives `authenticate()` to completion, updating an in-memory
`Mutex<HashMap<Uuid, Flight>>` the frontend polls via `poll_platform_connect_status` (every 800ms while a connect
dialog is open — never a global background poll loop) and can interrupt via `cancel_platform_connect`. A finished
flight's terminal state stays pollable for 60 seconds before its map entry is dropped, so the map doesn't grow
unboundedly over a long-running session.

`AuthSession` (the live PKCE/state material for one attempt) is deliberately **in-memory only**, 10-minute TTL,
never persisted, never logged — if the app is killed mid-flow, "click Connect again" is the recovery path for a
sub-10-minute action.

## 6. Token lifecycle

`TokenLifecycleService::refresh_expiring_accounts()` sweeps every account due for refresh (access token expiring
within the 24-hour buffer) and reuses the existing `JobRunner` — `spawn_periodic_token_refresh` runs every 15
minutes alongside Phase 3's `spawn_periodic_reconciliation`, not a second background-worker system.

Refreshing narrows (does not fully eliminate) the race between the periodic sweep and a manual reconnect/validate
click on the same account: `PlatformAuthService::refresh()` checks `status == Refreshing` before proceeding and
bails with a clean error if so. This is a pragmatic check-then-set guard, not a true compare-and-swap (no row
version column) — but either outcome (skip, or two harmless back-to-back refreshes) never corrupts data; the last
write always wins cleanly. A provider reporting `TokenRevoked` on refresh moves the account to `ReauthRequired`
(not `Error`) and fires a notification — refresh-token rotation is assumed by default, never "the old refresh
token still works."

## 7. UI surfaces

- **Settings → Integrations** (`src/features/settings/sections/integrations-section.tsx`) — one card per provider,
  every connected account across the workspace, a channel-picker step before handing off to the real connect flow,
  and a "Manage" sheet per account (profile, granted permissions, assigned channel with reassignment, token
  expiry/validation history, Validate/Reconnect/Disconnect). Never renders a raw token.
- **Channels** (`src/features/channels/channel-card.tsx`) — real per-platform account rows and inline connect,
  backed by the section-96 aggregate query (see below), not a placeholder quick-add.
- **Queue/Dashboard** — a missing/unhealthy account surfaces as a small warning on the affected queue rows and a
  compact "Account health" summary on the dashboard; it never cancels or removes anything from the queue.
- Disconnect uses the themed `confirmAction()`/`<ConfirmDialogHost/>` (established in Phase 3) — never
  `window.confirm`.

## 8. Section 96's N+1 fix

`ChannelService::list_operational_overview(workspace_id)` replaces the Phase 3 pattern of a `ChannelCard` calling
three separate hooks/IPC round trips per rendered card (platform accounts, schedule slots, queue count). It runs
exactly four workspace-scoped queries total — channels, queued counts grouped by channel, active-slot counts
grouped by channel, platform accounts — and assembles them in Rust via `HashMap` grouping, regardless of how many
channels the workspace has.

## 9. Known limitations

- No live provider credentials were available during development — every OAuth code path was verified against
  fakes (desktop-side `FakeAuthProvider`/`FakeConnector`) or a mocked HTTP layer (`wiremock`, broker-side). Nothing
  in this codebase or its tests should be read as a claim of live end-to-end verification against real Google/TikTok/
  Kwai endpoints.
- The refresh-race guard is check-then-set, not a true CAS — see §6.
- `domain::readiness::compute_readiness` (account/video readiness issues for a publication) was, at the time this
  was written, unit tested but not wired to a Tauri command; Phase 5 has since wired it via
  `PublishingReadinessService` and the `get_publication_readiness` command (`docs/publishing-engine.md`). The
  Queue/Dashboard readiness signals described in §7 predate that and are still a client-side equivalent computed
  directly from already-fetched `PlatformAccount[]` data — not yet switched over to the real command.

## What Phase 5 built on top of this

Real video publishing/uploads for YouTube, TikTok and Kwai, crash recovery, retry, and TikTok's express-consent
gate — see `docs/publishing-engine.md`. Comments fetch/reply and full social analytics ingestion remain out of
scope. `CredentialAcquisitionService` (introduced in Phase 5) is now the single point all publishing code acquires
an access token through, calling into the exact `PlatformConnector`/`acquire_access_token` and
`PlatformAuthService::refresh` machinery this document describes — nothing about token storage or refresh changed
underneath it.
