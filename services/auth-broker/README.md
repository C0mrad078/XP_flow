# XP FLOW Auth Broker

A deliberately small, separately-deployable service that performs the one
class of operation a distributed desktop binary cannot safely do itself:
**confidential OAuth token exchange/refresh/revocation for TikTok and
Kwai**, whose provider-issued `client_secret`/`app_secret` cannot be
shipped inside `XP FLOW.exe`/`XP FLOW.app` without effectively publishing
it.

## What this is _not_

This is **not** the XP FLOW backend, and it never becomes one. It has no
concept of:

- content, videos, thumbnails, or the content library
- the queue, scheduler, or calendar
- channels, publications, or application workspace state
- analytics, comments, or anything else XP FLOW does

XP FLOW remains local-first: all of that data lives only in the desktop
app's own SQLite database. This broker's entire job is authentication —
see `docs/auth-broker.md` in the main repository for the full
responsibility boundary and data-flow diagram.

YouTube never talks to this broker at all — Google does not treat an
installed desktop app's OAuth client secret as confidential the way
TikTok's and Kwai's do, so YouTube authenticates directly
(`src-tauri/src/infrastructure/connectors/youtube/`).

## Running locally

```bash
cp .env.example .env
# fill in AUTH_BROKER_MASTER_KEY (openssl rand -base64 32) and whichever
# of TIKTOK_*/KWAI_* you have real developer credentials for — leaving a
# provider's credentials unset is fine, that provider just returns
# BROKER_CONFIGURATION_ERROR

cargo run -p xpflow-auth-broker
```

The broker refuses to start without a valid `AUTH_BROKER_MASTER_KEY`
(section 85 of the Phase 4 brief: fail fast rather than run in a
half-usable state) — every other piece of configuration degrades
independently per provider instead.

## API surface

All routes are under `/v1`. See `src/routes.rs` for the exact request/
response shapes; `docs/auth-broker.md` in the main repository documents
the flow each route participates in.

```
GET  /v1/health
POST /v1/auth/tiktok/exchange       desktop already has the auth code; broker does the confidential exchange
POST /v1/auth/kwai/start            broker creates a session + authorize URL; broker owns the whole flow
GET  /v1/auth/kwai/callback         Kwai's own redirect target (registered with Kwai, not the desktop)
GET  /v1/auth/sessions/:id          poll for completion (used by the Kwai flow)
POST /v1/connections/:id/refresh
POST /v1/connections/:id/revoke
GET  /v1/connections/:id/status
```

There is no generic `/proxy` endpoint and no route accepts a caller-
supplied URL — every provider endpoint is hardcoded in
`src/providers/{tiktok,kwai}.rs` (section 25: SSRF prevention by
construction, not by a runtime allowlist check).

## Security properties

- Every token is AES-256-GCM encrypted at rest (`src/crypto.rs`) under
  `AUTH_BROKER_MASTER_KEY` — never plaintext in SQLite.
- Every OAuth session is single-use and expires (`store::SESSION_TTL`,
  10 minutes) — a replayed exchange request is rejected
  (`tests/tiktok_flow.rs::a_replayed_exchange_with_the_same_session_id_is_rejected`).
- Nothing sensitive is ever logged unredacted — `src/redact.rs`, tested.
- Request bodies are capped, requests time out, and concurrency is
  bounded (`src/main.rs`'s middleware stack) — see
  `tests/security.rs`.
- CORS is fully locked down: nothing in this API is ever called from a
  browser.

## Testing

```bash
cargo test
```

No test ever calls a real TikTok/Kwai endpoint — `tests/tiktok_flow.rs`
and `tests/kwai_flow.rs` run against a local `wiremock` server standing
in for the provider, injected via `TikTokProviderClient::with_base_url`/
`KwaiProviderClient::with_base_url` (every production code path still
only ever constructs via `::new`, which always uses the real hardcoded
endpoint).

## Production deployment

- `AUTH_BROKER_PUBLIC_URL` and any broker URL the desktop app is
  configured to use in production **must** be HTTPS
  (`AuthBrokerConfig::is_secure_enough`, checked on the desktop side) —
  plain HTTP is only accepted for an explicit loopback address, a
  development convenience.
- `AUTH_BROKER_MASTER_KEY` must be a securely generated, durably stored
  secret. **Never rotate it casually** — rotating the key without a
  migration makes every already-stored token undecryptable
  (`crypto::CryptoError::DecryptFailed`), which forces every connected
  account to be reconnected from scratch.
- This service is stateless aside from its own SQLite database — running
  it behind a reverse proxy that terminates TLS is the expected shape.
