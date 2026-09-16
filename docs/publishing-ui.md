# Publishing UI

Phase 5.1 closes the gap between the publishing engine and the desktop product. The UI reads persisted publication state from the Rust backend and uses the same commands for immediate publishing, retry, consent and settings changes.

## State presentation

Queue rows use the authoritative publication status (`scheduled`, `uploading`, `processing`, `published`, `retry_wait`, `rate_limited`, `auth_required`, `blocked`, `paused` and failure states). Status text is paired with semantic badges so color is never the only signal.

Publication Details is the inspection surface for readiness issues, rendered metadata, metadata overrides, upload progress and persisted attempt history. A blocked or ambiguous publication is presented as **Needs verification** and does not expose a blind retry action.

## Progress and refresh

Attempt history is refreshed while an attempt is running. Progress is derived from the persisted uploaded and total byte counters; the backend remains authoritative and the UI does not treat transient progress as final success.

## Actions by state

Publish Now is available for queued/scheduled and recoverable states when readiness checks pass. Retry is limited to safe failed or rate-limited states. Scheduling controls are unavailable while an upload or provider processing operation is active. The backend validates every action independently.

## Metadata

The editor can save publication-level title, description and hashtag overrides. The rendered preview comes from the metadata template service, which applies publication, channel/platform, channel, workspace/platform, workspace and raw-field precedence in that order. Once execution starts, the rendered metadata snapshot is immutable.

Template and hashtag CRUD is available under **Settings → Publishing**. Templates are scoped by channel and platform and are rendered only from supported variables.

## TikTok consent

TikTok publications that require approval show an explicit approval action in Publication Details. Consent is persisted by the backend and is bound to the rendered metadata hash; changing metadata invalidates the approval. No implicit “approve all future posts” behavior is exposed.

## Publishing settings

Settings → Publishing exposes publishing enabled/paused state, maximum simultaneous uploads, missed-schedule policy and grace period. Pause/resume uses the engine commands and does not cancel active uploads implicitly.

## Rate limits and diagnostics

Provider readiness issues distinguish rate limiting from offline, authentication and validation failures. Rate-limit state is persisted per provider account and operation, and `Retry-After` is preferred over generic backoff. Diagnostics show safe IDs, state and classification only; tokens, signed upload URLs and authorization headers are never serialized to the frontend.

Phase 6 begins with post-publication analytics and engagement. Comments, views, likes and moderation are intentionally outside Phase 5.1.
