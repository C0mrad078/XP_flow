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
│   ├── connectors/       Stub PlatformConnector (YouTube/TikTok/Kwai) — proves the contract, returns NotImplemented
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
without adding a single new state; "Overdue" is deliberately a *derived* label
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

### Platform connectors

`domain::ports::platform_connector::PlatformConnector` is the contract every social platform integration must
satisfy: `authenticate`, `disconnect`, `validate_session`, `publish_video`, `get_publication_status`,
`fetch_metrics`, `fetch_comments`. `infrastructure::connectors::StubConnector` implements it for all three platforms
today, and every method returns `PlatformConnectorError::NotImplemented`. This is intentional: the shape of the
integration is fixed now, under review, rather than each platform's real implementation inventing its own shape
later.

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

## 8. What a future phase is expected to add on top of this

- Real `PlatformConnector` implementations (OAuth flows, upload APIs, metrics/comment sync) behind the existing
  trait — no changes to `domain` or `commands` should be required. `SchedulerService::due_publications` /
  `PublicationRepository::list_due` already exist as the seam a real uploader would consume.
- Background/async triggering for auto-schedule/rebuild/fill-gaps (the `JobType` vocabulary already exists —
  `docs/scheduler.md` §7).
- Bundled FFmpeg/FFprobe binaries in packaged builds (the sidecar resolution order already exists —
  `infrastructure::media::resolver` — nothing is bundled yet).
- Comments/Analytics/Automation screens, currently explicit placeholders (out of scope through Phase 3, section 111).
- A full custom-schedule-for-one-date exception system (only "skip this date" is implemented — `docs/scheduler.md` §1).
