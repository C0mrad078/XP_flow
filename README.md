# XP FLOW

A professional, local-first desktop application for managing networks of short-form content channels — importing
video, organizing it by channel, queuing and scheduling publications across YouTube, TikTok and Kwai, and tracking
what happened along the way.

This repository currently implements **Phase 1: Foundation, Architecture, Desktop Shell and Design System**. It is a
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
| State                   | Zustand                                                                                                            |
| Routing                 | React Router                                                                                                       |
| Backend                 | Rust, Tokio                                                                                                        |
| Database                | SQLite via SQLx, with versioned migrations                                                                         |
| Secure storage          | OS keychain via the `keyring` crate (macOS Keychain / Windows Credential Manager / Secret Service)                 |
| Logging                 | `tracing` + `tracing-subscriber`, JSON file logs plus console output                                               |

## Project structure

```text
xp-flow/
├── src/                        # Frontend (React + TypeScript)
│   ├── app/                    # Root component, routes, providers, layouts
│   ├── components/             # ui/ (design system primitives), navigation/, feedback/, common/
│   ├── features/                # One folder per screen/domain area
│   ├── development/mock-data/  # Isolated mock data for screens not yet wired to real persistence
│   ├── hooks/, lib/, stores/, styles/, types/
│
├── src-tauri/                  # Backend (Rust)
│   ├── src/
│   │   ├── domain/             # Entities, value objects, ports (traits) — no infra dependency
│   │   ├── application/        # Use-case services (workspace, settings, activity)
│   │   ├── services/           # Cross-cutting technical services (media status, notifications)
│   │   ├── infrastructure/     # SQLx repositories, platform connector stubs, media/log infra
│   │   ├── persistence/        # SQLite pool + migration bootstrap
│   │   ├── platform/           # OS-isolated code: data paths, secure storage backend
│   │   ├── jobs/                # Job/JobType/JobStatus vocabulary (no scheduler yet)
│   │   ├── commands/            # Thin Tauri IPC command handlers
│   │   └── lib.rs, main.rs, state.rs, error.rs
│   └── migrations/             # SQL migrations (SQLx)
│
└── docs/                       # architecture.md, development-guidelines.md
```

See `docs/architecture.md` for the full explanation of why the backend is layered this way.

## Development requirements

- [Node.js](https://nodejs.org/) 20+ and npm
- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain) and Cargo
- Platform build tools for Tauri — see the [Tauri prerequisites guide](https://tauri.app/start/prerequisites/) for
  your OS (Xcode Command Line Tools on macOS; WebView2 + MSVC Build Tools on Windows)

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

Inside that directory: `xpflow.db` (SQLite database), `logs/` (structured JSON logs, rotated daily), `cache/`. These
paths are also shown in-app under **Settings → Storage**, and are resolved by `src-tauri/src/platform/paths.rs` —
never hardcoded elsewhere in the codebase.

## Current implementation status (Phase 1)

**Implemented and real:**

- Tauri 2 + React + TypeScript (strict) + Vite shell, cross-platform window defaults
- SQLite persistence with versioned migrations, applied automatically on startup
- Domain model for Workspace, Channel, Platform, PlatformAccount, Video, VideoSource, Publication, QueueItem,
  ScheduleSlot, Template, ActivityEvent, Notification, AppSettings
- A fully-typed `PublicationStatus` state machine with centrally-validated transitions (unit tested)
- Typed IPC boundary (`src/lib/tauri/*` ↔ `src-tauri/src/commands/*` ↔ application services ↔ repositories)
- Real, persisted: workspace creation/onboarding, app settings (theme, launch-on-startup), activity event log
- OS-native secure storage abstraction (Keychain / Credential Manager) — implemented, not yet storing real credentials
- Stub `PlatformConnector` implementations for YouTube/TikTok/Kwai (prove the contract; no real API calls)
- FFmpeg/FFprobe toolchain detection (no transcoding pipeline yet)
- Structured logging to a local JSON log file
- A complete, customized UI component library (not default shadcn) and design token system
- Every Phase 1 screen: Dashboard, Today, Queue (Timeline/List), Content (Grid/List), Channels, Activity (real +
  demo events), Settings, onboarding, plus explicit placeholders for Calendar/Comments/Analytics/Automation
- Command palette, notification center, collapsible sidebar, dark theme (polished) / light theme (structural)

**Deliberately not implemented in Phase 1** (see `docs/architecture.md` for what each needs before it can land):

- Real YouTube/TikTok/Kwai API integration, OAuth, publishing, or metrics/comment sync
- Video transcoding, thumbnail generation, or a Cut.pro folder watcher
- The job scheduler/worker pool (only the `Job`/`JobType`/`JobStatus` vocabulary exists)
- Automation rules, AI of any kind, cloud sync

## License

See `LICENSE`.
