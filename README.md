# XP FLOW

A professional, local-first desktop application for managing networks of short-form content channels — importing
video, organizing it by channel, queuing and scheduling publications across YouTube, TikTok and Kwai, and tracking
what happened along the way.

This repository currently implements **Phase 1 (Foundation, Architecture, Desktop Shell and Design System)**,
**Phase 2 (Media Library, Cut.pro Folder Watcher, FFmpeg Integration and Local Content Management)**,
**Phase 3 (Persistent Queue Engine, Scheduler, Calendar, Priority System and Operational Workflow)**,
**Phase 4 (Real Platform Authentication, Account Management, OAuth Lifecycle and Connector Foundation)**, and the
backend of **Phase 5 (Real Publishing Engine, Resumable Uploads, Crash Recovery, Idempotency and Provider
Execution)** — real uploads to YouTube, TikTok and Kwai, with no frontend built on top of it yet. It is a
production-grade base for the full product, not a throwaway prototype — see [Current implementation status](#current-implementation-status)
for exactly what is real versus what is intentionally deferred.

## Overview

The eventual end-to-end workflow XP FLOW is built toward:

```text
Cut.pro → XP FLOW → Content Library → Queue / Scheduling → YouTube Shorts / TikTok / Kwai → Metrics, Comments, Automation, Analytics
```

Core product principles (see `docs/architecture.md` for the detail):

- **Local-first.** All persistent data lives in a local SQLite database. No cloud backend is required to run the app.
- **AI is not a dependency.** No core feature requires an LLM or any AI API.
- **Cross-platform first.** Windows (x64/ARM64) and macOS (Apple Silicon/Intel) are both first-class targets; nothing
  in shared code assumes one OS.

## Technology stack

| Layer                   | Technology                                                                                                         |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------ |
| Desktop runtime         | [Tauri 2](https://tauri.app)                                                                                       |
| Frontend                | React 19, TypeScript (strict), Vite                                                                                |
| Styling / design system | Tailwind CSS v4, Radix UI primitives, a custom component layer (not left at shadcn defaults), Lucide icons, Motion |
| Server state            | TanStack Query                                                                                                     |
| UI/interaction state    | Zustand                                                                                                            |
| Routing                 | React Router                                                                                                       |
| Backend                 | Rust, Tokio                                                                                                        |
| Database                | SQLite via SQLx, with versioned migrations                                                                         |
| Media tooling           | FFmpeg / FFprobe (system-installed or a bundled sidecar; detected, not required just to build/run the shell)       |
| Folder watching         | `notify` + `notify-debouncer-full`                                                                                 |
| Secure storage          | OS keychain via the `keyring` crate (macOS Keychain / Windows Credential Manager / Secret Service)                 |
| Logging                 | `tracing` + `tracing-subscriber`, JSON file logs plus console output                                               |

## Project structure

```text
xp-flow/
├── src/                        # Frontend (React + TypeScript)
│   ├── app/                    # Root component, routes, providers, layouts
│   ├── components/             # ui/ (design system primitives), navigation/, feedback/, common/
│   ├── features/                # One folder per screen/domain area (content/ is the Media Library — Phase 2)
│   ├── development/mock-data/  # Isolated mock data for screens not yet wired to real persistence
│   ├── hooks/                  # useAppInfo + TanStack Query hooks (use-content, use-sources, use-channels, ...)
│   ├── lib/, stores/, styles/, types/
│
├── src-tauri/                  # Backend (Rust)
│   ├── src/
│   │   ├── domain/             # Entities, value objects, ports (traits) — no infra dependency
│   │   ├── application/        # Use-case services: workspace, settings, activity, channel, source, content,
│   │   │                       # media_ingestion_service (the one ingestion pipeline every import path uses),
│   │   │                       # publication_service, scheduler_service, schedule_slot_service,
│   │   │                       # platform_account_service (Phase 3's queue/scheduler), publishing_engine_service +
│   │   │                       # publishing_readiness_service + credential_acquisition_service (Phase 5)
│   │   ├── services/           # Cross-cutting technical services: media status, notifications, job_runner
│   │   │                       # (bounded-concurrency import + folder-watcher dispatch + reconciliation)
│   │   ├── infrastructure/     # SQLx repositories, platform connector stubs, media (FFprobe/FFmpeg), hashing
│   │   │                       # (SHA-256 + perceptual), filesystem (stability/cache/reveal), watcher, logging
│   │   ├── persistence/        # SQLite pool + migration bootstrap
│   │   ├── platform/           # OS-isolated code: data paths, secure storage backend
│   │   ├── jobs/                # Job/JobType/JobStatus/JobRepository — executes real ingestion jobs (Phase 2) and
│   │   │                       # queue reconciliation (Phase 3)
│   │   ├── commands/            # Thin Tauri IPC command handlers + the xpflowmedia:// local preview protocol
│   │   └── lib.rs, main.rs, state.rs, error.rs, test_support.rs
│   └── migrations/             # SQL migrations (SQLx)
│
└── docs/                       # architecture.md, media-library.md, scheduler.md, development-guidelines.md,
                                # platform-authentication.md, auth-broker.md, provider-capabilities.md,
                                # publishing-engine.md, youtube-publishing.md, tiktok-publishing.md,
                                # kwai-publishing.md, crash-recovery.md
```

See `docs/architecture.md` for the full explanation of why the backend is layered this way,
`docs/media-library.md` for the ingestion pipeline, folder watching and duplicate detection specifically,
`docs/scheduler.md` for the queue engine, scheduler, timezone handling and calendar specifically, and
`docs/publishing-engine.md` for the exactly-once publishing engine, resumable uploads and crash recovery.

## Development requirements

- [Node.js](https://nodejs.org/) 20+ and npm
- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain) and Cargo
- Platform build tools for Tauri — see the [Tauri prerequisites guide](https://tauri.app/start/prerequisites/) for
  your OS (Xcode Command Line Tools on macOS; WebView2 + MSVC Build Tools on Windows)
- **Optional but recommended:** [FFmpeg](https://ffmpeg.org/download.html) (which includes FFprobe) on your `PATH`.
  XP FLOW builds, launches and runs without it — Settings → Advanced reports honestly if either tool isn't found —
  but video import (metadata extraction, thumbnails) needs it. `cargo test` never requires FFmpeg to be installed;
  see [Media Library](#media-library) below.

## Getting started

```bash
# Install frontend dependencies
npm install

# Run the app in development (starts Vite + compiles and runs the Rust backend)
npm run tauri:dev
```

On first launch, XP FLOW shows an onboarding screen to create your first workspace — everything after that persists
locally in SQLite.

### Other useful commands

```bash
npm run dev             # Vite dev server only (frontend, no Tauri window)
npm run typecheck       # tsc --noEmit
npm run lint            # ESLint
npm run lint:fix        # ESLint with autofix
npm run format          # Prettier --write
npm run format:check    # Prettier --check
npm run test            # Vitest (frontend unit/component tests)
npm run build           # Production frontend build (tsc + vite build)
npm run tauri:build     # Full production desktop build (installer/bundle)
```

Rust-side checks (run from `src-tauri/`):

```bash
cargo test     # Backend unit tests (domain rules, persistence, repositories)
cargo fmt      # Format
cargo clippy   # Lint
cargo build    # Compile
```

The desktop app runs and connects a real YouTube account with no other setup beyond `YOUTUBE_CLIENT_ID`/
`YOUTUBE_CLIENT_SECRET` in its environment. TikTok/Kwai need the separate Auth Broker service running too — see
`docs/auth-broker.md` for setup (`services/auth-broker/`, its own `cargo test`/`cargo build`, its own `.env`).
Neither is required to build or run XP FLOW itself; an unconfigured provider degrades to a clean
"not configured" state instead of failing to start.

## Where XP FLOW stores its data

| OS      | Location                                 |
| ------- | ---------------------------------------- |
| macOS   | `~/Library/Application Support/XP FLOW/` |
| Windows | `%APPDATA%/XP FLOW/`                     |

Inside that directory: `xpflow.db` (SQLite database), `logs/` (structured JSON logs, rotated daily), and `cache/`
(`thumbnails/` — one generated JPEG per video, `temp/` — scratch space, safe to clear anytime from Settings →
Storage). These paths are also shown in-app under **Settings → Storage**, and are resolved by
`src-tauri/src/platform/paths.rs` — never hardcoded elsewhere in the codebase. XP FLOW never writes anything into the
folders your original videos live in (section 65 of the Phase 2 brief) — it only reads, probes, hashes and previews
them.

## Media Library

Phase 2 turns XP FLOW into a real local content manager. See `docs/media-library.md` for the full write-up; the
essentials:

- **Supported formats:** `.mp4`, `.mov`, `.mkv`, `.webm`. The extension only gates whether XP FLOW attempts to
  analyze a file — the actual validation status always comes from a real FFprobe result, never the extension alone.
- **Manual import:** Content → Import lets you pick one video, several videos, or an entire folder via the native OS
  file dialog.
- **Drag and drop:** drop video files anywhere on the Content Library; they go through the exact same ingestion
  pipeline as a manual import.
- **Folder watching:** add a content folder (Settings → Storage → Content Sources, or the Content page's "Add
  Folder" button) and XP FLOW indexes what's already there, then — if you turn watching on — detects and imports new
  exports automatically. A "Cut.pro Export Folder" preset pre-fills sensible defaults (watch + recursive) but works
  with exports from any tool, not just Cut.pro.
- **File stability:** a newly-appeared file is never analyzed until its size stops changing across several
  consecutive checks — Cut.pro (or anything else) is never read mid-export.
- **Duplicate detection:** an exact content match (SHA-256) is reported as "already in your library," never
  imported as a second copy. A near-duplicate match (a lightweight perceptual hash of the thumbnail) is flagged as
  "possible duplicate" for you to review — never auto-discarded.
- **Local preview:** click a video to open its details, or hit Space over a selection for a quick preview — playback
  streams through a custom local protocol, never exposing a raw filesystem path to the frontend.

## Queue and Scheduler

Phase 3 turns the Content Library into a real content-operations system. See `docs/scheduler.md` for the full
write-up; the essentials:

- **Add to Queue:** from a video's card, its action menu, or a bulk selection in the Content Library — pick a
  channel, platform and optional priority override. Duplicate video/channel/platform combinations are rejected with a
  clean error, enforced at both the application layer and (against a real concurrent race) the database.
- **Manual and auto-scheduling:** schedule a queued publication to an exact time, or let the scheduler place it in
  the channel's next available recurring slot. "Auto-schedule channel," "Fill Empty Slots" and "Rebuild Schedule"
  operate on a whole channel at once, priority-first, inside a single database transaction.
- **Calendar:** Month/Week views with drag-and-drop rescheduling between days (locked publications can't be
  dragged), resolved through the workspace's configured IANA timezone — never the OS's.
- **Channel weekly schedule editor:** per-weekday recurring time slots (channel-default or platform-specific), a
  "copy this day to every other day" shortcut, and skip-date exceptions.
- **Priority and locking:** every publication carries its own priority (seeded from its video, independently
  mutable) and a lock flag that protects it from ever being moved by the auto-scheduler or a schedule rebuild.
- **Overdue** is a derived label (a `Scheduled` publication whose time has passed), never a persisted state and never
  a false "Failed" — Phase 3 has no real uploader to fail.

## Platform authentication

Phase 4 adds real OAuth account connection for YouTube, TikTok and Kwai — connect, validate, refresh and disconnect,
never real video publishing (that's Phase 5). See `docs/platform-authentication.md`, `docs/auth-broker.md` and
`docs/provider-capabilities.md` for the full write-up; the essentials:

- **YouTube** goes straight to Google — Authorization Code + PKCE, system browser, a loopback listener bound only
  to `127.0.0.1`, no embedded browser, no broker.
- **TikTok** captures its own authorization code on a desktop loopback listener, then hands the confidential
  code-for-token exchange to a small separate **Auth Broker** service (`services/auth-broker/`) that holds the
  TikTok client secret so the desktop binary never has to.
- **Kwai**'s entire flow is broker-owned — the desktop only opens a browser to a broker-issued URL and polls for
  completion; Kwai's own redirect lands on the broker, never on the desktop.
- A `PlatformAccount` carries derived `Capability`s (`ReadProfile`, `UploadVideo`, ...) from granted scopes, and a
  derived-only `ConnectionHealth` distinct from its persisted `PlatformAccountStatus` — "connected" and "approved
  to publish" are never the same claim.
- The same real provider account can't be connected twice in a workspace (enforced at the database and the
  application layer); reconnecting with a different real account fails closed and requires explicit confirmation.
- Settings → Integrations (provider cards, connect/manage/disconnect) and the Channels screen both use the same
  connect flow and the same `ChannelOverview` aggregate query introduced to fix a documented N+1 IPC pattern.

## Publishing engine

Phase 5 turns Phase 3's scheduler and Phase 4's connected accounts into a real publishing system. See
`docs/publishing-engine.md` for the full write-up, plus `docs/youtube-publishing.md`, `docs/tiktok-publishing.md`,
`docs/kwai-publishing.md` and `docs/crash-recovery.md` per provider; the essentials:

- **At most one remote post per publication, always.** Claiming a due publication is a single guarded database
  write (`WHERE status='scheduled' AND locked=0`, proven race-safe under real concurrency), and every subsequent
  write to its execution state is guarded on the claim token that operation actually holds — an unrelated action
  (editing a title, rescheduling) can never revert an active claim.
- **Real uploaders for YouTube, TikTok and Kwai** — resumable/chunked/stepwise transfer per each provider's actual
  protocol, talking directly to the provider (never proxied through any XP FLOW server) except for the token itself
  on TikTok/Kwai, which the Auth Broker issues.
- **Crash recovery that verifies before ever resuming** — a claim abandoned by a killed process is never blindly
  restarted; real remote state is checked first, per provider, before anything is re-sent.
- **TikTok's express-consent requirement** is a hard gate, keyed to the exact rendered metadata by content hash —
  an edit after approval means the old approval no longer covers it.
- Commands to drive all of this: `publish_now`, `retry_publication`, `get_publication_attempts`,
  `get_publication_readiness`, `record_publication_consent`.
- **No frontend built on this yet** — see Current implementation status below.

## Current implementation status

**Implemented and real:**

- Tauri 2 + React + TypeScript (strict) + Vite shell, cross-platform window defaults
- SQLite persistence with versioned migrations, applied automatically on startup
- Domain model for Workspace, Channel, Platform, PlatformAccount, Video, VideoSource, Publication, QueueItem,
  ScheduleSlot, ScheduleException, Template, ActivityEvent, Notification, AppSettings, DuplicateMatch, Job
- A fully-typed `PublicationStatus` state machine with centrally-validated transitions (unit tested), kept entirely
  separate from `ValidationStatus`/`AvailabilityStatus` (media health has nothing to do with publishing state)
- Typed IPC boundary (`src/lib/tauri/*` ↔ `src-tauri/src/commands/*` ↔ application services ↔ repositories), plus a
  custom `xpflowmedia://` protocol for local video/thumbnail preview
- Real, persisted: workspace creation/onboarding, app settings, activity event log, channels (list/create), the full
  media library (videos, folder sources, duplicate matches, jobs)
- **The complete local media ingestion pipeline** (Phase 2): manual import, native OS drag-and-drop, and folder
  watching all converge on one `MediaIngestionService` — file-stability detection, real FFprobe analysis, SHA-256
  exact-duplicate detection (with move detection), a lightweight perceptual-hash near-duplicate check, thumbnail
  generation, startup + periodic folder reconciliation, and idempotent/crash-recoverable background jobs bounded to
  3 concurrent FFmpeg/FFprobe processes at a time
- A functional Content Library: search, composable filters, sort, server-side pagination, grid/list views, bulk
  selection/actions, a rich details panel, local preview (with Space/Escape quick preview), and a Folder Sources
  management screen with per-source watch status and cache accounting
- **The full queue engine and scheduler** (Phase 3): real `Publication`/`QueueItem`/`ScheduleSlot`/
  `ScheduleException` persistence, a pure DST-safe scheduling algorithm, manual/auto/bulk scheduling (transactional),
  a real Queue (List/Timeline) and Calendar (Month/Week, drag-and-drop reschedule) screen, a channel weekly-schedule
  editor, and database-level concurrency safety for the invariants that matter under a real race
- **Real platform authentication** (Phase 4): YouTube OAuth 2.0 + PKCE direct to Google; TikTok Login Kit + PKCE
  with the confidential exchange handled by the separate Auth Broker service; Kwai's entire flow broker-owned; a
  background-task/poll/cancel connect flow with a live progress dialog; periodic token-refresh sweep reusing the
  existing `JobRunner`; identity-uniqueness enforced at both the application layer and the database; a real
  Settings → Integrations screen and real per-platform connection state on the Channels/Queue/Dashboard screens.
  OS-native secure storage (Keychain / Credential Manager) now holds real YouTube credentials; TikTok/Kwai tokens
  never leave the Auth Broker's own encrypted SQLite.
- **Real publishing engine, backend only** (Phase 5): exactly-once claim/execution against real YouTube, TikTok and
  Kwai uploaders (resumable/chunked/stepwise per provider), crash recovery that verifies real remote state before
  ever resuming or restarting, jittered exponential retry backoff, TikTok's express-consent gate keyed to a
  metadata content hash, and the `publish_now`/`retry_publication`/`get_publication_attempts`/
  `get_publication_readiness`/`record_publication_consent` commands exposing it — see `docs/publishing-engine.md`.
- Structured logging to a local JSON log file
- A complete, customized UI component library (not default shadcn) and design token system, including a themed
  confirmation dialog (`confirmAction()`) replacing every native `window.confirm()`
- Every screen: Dashboard, Today, Queue (real), Calendar (real), Content (real, Grid/List), Channels (real, including
  pause/resume and platform-target management), Activity (real + demo events), Settings, onboarding, plus explicit
  placeholders for Comments/Analytics/Automation
- Command palette, notification center, collapsible sidebar, dark theme (polished) / light theme (structural)

**Deliberately not implemented yet** (see `docs/architecture.md`, `docs/scheduler.md`,
`docs/platform-authentication.md` and `docs/publishing-engine.md` for what each needs before it can land):

- **No frontend for real publishing.** Phase 5's engine, uploaders and commands are real and tested, but no UI
  consumes them yet — no live upload progress, no Publication Details attempt history, no metadata editor, no
  Settings → Publishing screen, no TikTok consent-confirmation dialog. The Queue/Today/Dashboard/Activity screens
  still reflect Phase 3/4 state only.
- **No metadata template system.** `MetadataTemplate`/`HashtagSet` domain types and their repositories exist;
  the precedence-resolution service (`MetadataTemplateService`) and CRUD commands don't. Publishing renders
  metadata directly from a Publication's own fields today.
- **No rate-limit-aware backoff.** `provider_rate_state` has a repository but nothing writes to it yet; a
  provider's own rate-limit response is only ever handled as that one attempt's `RateLimited` error.
- **No deliberate "repost" action** — the `execution_key` mechanism that would let one exist is in place, but
  nothing in the app currently mints a fresh one.
- Comments fetch/reply and full social analytics ingestion remain entirely out of scope, as before.
- No live YouTube/TikTok/Kwai developer credentials were available while building Phase 4 or Phase 5 — every OAuth
  and publishing code path was verified against fakes/mocks (desktop) or `wiremock` (broker/uploaders), never a
  live provider. "Implementation complete" is not the same claim as "provider-approved for production publishing"
  — see `docs/provider-capabilities.md`. Kwai's uploader specifically is grounded in an unofficial third-party API
  client rather than Kwai's own documentation, which was not reachable while building it — see
  `docs/kwai-publishing.md` for exactly which parts of that implementation are verified fact versus a stated
  assumption.
- Video transcoding (FFmpeg detection and thumbnail extraction exist; re-encoding does not)
- Bundled FFmpeg/FFprobe binaries in packaged builds (the sidecar-resolution logic exists; nothing is bundled)
- Background/async triggering of auto-schedule/rebuild/fill-gaps (the job-type vocabulary exists; nothing enqueues it
  yet — those actions run as synchronous commands today)
- A full custom-schedule-for-one-date exception system (only "skip this date" is implemented)
- Automation rules, AI of any kind, cloud sync

## License

See `LICENSE`.
