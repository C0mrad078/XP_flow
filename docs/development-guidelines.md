# Development Guidelines

Conventions for working in this codebase. Most of these exist because violating them quietly reintroduces the
architectural problems Phase 1 was built to avoid — see `docs/architecture.md` for the reasoning behind the
boundaries themselves.

## Development phase completion and release validation

A development phase is complete when its scoped engineering work, relevant automated tests, supported builds, persistence and documentation are complete, and known engineering defects within scope are resolved. A known in-scope engineering defect still blocks completion.

Third-party credentials, manual provider consent, unavailable operating systems or runtimes, screen capture permissions and other required external interactions do not hold a development phase open indefinitely. Record each unperformed check as a persistent release gate in [release-validation.md](release-validation.md). Release gates must pass before production readiness, but may remain pending while the next development phase begins. Never claim an external test passed without evidence.

## Architectural rules (non-negotiable)

1. **React components never touch SQLite.** All persistence goes through `src/lib/tauri/*` → a Tauri command → an
   application service → a repository.
2. **Pages never implement platform API calls.** Platform integration work belongs behind
   `domain::ports::platform_connector::PlatformConnector`, implemented in `infrastructure::connectors`.
3. **`domain/` imports nothing from `infrastructure/`, `persistence/`, `platform/`, `commands/`, or any UI code.**
   If a domain file needs `sqlx`, `tauri`, or `keyring`, something is in the wrong layer.
4. **Infrastructure may depend on domain abstractions (ports). Domain never depends on infrastructure.** A repository
   implementation (`infrastructure::repositories::Sqlite*Repository`) implements a trait defined in
   `domain::ports::repositories`; it is never the other way around.
5. **Platform-specific OS code stays inside `src-tauri/src/platform/`.** No `cfg!(target_os = ...)` outside that
   module in shared logic.
6. **Never log a secret.** No credential, token, or password value reaches a `tracing::*!` call site — ever.
7. **No giant files.** If a file is doing more than one clear job (e.g. a settings page mixing five unrelated
   sections), split it — see `src/features/settings/sections/*` for the pattern.
8. **Avoid premature abstraction, but keep the boundaries above.** Three similar lines beat a speculative
   abstraction; a domain/infrastructure boundary is not "speculative."
9. **No placeholder implementation may pretend to be a complete feature.** If something isn't implemented (Comments,
   Analytics, Automation, platform connectors), it says so explicitly in the UI/code — see
   `src/components/common/placeholder-page.tsx` and `infrastructure::connectors::StubConnector`.
10. **No silent failures.** Every `Result`/`Promise` that can fail either gets handled or is allowed to propagate as
    a typed error (`AppError` on the backend, an `AppError`-shaped rejection on the frontend) — never swallowed with
    an empty `catch {}`.

## Media/ingestion rules (Phase 2, section 96)

11. **Every import goes through `MediaIngestionService::ingest_path`.** Manual import, drag-and-drop, folder-watcher
    dispatch and reconciliation all call the exact same function (via `JobRunner`) — never write a second "create a
    Video row" code path. If a new import source is added later, it should produce a list of paths and hand them to
    the existing pipeline, not reimplement validation/hashing/thumbnailing.
12. **XP FLOW never renames, moves, transcodes or deletes an original video file.** It reads, probes, hashes and
    previews (section 65). "Remove from XP FLOW" deletes the database row and generated thumbnail only
    (`ContentService::remove`) — confirm this behavior isn't changed without an explicit, separately-confirmed user
    action.
13. **FFprobe/FFmpeg are never called from the frontend.** They're shelled out to only inside
    `infrastructure::media::*`, behind the `MediaProbeService`/`ThumbnailService` ports — React only ever sees the
    typed `MediaProbe` result or a `MediaError`.
14. **Never trust a file extension alone.** `is_supported_video_extension` gates which files even get a stability
    check, but `validation_status` always comes from a real FFprobe result (`classify_validation`) — a `.mp4` that
    fails to probe is `Invalid`/`Corrupted`, not assumed valid because of its name.
15. **Never identify a video by filename or path.** Use the database id, and `content_hash` for identity/duplicate
    checks — filenames get reused, paths move. `file_path` is tracked data, not an identity key (section 31).
16. **Testing the ingestion pipeline never requires FFmpeg to be installed.** Use the fakes in
    `src-tauri/src/test_support.rs` (`FakeProbeService`, `FakeThumbnailService`, `FakeHashService`,
    `FakePerceptualHashService`) behind the same ports the real FFmpeg-backed services implement — see
    `docs/media-library.md` for why.

