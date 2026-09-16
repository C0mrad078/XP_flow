# Auth Broker

`services/auth-broker/` — a small, separately deployable Axum service that exists for exactly one reason: TikTok
and Kwai's confidential OAuth operations need a `client_secret` no distributed desktop binary can hold safely.
YouTube never talks to this service (see `docs/platform-authentication.md` §3).

It is **not** the XP FLOW backend. It has no opinion about content, queues, schedules, analytics or workspace
state, and is never allowed to grow into one — its entire surface is versioned OAuth session/connection
management for two providers.

## 1. What it owns

- `oauth_sessions` — single-use, expiring (10 minutes) authorization attempts. Serves two shapes: TikTok supplies
  its own `session_id` as an idempotency key and the row is inserted already `Completed` in one step (a unique
  violation on replay maps to a clean `CodeInvalid` — the exchange endpoint is not retry-safe by design); Kwai gets
  a broker-generated session that starts `Pending` and is completed later by its own callback route.
- `connections` — encrypted-at-rest tokens (AES-256-GCM, nonce-prefixed base64), one row per real connected
  account, with the same `(platform, provider_account_id)` identity-uniqueness partial index the desktop uses.
- Nothing else. No video metadata, no schedules, no user-facing text beyond error codes.

## 2. Security properties

- **No generic proxy / no SSRF surface.** Every provider endpoint (`open.tiktokapis.com/...`,
  `open.kwai.com/...`) is a hardcoded constant in `src/providers/{tiktok,kwai}.rs`. Nothing ever passes the broker
  a URL to fetch. Test-only constructors (`with_base_url`) exist to point at `wiremock` in the test suite and are
  structurally unreachable from any production code path (`state.rs` always calls `::new()`).
- **Encryption at rest.** `AUTH_BROKER_MASTER_KEY` (32 raw bytes, base64-encoded) drives AES-256-GCM for every
  token this broker persists. Missing or malformed → the process refuses to start (fail-fast, section 85 —
  unlike the desktop, which must degrade a single unconfigured provider gracefully, this service's entire reason
  to exist is handling that key correctly).
- **Redaction.** `src/redact.rs` strips anything that looks like a secret field before it reaches a log line —
  the TikTok exchange request body is logged for debugging only through this filter.
- **Locked-down surface.** `TimeoutLayer` (20s), `RequestBodyLimitLayer` (16KB), `ConcurrencyLimitLayer` (64), and
  a fully closed `CorsLayer` (no origin is allowed — this service is only ever called by the desktop app's own
  backend, never by a browser).
- **No secrets ever committed.** `.env` is gitignored; `.env.example` documents every variable with no real
  values.

## 3. API (all under `/v1`)

| Route | Purpose |
|---|---|
| `GET /v1/health` | Liveness check |
| `POST /v1/auth/tiktok/exchange` | Confidential code-for-token exchange, given `{session_id, workspace_id, code, code_verifier, redirect_uri}` from the desktop's own loopback capture |
| `POST /v1/auth/kwai/start` | Creates a pending session, returns `{session_id, authorize_url}` |
| `GET /v1/auth/kwai/callback` | Kwai's own OAuth redirect target — registered with Kwai, not a desktop loopback port |
| `GET /v1/auth/sessions/:id` | Poll target for both TikTok and Kwai flows |
| `POST /v1/connections/:id/refresh` | Refresh a stored token |
| `POST /v1/connections/:id/revoke` | Best-effort provider-side revocation |
| `GET /v1/connections/:id/status` | Connection status without exposing the token itself |

## 4. Running it locally

```bash
cd services/auth-broker
cp .env.example .env
# Generate a master key:
openssl rand -base64 32   # paste into AUTH_BROKER_MASTER_KEY
cargo run
```

With no `TIKTOK_CLIENT_KEY`/`TIKTOK_CLIENT_SECRET` or `KWAI_APP_ID`/`KWAI_APP_SECRET` set, the broker still starts
cleanly — it just answers every request for that provider with a clean `ProviderNotConfigured`/
`BROKER_CONFIGURATION_ERROR` instead of crashing, mirroring the desktop's own per-provider graceful-degradation
rule (§5).

The desktop discovers this broker via `XPFLOW_AUTH_BROKER_URL` (see `infrastructure::auth::broker_config`); with
that unset, a debug desktop build defaults to `http://127.0.0.1:8787`, and a release build has no broker at all
until explicitly configured. `AuthBrokerConfig::is_secure_enough()` refuses anything that isn't HTTPS unless the
URL is explicitly loopback (`127.0.0.1`/`localhost`) — a production desktop build pointed at a non-loopback,
non-HTTPS broker URL drops the broker client entirely (`Stub`) rather than sending credentials in the clear.

## 5. Provider credentials setup

**TikTok** — a Login Kit + Content Posting API app in the [TikTok for Developers](https://developers.tiktok.com)
console. Register the desktop's loopback redirect pattern (`http://127.0.0.1:*/oauth/tiktok/callback` — TikTok's
console needs an exact or wildcard-compatible URI depending on current policy; check the console's current
requirements). `TIKTOK_CLIENT_KEY` goes to *both* the desktop (`.env`/environment, non-secret) and the broker;
`TIKTOK_CLIENT_SECRET` goes to the broker **only** — it must never be present in the desktop's environment, build,
or source.

**Kwai** — an app in Kwai's Open Platform developer console, with the broker's own `AUTH_BROKER_PUBLIC_URL` +
`/v1/auth/kwai/callback` registered as the redirect URI (this must be a real, reachable HTTPS URL for anything
beyond local development — Kwai's own servers call it directly). `KWAI_APP_ID`/`KWAI_APP_SECRET` are broker-only;
the desktop never sees them.

## 6. Deployability

This is a normal, statically-linkable Axum binary — `cargo build --release` in `services/auth-broker/` produces
one. It is deliberately not a Cargo workspace member of the desktop app (`src-tauri/`); the two crates share no
code and are versioned/deployed independently, matching the "this is a separate service, not a shared library"
framing throughout this document.

## 7. What this is explicitly not for

Never a place to add: content storage, queue/schedule state, analytics, comment threads, or any workspace concept.
If a future feature needs a server-side component that isn't provider-confidential-credential handling, it needs
its own justification and its own service — growing this one into a general backend was an explicit non-goal from
the start (Phase 4 spec, "no generic proxy / must never become a general SaaS backend").
