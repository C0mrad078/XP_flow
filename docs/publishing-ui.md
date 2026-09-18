# Publishing UI

Phase 5.1 closes the gap between the publishing engine and the desktop product. The UI reads persisted publication state from the Rust backend and uses the same commands for immediate publishing, retry, consent and settings changes.

## State presentation

Queue rows use the authoritative publication status (`scheduled`, `uploading`, `processing`, `published`, `retry_wait`, `rate_limited`, `auth_required`, `blocked`, `paused` and failure states). Status text is paired with semantic badges so color is never the only signal.

Publication Details is the inspection surface for readiness issues, rendered metadata, metadata overrides, upload progress and persisted attempt history. An ambiguous publication is presented as **Needs verification** and does not expose a blind retry action — this is keyed off `Publication.last_error_code === "UNKNOWN_REMOTE_RESULT"`, a stable code populated from the originating `PublishError`, not off `PublicationStatus` (no status value is ever set specifically for this case) or free-text matching on `last_error`.

## Progress and refresh

The backend broadcasts a `publish-progress` Tauri event on every acknowledged chunk (`domain::ports::progress_publisher`). The frontend subscribes exactly once, at the app root (`usePublishProgressListener`), and writes each event into a small Zustand store keyed by publication id — deliberately not TanStack Query state, since it's transient per-viewer presentation, never the authoritative result. Queue rows and Publication Details read from this store for instant updates, falling back to the persisted attempt's byte counters when no live event has arrived yet (e.g. right after opening the app). Attempt history itself is additionally refreshed on a short poll while an attempt is running, as a correctness backstop independent of the event stream. The backend remains authoritative either way — the UI never treats a progress event as final success; only `Publication.status` is.

## Actions by state

Publish Now is available for queued/scheduled and recoverable states when readiness checks pass. Retry is limited to safe failed or rate-limited states. Both immediate actions ask for confirmation before starting a real provider request. Scheduling controls are unavailable while an upload or provider processing operation is active. The backend validates every action independently and refuses an `UNKNOWN_REMOTE_RESULT` even if invoked outside the UI.

## Metadata

The editor can save publication-level title, description and hashtag overrides. The rendered preview comes from the metadata template service, which applies publication, channel/platform, channel, workspace/platform, workspace and raw-field precedence in that order. Once execution starts, the rendered metadata snapshot is immutable.

Template and hashtag CRUD is available under **Settings → Publishing**. Templates are scoped by channel and platform and are rendered only from supported variables.

## TikTok consent

TikTok publications that require approval show an explicit approval action in Publication Details. Clicking it opens a detail dialog — video, the actual rendered caption (from the same resolution the engine uses at execution time, not a guess), and privacy/comment/duet/stitch settings — before the approval is recorded; there is no bare one-click "Approve." Consent is persisted by the backend and is bound to the rendered metadata hash; changing metadata invalidates the approval. No implicit "approve all future posts" behavior is exposed.

The Queue screen also surfaces a bulk-approval entry point: a banner counts TikTok publications that are either about to become due or already known to be blocked specifically on missing consent (a deliberately wide heuristic — approving an item that already had valid consent is a harmless extra audit row, so false positives cost nothing, while a false negative would hide a real block). The bulk dialog lists every candidate with a per-item opt-out checkbox and records one real consent row per selected publication; it never introduces a blanket future-approval toggle.

## Rate-limit visibility outside Publication Details

The Channels/Integrations screen shows each connected account's current publish rate-limit state (`get_provider_rate_state`) next to its connection health, so an account that looks "Connected" but would actually have every publish attempt skipped by the engine's own pre-check is never silently misleading.

## Publishing settings

Settings → Publishing exposes publishing enabled/paused state, maximum simultaneous uploads, missed-schedule policy and grace period. Pause/resume uses the engine commands and does not cancel active uploads implicitly.

## Rate limits and diagnostics

Provider readiness issues distinguish rate limiting from offline, authentication and validation failures. Rate-limit state is persisted per provider account and operation, and `Retry-After` is preferred over generic backoff.

Publication Details has its own "Copy diagnostics" action (separate from the Phase 4 account-level one on the Manage Account sheet), covering: publication id, attempt id, platform, `PublicationStatus`, the latest attempt's remote state, the provider's own remote operation id (safe — what a support agent needs to look the post up on the platform itself), the stable `last_error_code`, any currently-active rate-limit windows, and current readiness reasons. It never includes an access token, upload URL/token or authorization header — none of those are ever serialized to the frontend in the first place, so there is nothing to accidentally leak here.

## Known limitations of this pass

- No development-only "Simulation Mode" UI was built. `FakePublisher` exists as a real backend seam for one, but nothing in the frontend toggles it on.
- Today and Dashboard already reflect real `PublicationStatus` values (they predate this phase and were built against the same enum the engine now writes), but neither breaks "uploading" and "processing" into separate counters, or distinguishes "needs verification" as its own bucket the way Publication Details does.
- Activity already logs some real publishing transitions (upload started/waiting on processing, published) through the existing generic activity feed, but not every event Phase 5.1 introduced (rate-limited, retry scheduled, consent recorded) has a dedicated log entry.
- The metadata editor is a single overrides panel (title/description/hashtags as literal text), not the multi-tab per-provider (YouTube/TikTok/Kwai) editor with a visible template-precedence selector described in the original brief. Template/hashtag-set CRUD exists under Settings → Publishing, scoped to the workspace only — assigning a template to a specific channel or platform has no UI yet (the backend already supports it).
- No automated visual verification was performed — see the Phase 5.1 final report for exactly what that means and why.

Phase 6 begins with post-publication analytics and engagement. Comments, views, likes and moderation are intentionally outside Phase 5.1.