## Queue/scheduler rules (Phase 3, section 116's "zero-bug mindset")

17. **Every critical scheduling invariant gets a database-level backstop, not just an application-level check.**
    "No double-booked slot" and "no duplicate active publication for the same video/channel/platform" are each a
    partial unique index (`migrations/0004_queue_scheduler.sql`) _in addition to_ the application check — the
    sequential check alone has a TOCTOU window under real concurrency (see `docs/scheduler.md` §4 and its
    real-concurrent-race test). If a new scheduling invariant is added, ask whether it needs the same treatment.
18. **Never do timezone math against the OS/browser's local zone for anything schedule-related.** Resolve through the
    _workspace's_ configured IANA timezone (`Workspace::timezone` / `useWorkspaceStore().workspace.timezone`) —
    `domain::scheduling`/`SchedulerService` on the backend, `formatTimeInZone`/`formatDateInZone`/`localDateKeyInZone`
    on the frontend. Plain `Date`/`Intl` calls without an explicit `timeZone` are for genuinely zone-agnostic things
    only (e.g. calendar _grid_ layout — see `calendar-dates.ts`'s own comment on why it's exempt).
19. **"Overdue" (and any similar operationally-useful-but-not-a-real-state label) is derived, never persisted.**
    Compute it from existing fields (`Publication::is_overdue` / `isPublicationOverdue`) rather than adding a new
    `PublicationStatus` variant or mutating a publication's status to reflect something that hasn't actually failed.
20. **Bulk scheduling operations are transactional; bulk queue-membership operations are per-item.** A partial bulk
    _schedule_ would leave the calendar in a confusing half-filled state, so those use
    `PublicationRepository::bulk_update` (one transaction, all-or-nothing). A partial bulk _add-to-queue_ (some
    videos already queued for that channel/platform) is expected and benign, so that one reports a per-item outcome
    instead. Don't default new bulk operations to one shape without asking which of these two they actually are.

## Platform authentication rules (Phase 4, section 116's "zero-secret mindset")

21. **Never hardcode a provider client secret in the desktop crate.** `TIKTOK_CLIENT_SECRET`/`KWAI_APP_SECRET` are
    broker-only environment variables (`services/auth-broker/`) and must never appear in `src-tauri/`'s source,
    environment, or build output. `YOUTUBE_CLIENT_SECRET` is the one exception — Google's own threat model doesn't
    treat an installed app's secret as confidential — but it's still resolved from an environment variable
    (`YouTubeAuthConfig::resolve()`), never a literal string in source, and the desktop PKCE flow never requires it
    to be set at all. A **client id** is a different thing: it's non-secret by definition, which is why
    `YouTubeAuthConfig::DEFAULT_CLIENT_ID` is a literal in source — that's deliberate, not an exception to this rule.
22. **A secret-shaped type gets a redacting `Debug` impl, not caller discipline.** `LocalCredential` and `Pkce` both
    implement `fmt::Debug` by hand to print `"[redacted]"` for their secret fields — see
    `infrastructure::auth::redact` (desktop) and `redact.rs` (broker) for the shared pattern. If a new type ever
    carries a raw token, access token, refresh token, or PKCE verifier, redact it at the type level; don't rely on
    every future `tracing::debug!` call site remembering not to print it.
23. **`AuthFlowState` (live connect progress), `PlatformAccountStatus` (persisted lifecycle), and
    `ConnectionHealth` (derived display signal) are three different things — don't collapse them.** A connect
    dialog polls `AuthFlowState`. A `PlatformAccount` row's actual state is `PlatformAccountStatus`. Whether to show
    a warning badge is `ConnectionHealth`, computed fresh every time (`derive_connection_health` /
    `deriveConnectionHealth`), never persisted.
24. **"Account connected" and "account can publish" are different claims — check `Capability`, never a raw scope
    string, and never assume `Connected` implies `UploadVideo`.** See `docs/provider-capabilities.md`.
25. **Identity-uniqueness gets the same database-level backstop as Phase 3's scheduling invariants.** The same real
    provider account can't be connected twice in a workspace — enforced by
    `idx_platform_accounts_identity` (partial unique index) in addition to `PlatformAuthService::finish_connect`'s
    application-layer check, for the same TOCTOU reasons as rule 17.
26. **A `Revoked` account row is available history, not a live connection — treat it accordingly when writing new
    identity checks.** It doesn't count as "connected" for the identity-uniqueness check (a fresh Connect for the
    same real account revives it), but it's never deleted (disconnect's whole point is that scheduled publications
    and activity history referencing it survive).
