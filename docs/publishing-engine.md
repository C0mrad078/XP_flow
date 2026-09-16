# Publishing engine

How XP FLOW turns a due `Publication` into a real remote post — `PublishingEngineService`
(`src-tauri/src/application/publishing_engine_service.rs`) — and the guarantee everything here exists to protect:

> A duplicate publication is a critical bug. A silently lost publication is a critical bug. A false "Published"
> status is a critical bug. At most one remote post per `Publication` unless the user explicitly initiates a repost.

This builds directly on Phase 3's scheduler (`docs/scheduler.md`) and Phase 4's account layer
(`docs/platform-authentication.md`) — it does not replace either. There is one worker runtime
(`services::job_runner::JobRunner`, already built in Phase 3) and one publishing orchestrator. No
`PublishingEngineV2`, no second scheduler, no parallel job system.

## The exactly-once guarantee, concretely

A `Publication` moving from `Scheduled` to `Uploading` is an atomic, guarded database write, not an
application-level decision:

```sql
UPDATE publications
SET status = 'uploading', execution_key = COALESCE(execution_key, ?), claim_token = ?, lease_expires_at = ?
WHERE id = ? AND status = 'scheduled' AND scheduled_at <= ? AND locked = 0
```

(`SqlitePublicationRepository::try_claim_due`.) Two callers racing to claim the same row — the periodic scan and a
concurrent "Publish Now," or two scan ticks overlapping — only one gets a `true` back from this call, verified by
concurrency tests (`two_concurrent_claims_on_the_same_publication_only_one_wins`,
`scan_and_claim_due_is_exactly_once_under_concurrency`) that actually run both calls with `tokio::join!` rather
than asserting behavior from reading the code.

`execution_key` is the identity of "one logical remote write" — it's set once and reused (`COALESCE`) across every
retry of the same publication. A deliberate repost is the one case that should get a _new_ execution key, since
it's an intentionally new remote write; that reset isn't wired to any UI yet (see Deferred, below).

`claim_token` proves current ownership of an active claim. Once a publication is `Uploading`/`Processing`, the
**only** legitimate way to touch its execution state is `PublicationRepository::update_execution_state(id,
claim_token, ...)`, which is itself guarded on `WHERE claim_token = ?`. The generic `update()` used everywhere else
in the app (editing a title, rescheduling, cancelling) structurally excludes the four execution-state columns from
its `SET` clause and additionally refuses to run at all against a row currently `Uploading`/`Processing`,
returning `DomainError::Conflict` instead of silently doing nothing. This was a real bug found and fixed during
this phase (see `an unrelated write reverting an active claim`, below) — without it, an unrelated action like
editing a publication's title while it was mid-upload could silently un-claim a row that was genuinely still in
flight, opening exactly the duplicate-execution window this whole design exists to prevent.

## Pipeline

`execute_inner` (the private method `execute` wraps, converting any error into a logged warning rather than a
panic) runs, in order:

1. **Load and re-verify.** The publication and its video are re-fetched fresh — never assumed still valid because
   they were valid when queued. The source file's existence and content hash are re-checked immediately before use
   (section 117/118): a file moved or edited after scheduling fails closed as `VideoUnavailable`/`InvalidMedia`,
   never silently uploads the wrong bytes.
2. **Account and capability check.** No connected account, or a connected account missing `Capability::UploadVideo`,
   fails before any network call — "connected" and "can publish" are different claims (`docs/provider-capabilities.md`).
3. **Metadata render + validate.** Currently rendered directly from the `Publication`'s own `title`/`description`/
   `hashtags` fields with empty `provider_options` — the full template-precedence system
   (`domain::publishing::metadata`) exists but isn't wired into this render step yet (see Deferred). Each
   `PlatformPublisher::validate_metadata` call is provider-specific and runs before any bytes move.
4. **Consent gate (TikTok only).** `requires_express_consent(platform)` is true only for TikTok's Content Posting
   API. The engine hashes the _current_ rendered metadata and checks it against the most recent
   `PublicationConsent` row; a stale or missing consent fails closed as `PublishError::ConsentRequired` — recorded
   via the `record_publication_consent` command — before ever reaching the provider.
