# XP FLOW Architecture

This document explains how XP FLOW is put together and, more importantly, _why_ — the constraints each layer exists
to enforce, so future phases extend this foundation instead of working around it.

This covers Phase 1 (foundation/shell), Phase 2 (media library) and Phase 3 (queue engine, scheduler, calendar,
priority system). See `docs/media-library.md` for a deeper dive on the ingestion pipeline, folder watching, duplicate
detection and caching, and `docs/scheduler.md` for a deeper dive on the queue/scheduler/calendar internals.

## 1. High-level shape

```text
React (src/)
   │  invoke()
   ▼
Typed IPC client (src/lib/tauri/*)
   │  Tauri IPC
   ▼
Tauri commands (src-tauri/src/commands/*)
   │
   ▼
Application services (src-tauri/src/application/*, src-tauri/src/services/*)
   │
   ▼
Domain ports (src-tauri/src/domain/ports/*)      ◄── domain never depends on what implements these
   │
   ▼
Infrastructure adapters (src-tauri/src/infrastructure/*, src-tauri/src/platform/*)
   │
   ▼
SQLite (src-tauri/src/persistence/*) / OS keychain / filesystem
```

Every arrow points one direction. A React component never imports `@tauri-apps/api` directly; a Tauri command never
writes SQL; the domain layer never imports anything from `infrastructure`. This is enforced by convention and code
review today — nothing in Phase 1 needs a lint rule to catch it because the module boundaries make the violation
visually obvious (an `infrastructure::` import inside `domain/` stands out immediately).

## 2. Frontend architecture

```text
src/
├── app/            Root component, route table, providers (router, tooltip), layouts (AppShell)
├── components/
│   ├── ui/          Design-system primitives (Button, Dialog, Card, ...) — Radix + Tailwind, XP FLOW's own look
│   ├── navigation/   Sidebar, TopBar, CommandPalette, NotificationCenter
│   ├── feedback/     EmptyState, ErrorState, LoadingState, Toaster
│   └── common/       Cross-feature building blocks (PageHeader, MediaThumbnail, PageTransition, ...)
├── features/        One folder per screen (dashboard/, today/, queue/, content/, channels/, activity/, settings/, ...)
├── development/
│   └── mock-data/    Mock data for screens not yet wired to real persistence — isolated so it's trivial to delete
├── hooks/            Cross-feature React hooks (e.g. useAppInfo) + TanStack Query hooks (use-content, use-sources, ...)
├── lib/
│   ├── tauri/        The ONLY files that import @tauri-apps/api — a typed client per backend domain
│   ├── formatting/   Date/number/duration/byte/media formatters (UTC → local conversion lives here, nowhere else)
│   ├── query-client.ts  Shared TanStack QueryClient — backend-derived state goes through Query, not Zustand
│   └── utilities/    cn() (Tailwind class merge), feature flags, theme resolution, seeded-gradient thumbnails, media-url (xpflowmedia:// URL builders)
├── stores/           Zustand stores — one per concern (settings, workspace, notifications, toasts, ui, content — the
│                     latter is UI-only: view mode/filters/selection, never a copy of server data, section 71)
├── styles/            globals.css — every design token, Tailwind v4 @theme config
└── types/domain.ts, media.ts   Hand-mirrored TypeScript types for every Rust type serialized over IPC
```

### Server state vs. UI state (section 71)

Anything that originates on the backend — the Content Library, folder sources, channels — is fetched and cached with
**TanStack Query** (`hooks/use-content.ts`, `use-sources.ts`, `use-channels.ts`), never copied into a Zustand store.
`stores/content-store.ts` holds only interaction state that has no server representation: which view mode is active,
the current filter/sort/page selection, which video ids are checked, and which video's details/quick-preview panel is
open. Mutations (`useUpdateVideo`, `useBulkUpdateVideos`, ...) invalidate the relevant query keys on success rather
than hand-rolling optimistic cache patches — simpler, and correct by construction since the next fetch is always the
source of truth.

### Why a typed IPC layer