27. **The Auth Broker never becomes a place to add anything besides TikTok/Kwai confidential OAuth handling.** No
    content, queue, schedule, analytics, or workspace state belongs there — if a feature seems to need a
    server-side component, that's a new, separately justified service, not scope creep on this one
    (`docs/auth-broker.md` §7).

## Frontend conventions

- **Only `src/lib/tauri/*.ts` may import `@tauri-apps/api`.** Add a new backend feature by adding a typed wrapper
  function there, not by calling `invoke()` from a component.
- **Design tokens only.** Every color/spacing/radius value a component uses should trace back to a token in
  `src/styles/globals.css`(`bg-surface`, `text-muted-foreground`, `rounded-md`, etc.) — no raw hex codes or arbitrary
  pixel values in component code.
- **`cn()` for conditional classes.** Use `cn(...)` from `src/lib/utilities/cn.ts` (clsx + tailwind-merge) instead of
  manual string concatenation, so conflicting Tailwind utilities resolve predictably.
- **Formatting goes through `src/lib/formatting/`.** Never call `Date` formatting or `Intl` directly on a raw
  backend timestamp inside a component — use `formatDate`/`formatTime`/`formatRelativeTime`/etc., which handle the
  UTC → local conversion in one place.
- **Mock data is isolated.** Anything under `src/development/mock-data/` is UI-only fixture data, typed separately
  from real domain DTOs (`src/types/domain.ts`, `src/types/media.ts`, `src/types/scheduling.ts`), and is expected to
  be deleted the moment the screen it backs is wired to a real repository/command — `content.ts` was deleted when the
  Content Library went real in Phase 2; `queue.ts`/`channels.ts` were deleted the same way in Phase 3. What remains
  (`activity.ts`, `dashboard.ts`'s view/performance widgets) backs screens still genuinely out of scope.
- **State**: backend-derived data goes through TanStack Query (`src/hooks/use-*.ts`), never copied into Zustand
  (section 71) — reach for a Zustand store (`src/stores/`) only for state that has no server representation
  (selection, view mode, which panel is open). Use local `useState` for state scoped to one component subtree.
- **Feature flags**: gate not-yet-real functionality behind `src/lib/utilities/feature-flags.ts` rather than
  commenting code out or leaving a dead UI control.

## Backend conventions

- **Repositories return `DomainResult<T>` (`Result<T, DomainError>`), never a raw `sqlx::Error`.** Map SQLx/serde
  errors to `DomainError::Repository`/`DomainError::Validation` at the point they occur.
- **Commands are thin.** A `#[tauri::command]` function should be: extract `State<AppState>`, call one
  `application`/`services` method, `Ok(...)`/`?` the result. If a command has an `if`/`match` beyond argument
  shaping, that logic belongs in an application service instead.
- **New entity → new port → new SQLite adapter → new migration**, in that order, following the existing pattern
  (see `domain::workspace` / `domain::ports::repositories::WorkspaceRepository` /
  `infrastructure::repositories::SqliteWorkspaceRepository` / `migrations/0001_init.sql`).
- **Timestamps**: store as RFC 3339 UTC strings (`DateTime<Utc>::to_rfc3339()`), parse back with the shared
  `infrastructure::repositories::parse_dt` helper.

## Testing

- **Backend**: `cargo test` from `src-tauri/`. Prioritize domain logic (state machines, validation) and
  persistence round-trips over incidental coverage. Every new repository should have at least one round-trip test
  using a real (temp-file) SQLite database via `persistence::db::init_pool` — not a mock.
- **Frontend**: `npm run test` (Vitest + Testing Library). Prioritize pure logic (`lib/formatting`, `lib/utilities`)
  and the IPC error-normalization boundary (`lib/tauri/client.ts`) over exhaustive component snapshot tests.

## Code quality gates

Run before considering a change done:

```bash
npm run typecheck && npm run lint && npm run format:check && npm run test && npm run build
cd src-tauri && cargo fmt --check && cargo clippy --all-targets && cargo test
```

Touching anything under `services/auth-broker/` runs its own, separate gate (it's not a workspace member, and its
`wiremock`-backed integration tests are the only coverage for the TikTok/Kwai token exchange):

```bash
cd services/auth-broker && cargo fmt --check && cargo clippy --all-targets && cargo test
```

TypeScript strict mode (`strict: true`, `noUncheckedIndexedAccess: true`, `noUnusedLocals`, `noUnusedParameters`) is
on — don't add `any` unless there is truly no better option, and don't silence a warning without a comment saying
why.
