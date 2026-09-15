# Development Guidelines

Conventions for working in this codebase. Most of these exist because violating them quietly reintroduces the
architectural problems Phase 1 was built to avoid — see `docs/architecture.md` for the reasoning behind the
boundaries themselves.

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
9. **No placeholder implementation may pretend to be a complete feature.** If something isn't implemented (Calendar,
   Comments, Analytics, Automation, platform connectors), it says so explicitly in the UI/code — see
   `src/components/common/placeholder-page.tsx` and `infrastructure::connectors::StubConnector`.
10. **No silent failures.** Every `Result`/`Promise` that can fail either gets handled or is allowed to propagate as
    a typed error (`AppError` on the backend, an `AppError`-shaped rejection on the frontend) — never swallowed with
    an empty `catch {}`.

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
  from real domain DTOs (`src/types/domain.ts`), and is expected to be deleted the moment the screen it backs is
  wired to a real repository/command.
- **State**: reach for a Zustand store (`src/stores/`) for state shared across components/routes; use local
  `useState` for state that belongs to one component subtree.
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

TypeScript strict mode (`strict: true`, `noUncheckedIndexedAccess: true`, `noUnusedLocals`, `noUnusedParameters`) is
on — don't add `any` unless there is truly no better option, and don't silence a warning without a comment saying
why.