`src/lib/tauri/client.ts` wraps `@tauri-apps/api`'s `invoke` in a single function that normalizes _any_ thrown value
into the same `AppError` shape the Rust side already returns (see §5). Every other module in `lib/tauri/` (`workspace.ts`,
`settings.ts`, `activity.ts`, `notifications.ts`, `system.ts`) is a thin, fully-typed wrapper per backend feature. A
React component calls `workspaceApi.create(name)`, gets a `Promise<Workspace>`, and never sees a raw command string
or an `unknown` catch value. This is also the seam a future remote backend would plug into without touching a single
component.

### Why `development/mock-data` is separate

A few screens remain intentionally mock-backed because their underlying feature (social analytics, comments sync,
automation/AI) is explicitly out of scope through Phase 3 (`docs/scheduler.md` §"what's still a placeholder"). Every
remaining mock dataset lives under `src/development/mock-data/`, is typed independently of the real domain DTOs, and
is imported only by the feature that needs it — Queue, Content, Channels, Today and Dashboard's queue-derived tiles
were all migrated off mock data in Phase 3 (`mock-data/queue.ts` and `mock-data/channels.ts` were deleted); Dashboard
keeps mock view-count/performance widgets since real social analytics don't exist yet.

### Design tokens

`src/styles/globals.css` defines every color, radius, animation-duration and z-index value as a CSS custom property
under `@theme` (Tailwind v4's CSS-first config). Dark is the only fully art-directed theme for Phase 1; light exists
structurally via a `data-theme="light"` attribute swap and is applied by `src/lib/utilities/theme.ts`. No component
reaches for a raw hex value — see `docs/development-guidelines.md` for the enforcement rule.

## 3. Backend architecture

```text
src-tauri/src/
├── domain/            Entities + value objects + ports (traits). Zero dependencies on anything below this line.
│   └── ports/          Trait contracts: repositories, SecureStorage, PlatformConnector, MediaService, MediaProbeService,
│                        ThumbnailService, ContentHashService, PerceptualHashService
├── application/        Use-case orchestration that Tauri commands call directly: WorkspaceService, SettingsService,
│                        ActivityService, ChannelService, SourceService, ContentService, MediaIngestionService (the
│                        single ingestion pipeline every import path converges on — see docs/media-library.md §1),
│                        PublicationService (queue lifecycle), SchedulerService (scheduling/calendar),
│                        ScheduleSlotService, PlatformAccountService — see docs/scheduler.md
├── services/           Cross-cutting technical services: MediaStatusService, NotificationService, JobRunner (the
│                        concurrency/orchestration layer on top of MediaIngestionService — bounded-concurrency import,
│                        folder-watcher dispatch, startup/periodic reconciliation — see docs/media-library.md §2-3)
├── infrastructure/      Concrete adapters implementing domain ports
│   ├── repositories/    SQLx implementations of the repository traits — the only place SQL is written
│   ├── connectors/       Real PlatformConnector (account lifecycle) + PlatformPublisher (real uploads) per
│   │                     platform — YouTube/TikTok/Kwai; StubConnector/StubPublisher for an unconfigured platform
│   ├── media/            FFmpeg/FFprobe detection, binary resolution, structured probing, thumbnail extraction
│   ├── hashing/          SHA-256 content hashing (streamed) and dHash perceptual hashing
│   ├── filesystem/       File-stability detection, path normalization, cache accounting, reveal-in-file-manager
│   ├── watcher/          Cross-platform multi-root folder watching (notify + notify-debouncer-full)
│   └── logging/          tracing-subscriber setup (console + rotating JSON file)
├── platform/            The only code allowed to know about the OS: data-directory resolution, OS keychain backend
├── persistence/          SQLite pool creation + migration bootstrap (sqlx::migrate!)
├── jobs/                 Job/JobType/JobStatus/JobRepository — persisted and executed for real by JobRunner as of
│                        Phase 2 (ingestion/scan/reconcile job types); Phase 3 adds AutoSchedule/RebuildSchedule/
│                        FillScheduleGaps/QueueReconciliation to the vocabulary (QueueReconciliation runs at startup;
│                        the other three are reserved for a future async trigger — see docs/scheduler.md §7).
│                        Publishing job types (PublishVideo/CollectMetrics/FetchComments) remain unused placeholders
├── commands/             Tauri IPC handlers — extract State<AppState>, call one application service method, map the
│                        result — plus commands::media_protocol, a custom xpflowmedia:// URI scheme for local preview
├── state.rs              AppState: every application service + services, assembled once at startup
├── error.rs              AppError: stable error code + separate developer/user-facing messages
├── test_support.rs       (cfg(test) only) fake MediaProbeService/ThumbnailService/hashing implementations so the
│                        ingestion pipeline can be tested without FFmpeg installed — see docs/media-library.md
└── lib.rs                Wires everything: resolves paths → opens DB → runs migrations → builds AppState → starts the
                         folder watcher and reconciliation → registers commands and the media protocol
```

### Why domain/application/infrastructure are separate crates-in-spirit

- **`domain`** contains only entities, enums and trait definitions. It has no `sqlx`, no `tauri`, no `tokio::fs`. This
  is what lets `cargo test` run the `PublicationStatus` state-machine tests in milliseconds with zero I/O, and it's
  what makes "swap SQLite for a remote sync target later" a bounded change: you write a new
  `impl PublicationRepository for RemoteRepository` and nothing in `domain` or `application` changes.
- **`application`** depends on `domain` (specifically, on the port traits) but never on `infrastructure` directly —
  it receives `Arc<dyn WorkspaceRepository>`, not a concrete `SqliteWorkspaceRepository`. `state.rs` is the one place
  that knows the concrete types and wires them together.
- **`infrastructure`** is the only layer allowed to import `sqlx`, `keyring`, or shell out to `ffmpeg`.

### Publication state machine

`domain::publication::PublicationStatus` encodes every state from the brief (`Imported → Validating → Ready →
Queued → Scheduled → Uploading → Processing → Published`, plus `Failed`, `RetryWait`, `AuthRequired`, `RateLimited`,
`Blocked`, `Paused`, `Cancelled`, `Archived`, `Duplicate`). `PublicationStatus::allowed_next()` is the single source
of truth for legal transitions; `Publication::transition()` is the only way to change a publication's status, and it
returns `DomainError::InvalidTransition` rather than allowing an impossible jump (e.g. `Imported → Published`
directly). This is unit tested (`cargo test -p xp-flow domain::publication`) for the happy path, invalid skips,
terminal-state enforcement, and the failure/retry loop. Phase 3 (`docs/scheduler.md`) drives this machine for real —
`Ready → Queued` on "Add to Queue", `Queued → Scheduled` on manual/auto-schedule, `Scheduled → Queued` on unschedule —
without adding a single new state; "Overdue" is deliberately a _derived_ label
(`Publication::is_overdue`/`isPublicationOverdue`), never a persisted status.

### Error architecture

`src-tauri/src/error.rs::AppError` is the _only_ type every `#[tauri::command]` returns as its `Err` variant. It
carries:

- `code: ErrorCode` — a closed enum (`Validation`, `Database`, `Authentication`, `Network`, `RateLimit`, `Media`,
  `Platform`, `Internal`) the frontend can `switch` on without parsing strings.
- `user_message` — short, safe, always fine to show a person.
- `developer_message` — the real detail, logged and available in dev builds, never a raw `Debug` dump of an internal
  Rust error.

`From<DomainError>`, `From<SecureStorageError>` and `From<PlatformConnectorError>` implementations translate every
internal error type into this shape at the command boundary, so a `sqlx::Error` (or its message) never reaches the
frontend directly.

### Secure storage

`domain::ports::secure_storage::SecureStorage` is a three-method trait (`set`/`get`/`delete`).
`platform::secure_storage_keyring::KeyringSecureStorage` implements it via the `keyring` crate, which selects macOS
Keychain, Windows Credential Manager, or Linux Secret Service at compile time. Phase 1 does not store any real
platform credential yet — this exists so Phase 2's OAuth work has a tested, working seam instead of inventing one
under deadline.

### Platform connectors and publishers

Two separate ports govern a platform integration, split along Phase 4/5's actual boundary between account
lifecycle and publishing:

- `domain::ports::platform_connector::PlatformConnector` — account-lifecycle operations that have nothing to do
  with uploading media: `validate_connection`, `refresh_connection`, `disconnect`, `get_profile`,
  `acquire_access_token`. Real YouTube/TikTok/Kwai implementations exist (`docs/platform-authentication.md`);
  `StubConnector` is the graceful-degradation fallback for an unconfigured platform.
- `domain::ports::platform_publisher::PlatformPublisher` — the actual upload contract:
  `validate_media`/`validate_metadata`, `initialize_upload`, `upload_media`, `finalize_publication`,
  `get_remote_status`, `recover_upload`, `cancel_upload_if_supported`. Real implementations exist per provider
  (`docs/publishing-engine.md`); `FakePublisher` (a first-class scripted backend) and `StubPublisher` (the same
  graceful-degradation pattern) round out the set.

Earlier Phase 4 revisions of `PlatformConnector` carried coarse publishing placeholder methods
(`publish_video`/`get_publication_status`/`fetch_metrics`/`fetch_comments`, all `NotImplemented`) as a seam for a
future phase to fill in. Phase 5 removed them entirely rather than filling them in — publishing needed a genuinely
different, richer contract (chunked transfer, recoverable session state, remote-status polling) than a single
`publish_video` call could express, so it got its own port instead of overloading `PlatformConnector`.

### Job foundation

`jobs::job` defines `Job`, `JobType` (`ValidateVideo`, `PublishVideo`, `CollectMetrics`, `FetchComments`,
`GenerateThumbnail`, `Backup`, `Cleanup`, plus Phase 2's `ScanFolder`/`IngestVideo`/`ReconcileSource`) and `JobStatus`
(`Pending`, `Running`, `Succeeded`, `Failed`, `Cancelled`), persisted via `jobs::JobRepository`. As of Phase 2, the
media-ingestion job types actually run: `services::job_runner::JobRunner` enqueues an `IngestVideo` job (with a
path-derived dedupe key) for every file the folder watcher discovers, bounds concurrency with a 3-permit semaphore,
and recovers any job left `running` by a killed process back to `failed` on the next startup. The publishing-related
job types (`PublishVideo`, `CollectMetrics`, `FetchComments`) remain unused placeholders until a future phase adds
real platform integrations. See `docs/media-library.md` §3 for the idempotency/crash-recovery details.

## 4. Database

SQLite via SQLx, opened at `<data_dir>/xpflow.db`. Migrations live in `src-tauri/migrations/` and are embedded at
compile time via `sqlx::migrate!("./migrations")`, then applied automatically in `persistence::db::init_pool` on
every startup (`open → check migrations → apply pending → continue`). A migration failure is returned as a
`DatabaseError` and the app refuses to start rather than run against a partially-migrated schema.

All primary keys are UUID v4 stored as `TEXT`. All timestamps are stored as RFC 3339 UTC strings and are converted to
the user's local timezone only in `src/lib/formatting/date.ts` — nowhere else formats a timestamp for display.

`0001_init.sql` establishes the Phase 1 schema: `workspaces`, `channels`, `platform_accounts`, `publications`,
`queue_items`, `schedule_slots`, `templates`, `activity_events`, `notifications`, `app_settings`.

`0002_media_library.sql` (Phase 2) recreates `videos` and `video_sources` with the expanded media-library shape (see
`docs/media-library.md`), and adds `duplicate_matches` and `jobs`. Recreating rather than `ALTER`ing was safe because
no production data existed yet; see the migration file's header comment for the reasoning. `video_sources` changed
meaning between phases — Phase 1's per-video provenance record became Phase 2's folder-source configuration (name,
`folder_path`, `channel_id`, `recursive`, `watch_enabled`, `last_scan_at`, `last_error`) per section 8 of the Phase 2
brief; every workspace gets an implicit `manual_import`-type source for file-dialog/drag-and-drop imports that aren't
tied to a folder.

`0003_content_hash_uniqueness.sql` adds the partial unique index closing a concurrent-ingestion duplicate race (see
`docs/media-library.md` §7).

`0004_queue_scheduler.sql` (Phase 3) recreates `publications`, `queue_items` and `schedule_slots` with the expanded
queue/scheduler shape and adds `schedule_exceptions` — again safe because no publishing pipeline had ever populated
them — and `ALTER`s `workspaces` (`timezone`), `channels` (`status`) and `platform_accounts` (`default_target`) in
place, since those tables could already hold real rows from manual testing. It also recreates `jobs` only to extend
its `job_type` CHECK list (SQLite cannot `ALTER` a CHECK constraint in place). Two partial unique indexes are the
database-level backstop for Phase 3's core invariants — see `docs/scheduler.md` §4 for exactly what they enforce and
why the sequential application-level check alone is not sufficient under concurrency.

## 5. IPC contract

Every `#[tauri::command]` function is registered once, in `lib.rs::run()`'s `invoke_handler`. The frontend never
calls `invoke("some_command_name")` directly outside `src/lib/tauri/` — see
`docs/development-guidelines.md` for why that boundary is enforced.

Local video/thumbnail preview is the one exception to plain `invoke()`: `commands::media_protocol` registers a custom
`xpflowmedia://` URI scheme (`register_asynchronous_uri_scheme_protocol`) that resolves a video id to a file path
server-side and streams bytes back with HTTP `Range` support. The frontend requests
`https://xpflowmedia.localhost/video/<id>` (or `.../thumbnail/<id>`) as a plain `<video>`/`<img>` `src` — it never
receives or constructs a filesystem path (section 89 of the Phase 2 brief).

## 6. What Phase 2 added

- The full local media ingestion pipeline (`MediaIngestionService`), folder watching and reconciliation (`JobRunner`,
  `FolderWatcherService`), exact/near-duplicate detection, and a working Content Library UI. See
  `docs/media-library.md` for the detail.
- Real `ChannelService`/`list_channels`/`create_channel` (list + create only) so the channel-assignment pickers this
  phase needed have real UUIDs to work with — the full Channels _screen_ stayed Phase 1's mock UI until Phase 3.
- `VideoRepository`/`ContentService` now back a real screen (Content); `PublicationRepository` existed but was unused
  by any UI until Phase 3 implemented real scheduling.

## 7. What Phase 3 added

- A real, persistent queue engine and scheduler (`PublicationService`, `SchedulerService`, `ScheduleSlotService`,
  `PlatformAccountService`) and the pure DST-safe scheduling algorithm (`domain::scheduling`) it's built on — see
  `docs/scheduler.md` for the full detail.
- Real Queue (List/Timeline), Calendar (Month/Week, drag-and-drop reschedule), a Channel weekly-schedule editor, and
  real Today/Dashboard/Channels screens, replacing every remaining Queue/Channel mock.
- Database-level concurrency safety (two partial unique indexes) for the two invariants that actually matter under
  a real race: no double-booked schedule slot, no duplicate active publication for the same video/channel/platform.
- The themed `confirmAction()`/`<ConfirmDialogHost/>` replacement for the two remaining `window.confirm()` call sites
  (a Phase 2 known limitation this phase was required to close).

## 8. What Phase 4 added

- Real OAuth account connection for YouTube (direct, PKCE, loopback), TikTok (desktop-captured code, broker-
  exchanged) and Kwai (entirely broker-owned) — see `docs/platform-authentication.md` for the full write-up.
- A new, separately deployable **Auth Broker** service (`services/auth-broker/`, not a Cargo workspace member) that
  holds TikTok/Kwai client secrets so the desktop binary never has to — see `docs/auth-broker.md`.
- An expanded `PlatformAccount` domain model (typed 8-state lifecycle, derived `Capability`s from granted scopes,
  derived-only `ConnectionHealth`), replacing Phase 1/3's placeholder shape — see `docs/provider-capabilities.md`.
- `PlatformAuthService` (background-task/poll/cancel connect flow) and `TokenLifecycleService` (periodic refresh
  sweep, reusing the existing `JobRunner` rather than a second worker system).
- Database-level identity-uniqueness (a real provider account can't be connected twice in a workspace) alongside
  the application-layer check, replacing the old `UNIQUE(channel_id, platform)` constraint that conflicted with
  `default_target` ever making sense.
- `ChannelService::list_operational_overview` — the section-96 N+1 fix for `ChannelCard`, four workspace-scoped
  queries total regardless of channel count, replacing three IPC round trips per rendered card.
- A real Settings → Integrations screen, and real per-platform connection state (never queue-cancelling) on
  Channels/Queue/Dashboard.

## 9. What Phase 5 added

- A real, exactly-once publishing engine (`PublishingEngineService`) replacing the Phase 4 "future phase" note
  below about `PlatformConnector`'s publishing methods — those coarse placeholder methods were removed entirely
  and replaced by the provider-neutral `PlatformPublisher` port (`domain::ports::platform_publisher`), which
  `PublishingEngineService` never branches on `Platform` against. See `docs/publishing-engine.md` for the full
  write-up.
- A new `domain::publishing` module: `RemoteUploadState` (distinct from `PublicationStatus`), `PublishError` (full
  classified taxonomy mirroring `AuthError`'s shape), `PublicationAttempt`/`AttemptStatus` (durable, never
  overwritten), `UploadSession` (recoverable per-attempt provider state, redacting `Debug`), `PublicationConsent`
  (TikTok's express-consent proof, covering by content hash rather than a boolean), and `RenderedMetadata`/
  `MetadataTemplate`/`HashtagSet` (the last two not yet wired to a service — see below).
- Real `PlatformPublisher` implementations for YouTube (resumable upload), TikTok (chunked Direct Post) and Kwai
  (stepwise fragment upload) — `docs/youtube-publishing.md`, `docs/tiktok-publishing.md`, `docs/kwai-publishing.md`
  — plus `FakePublisher` (a first-class scripted backend, not test-only) and `StubPublisher` (the same
  graceful-degradation pattern Phase 4's `StubConnector`/`StubAuthProvider` established).
  Every provider uploader builds its `reqwest::Client` through `infrastructure::publishing::http_client`, with
  upload-appropriate timeouts kept deliberately separate from the short-timeout auth client Phase 4 built.
- Claim/lease concurrency safety at the repository level: `PublicationRepository::try_claim_due` (a single guarded
  `UPDATE`), `update_execution_state` (claim-token-guarded — the only legitimate way to touch execution state once
  claimed), and `try_finish_processing` (status-guarded, since polling is idempotent and needs no token). The
  generic `update()`/`bulk_update()` structurally exclude the four execution-state columns and refuse to run
  against a currently `Uploading`/`Processing` row, returning `DomainError::Conflict` rather than silently
  reverting an active claim — a real bug found and fixed during this phase.
- `domain::readiness::compute_readiness` (noted as unwired in the Phase 4 write-up below) is now wired: a new
  `PublishingReadinessService` assembles its inputs from real account/video/consent state, exposed via
  `get_publication_readiness`.
- New commands: `publish_now`, `retry_publication`, `get_publication_attempts`, `get_publication_readiness`,
  `record_publication_consent` — `commands::publishing_commands`.
- `CredentialAcquisitionService` — the single point all publishing code acquires provider access tokens through,
  and `PlatformAccountRepository::try_begin_refresh`, a true compare-and-swap closing a TOCTOU gap Phase 4's
  check-then-set refresh pattern left open.
- The Auth Broker gained one deliberate token-exposing endpoint (`POST /v1/connections/:id/access-token`) — see
  `docs/auth-broker.md` §8 — the narrow, documented exception to "the broker never returns a raw token," needed so
  TikTok/Kwai video bytes can still go desktop → provider directly rather than through any XP FLOW server.
- **No frontend** — see `docs/publishing-engine.md`'s closing section and the README's implementation-status list
  for exactly what Phase 6 (or later) still needs to build on top of this.

## 10. What a future phase is expected to add on top of this

- Frontend surfaces for everything Phase 5 built: live upload progress, Publication Details attempt history,
  a metadata editor, Settings → Publishing, a TikTok consent-confirmation dialog, and Queue/Today/Dashboard/Activity
  screens that reflect real publishing state instead of Phase 3/4 state only.
- `MetadataTemplateService` — the precedence-resolution service for the `MetadataTemplate`/`HashtagSet` domain
  types Phase 5 added (Publication override → Channel+Platform → Channel default → Workspace default). Publishing
  currently renders metadata directly from a Publication's own fields.
- A rate-limit-aware backoff/concurrency limiter reading real provider `Retry-After` state — `provider_rate_state`
  has a repository but nothing writes to it yet.
- A deliberate "repost" action that mints a fresh `execution_key` rather than reusing the one tied to the original
  remote write.
- Background/async triggering for auto-schedule/rebuild/fill-gaps (the `JobType` vocabulary already exists —
  `docs/scheduler.md` §7).
- Bundled FFmpeg/FFprobe binaries in packaged builds (the sidecar resolution order already exists —
  `infrastructure::media::resolver` — nothing is bundled yet).
- Comments/Analytics/Automation screens, currently explicit placeholders (out of scope through Phase 5).
- A full custom-schedule-for-one-date exception system (only "skip this date" is implemented — `docs/scheduler.md` §1).