5. **Freeze metadata.** The rendered metadata is written onto the publication row (`rendered_metadata_json`) the
   moment execution starts, so a template edited later can never retroactively change what an in-flight or
   already-sent attempt claims it sent.
6. **Attempt + credential acquisition.** A new `PublicationAttempt` row is created (durable, never overwritten —
   this is the audit trail the Publication Details drawer reads). The access token comes from
   `CredentialAcquisitionService` — the single point all publishing code acquires tokens through.
7. **Initialize → upload → finalize**, via `PlatformPublisher` (below). `upload_media` always returns the session
   with whatever `bytes_committed`/state it actually reached, even on failure — a network drop partway through has
   real partial-progress information a later `recover_upload` needs, not just an error.
8. **Status transition**, guarded through `update_execution_state`/`try_finish_processing` only:
   - `RemoteSucceeded` → `Published`, with `published_at` set and the claim released.
   - `RemoteProcessing`/`Transferred` → `Processing` — transferred is not the same claim as published; a separate
     poll confirms the provider's own result.
   - Anything else → treated as `PublishError::UnknownRemoteResult` and routed through the failure/retry path.

## `PlatformPublisher`

`domain::ports::platform_publisher::PlatformPublisher` is the provider-neutral contract every uploader implements:
`validate_media`, `validate_metadata` (pure, no I/O), `initialize_upload`, `upload_media`, `finalize_publication`,
`get_remote_status`, `recover_upload`, `cancel_upload_if_supported`. The engine never branches on `Platform` inside
its own pipeline — every provider-specific detail lives inside the implementation. Three real implementations exist
today (`docs/youtube-publishing.md`, `docs/tiktok-publishing.md`, `docs/kwai-publishing.md`), plus two first-class
non-production backends:

- **`FakePublisher`** — a scripted publisher (`FakeScenario::Success` / `FailBeforeUpload` /
  `InterruptedThenRecoverable` / `NeedsProcessing`) used by every engine test. Not test-only scaffolding to be
  deleted later — it's the intended backend for a future development-mode "simulate publishing" toggle (not yet
  wired to any UI).
- **`StubPublisher`** — the graceful-degradation fallback for a platform with no configured credentials, returning
  a clear `PlatformNotApproved`-style error instead of crashing or silently doing nothing. Registered automatically
  in `lib.rs` for any platform whose config/broker prerequisites aren't met at startup.

## Retry and recovery

- **Retry backoff** (`domain::publishing::retry_policy`): jittered exponential backoff (30s / 2min / 10min / 30min
  / 1hr tiers, ±20% jitter), capped at `MAX_PUBLISH_ATTEMPTS = 6`. A retryable failure walks the real state machine
  — `Failed → RetryWait → Queued → Scheduled(new scheduled_at)` — the same transitions `Publication::allowed_next`
  already defines, not a bypass of it.
- **Not every failure is retryable.** `PublishError::is_retryable()` excludes anything requiring user action (auth,
  permissions, invalid metadata/media, consent) and, critically, `UnknownRemoteResult` — retrying blind after an
  ambiguous outcome is exactly how a duplicate post gets created. See `docs/crash-recovery.md` for how ambiguous
  outcomes actually get resolved.
- **Crash recovery** is a separate concern from retry — see `docs/crash-recovery.md`.

## What isn't wired yet (deferred to Phase 6 work)

- The metadata template precedence engine — `MetadataTemplateService`, resolving Publication override → Channel+
  Platform → Channel default → Workspace default — exists as domain types
  (`domain::publishing::metadata::{MetadataTemplate, HashtagSet, render_template}`) with no service or commands.
- `provider_rate_state` has a repository but nothing writes to it; no backoff-aware concurrency limiter reacts to a
  real provider rate-limit response beyond marking that single attempt `RateLimited`.
- A deliberate "repost" action that mints a fresh `execution_key`.
- Frontend surfaces for all of the above (Queue/Today/Activity real-publishing state, Publication Details drawer
  attempt history, metadata editor, Settings → Publishing).
