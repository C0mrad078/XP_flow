# XP FLOW Architecture

This document explains how XP FLOW is put together and, more importantly, _why_ — the constraints each layer exists
to enforce, so future phases extend this foundation instead of working around it.

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
├── hooks/            Cross-feature React hooks (e.g. useAppInfo)
├── lib/
│   ├── tauri/        The ONLY files that import @tauri-apps/api — a typed client per backend domain
│   ├── formatting/   Date/number/duration/byte formatters (UTC → local conversion lives here, nowhere else)
│   └── utilities/    cn() (Tailwind class merge), feature flags, theme resolution, seeded-gradient thumbnails
├── stores/           Zustand stores — one per concern (settings, workspace, notifications, toasts, ui)
├── styles/            globals.css — every design token, Tailwind v4 @theme config
└── types/domain.ts   Hand-mirrored TypeScript types for every Rust domain type serialized over IPC
```

### Why a typed IPC layer

`src/lib/tauri/client.ts` wraps `@tauri-apps/api`'s `invoke` in a single function that normalizes _any_ thrown value
into the same `AppError` shape the Rust side already returns (see §5). Every other module in `lib/tauri/` (`workspace.ts`,
`settings.ts`, `activity.ts`, `notifications.ts`, `system.ts`) is a thin, fully-typed wrapper per backend feature. A
React component calls `workspaceApi.create(name)`, gets a `Promise<Workspace>`, and never sees a raw command string
or an `unknown` catch value. This is also the seam a future remote backend would plug into without touching a single
component.

### Why `development/mock-data` is separate

Several Phase 1 screens (Queue, Content, Channels, Dashboard) are UI-complete but not wired to real backend
persistence yet — the brief is explicit that Phase 1 should establish the visual/interaction language, not the real
scheduler or platform sync. Every mock dataset lives under `src/development/mock-data/`, is typed independently of
the real domain DTOs, and is imported only by the feature that needs it. Deleting the directory and wiring a real
`VideoRepository`/`PublicationRepository`-backed hook in its place is a contained, mechanical change.

### Design tokens

`src/styles/globals.css` defines every color, radius, animation-duration and z-index value as a CSS custom property
under `@theme` (Tailwind v4's CSS-first config). Dark is the only fully art-directed theme for Phase 1; light exists
structurally via a `data-theme="light"` attribute swap and is applied by `src/lib/utilities/theme.ts`. No component
reaches for a raw hex value — see `docs/development-guidelines.md` for the enforcement rule.

## 3. Backend architecture

```text
src-tauri/src/
├── domain/            Entities + value objects + ports (traits). Zero dependencies on anything below this line.
│   └── ports/          Trait contracts: repositories, SecureStorage, PlatformConnector, MediaService
├── application/        Use-case orchestration that Tauri commands call directly (WorkspaceService, SettingsService, ActivityService)
├── services/           Cross-cutting technical services that aren't tied to one aggregate (MediaStatusService, NotificationService)
├── infrastructure/      Concrete adapters implementing domain ports
│   ├── repositories/    SQLx implementations of the repository traits — the only place SQL is written
│   ├── connectors/       Stub PlatformConnector (YouTube/TikTok/Kwai) — proves the contract, returns NotImplemented
│   ├── media/            FFmpeg/FFprobe detection
│   └── logging/          tracing-subscriber setup (console + rotating JSON file)
├── platform/            The only code allowed to know about the OS: data-directory resolution, OS keychain backend
├── persistence/          SQLite pool creation + migration bootstrap (sqlx::migrate!)
├── jobs/                 Job/JobType/JobStatus vocabulary — no scheduler/worker pool yet
├── commands/             Tauri IPC handlers — extract State<AppState>, call one application service method, map the result
├── state.rs              AppState: every application service + services, assembled once at startup
├── error.rs              AppError: stable error code + separate developer/user-facing messages
└── lib.rs                Wires everything: resolves paths → opens DB → runs migrations → builds AppState → registers commands
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
terminal-state enforcement, and the failure/retry loop.

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
`GenerateThumbnail`, `Backup`, `Cleanup`) and `JobStatus` (`Pending`, `Running`, `Succeeded`, `Failed`, `Cancelled`).
Nothing schedules or executes a `Job` yet — there is no worker pool, no polling loop, no persistence table for jobs.
This is deliberately just the vocabulary Phase 2's Tokio-based job runner will use.

## 4. Database

SQLite via SQLx, opened at `<data_dir>/xpflow.db`. Migrations live in `src-tauri/migrations/` and are embedded at
compile time via `sqlx::migrate!("./migrations")`, then applied automatically in `persistence::db::init_pool` on
every startup (`open → check migrations → apply pending → continue`). A migration failure is returned as a
`DatabaseError` and the app refuses to start rather than run against a partially-migrated schema.

All primary keys are UUID v4 stored as `TEXT`. All timestamps are stored as RFC 3339 UTC strings and are converted to
the user's local timezone only in `src/lib/formatting/date.ts` — nowhere else formats a timestamp for display.

See `0001_init.sql` for the full schema: `workspaces`, `channels`, `platform_accounts`, `videos`, `video_sources`,
`publications`, `queue_items`, `schedule_slots`, `templates`, `activity_events`, `notifications`, `app_settings`.

## 5. IPC contract

Every `#[tauri::command]` function is registered once, in `lib.rs::run()`'s `invoke_handler`. The frontend never
calls `invoke("some_command_name")` directly outside `src/lib/tauri/` — see
`docs/development-guidelines.md` for why that boundary is enforced.

## 6. What Phase 2+ is expected to add on top of this

- Real `PlatformConnector` implementations (OAuth flows, upload APIs, metrics/comment sync) behind the existing
  trait — no changes to `domain` or `commands` should be required.
- A job scheduler consuming the `jobs::Job` vocabulary, running on Tokio tasks with graceful shutdown.
- A `jobs` persistence table and repository, following the same pattern as every other entity.
- Cut.pro folder watching, real thumbnail/transcode generation via the `MediaService` port.
- Wiring `ChannelRepository`/`VideoRepository`/`PublicationRepository` (already implemented against real SQLite) into
  the Channels/Content/Queue screens in place of `development/mock-data`.
- Calendar/Comments/Analytics/Automation screens, currently explicit placeholders.
