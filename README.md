# XP FLOW

A professional, local-first desktop application for managing networks of short-form content channels — importing
video, organizing it by channel, queuing and scheduling publications across YouTube, TikTok and Kwai, and tracking
what happened along the way.

This repository currently implements **Phase 1 (Foundation, Architecture, Desktop Shell and Design System)** and
**Phase 2 (Media Library, Cut.pro Folder Watcher, FFmpeg Integration and Local Content Management)**. It is a
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
│   │   │                       # media_ingestion_service (the one ingestion pipeline every import path uses)
│   │   ├── services/           # Cross-cutting technical services: media status, notifications, job_runner
│   │   │                       # (bounded-concurrency import + folder-watcher dispatch + reconciliation)
│   │   ├── infrastructure/     # SQLx repositories, platform connector stubs, media (FFprobe/FFmpeg), hashing
│   │   │                       # (SHA-256 + perceptual), filesystem (stability/cache/reveal), watcher, logging
│   │   ├── persistence/        # SQLite pool + migration bootstrap
│   │   ├── platform/           # OS-isolated code: data paths, secure storage backend
│   │   ├── jobs/                # Job/JobType/JobStatus/JobRepository — executes real ingestion jobs (Phase 2)
│   │   ├── commands/            # Thin Tauri IPC command handlers + the xpflowmedia:// local preview protocol
│   │   └── lib.rs, main.rs, state.rs, error.rs, test_support.rs
│   └── migrations/             # SQL migrations (SQLx)
│
└── docs/                       # architecture.md, media-library.md, development-guidelines.md
```

See `docs/architecture.md` for the full explanation of why the backend is layered this way, and
`docs/media-library.md` for the ingestion pipeline, folder watching and duplicate detection specifically.

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

## Current implementation status

**Implemented and real:**

- Tauri 2 + React + TypeScript (strict) + Vite shell, cross-platform window defaults
- SQLite persistence with versioned migrations, applied automatically on startup
- Domain model for Workspace, Channel, Platform, PlatformAccount, Video, VideoSource, Publication, QueueItem,
  ScheduleSlot, Template, ActivityEvent, Notification, AppSettings, DuplicateMatch, Job
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
- OS-native secure storage abstraction (Keychain / Credential Manager) — implemented, not yet storing real credentials
- Stub `PlatformConnector` implementations for YouTube/TikTok/Kwai (prove the contract; no real API calls)
- Structured logging to a local JSON log file
- A complete, customized UI component library (not default shadcn) and design token system
- Every screen: Dashboard, Today, Queue (Timeline/List, still mock — see below), Content (real, Grid/List), Channels
  (mock UI over real list/create commands), Activity (real + demo events), Settings, onboarding, plus explicit
  placeholders for Calendar/Comments/Analytics/Automation
- Command palette, notification center, collapsible sidebar, dark theme (polished) / light theme (structural)

**Deliberately not implemented yet** (see `docs/architecture.md` for what each needs before it can land):

- Real YouTube/TikTok/Kwai API integration, OAuth, publishing, or metrics/comment sync
- Video transcoding (FFmpeg detection and thumbnail extraction exist; re-encoding does not)
- Bundled FFmpeg/FFprobe binaries in packaged builds (the sidecar-resolution logic exists; nothing is bundled)
- The real queue/scheduler and the Queue screen's backing data (still Phase 1 mock) — `QueueItem`/`ScheduleSlot`
  exist in the schema, and the job foundation Phase 2 exercised for ingestion is the same one a scheduler would use
- The full Channels screen (health indicators, platform connections) — still Phase 1 mock UI, now sitting on top of
  real channel rows
- Automation rules, AI of any kind, cloud sync

## License

See `LICENSE`.
