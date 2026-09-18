# Phase 5.2 — COMPLETE

## Engineering Verification

**COMPLETE.** The scoped publishing hardening is implemented and verified by 373 passing tests (261 desktop Rust, 25 Auth Broker, 87 frontend), a frontend production build, a Rust release build and a macOS Tauri app bundle. The publishing path, persistence, recovery, retry and provider adapters were audited; the engineering defects found in scope were fixed. This is development phase completion, not a production release claim.

## Release Validation

**PENDING.** Live Google OAuth, one controlled YouTube publication, native macOS visual QA and Windows runtime QA are tracked as RG-01 through RG-04 in [release-validation.md](release-validation.md). They require credentials, user interaction or runtime permissions unavailable during the engineering pass. No live upload, native visual inspection or Windows runtime test is claimed here. These gates must pass before production readiness; they do not keep Phase 5.2 engineering open.

Date: 2026-09-18. Starting commit: `30104e9` on `main`, matching `origin/main`; the tree was clean before this work. This record distinguishes local tests from a real provider publication.

## Automated baseline

- `npm run typecheck && npm run lint && npm run format:check && npm run test && npm run build`: passed; 87 frontend tests in 17 files. Vite reports a non-fatal 500 kB chunk warning.
- `cd src-tauri && cargo fmt --check && cargo clippy --all-targets && cargo test`: passed at baseline; 259 library tests, no binary or doc tests.
- `cd services/auth-broker && cargo fmt --check && cargo clippy --all-targets && cargo test`: passed; 25 tests (9 unit, 16 integration).
- There are no Python source files or Python test/tool configuration in this checkout; no Python gate applies.
- After the targeted fixes, the same Rust gate passed with 261 library tests; the frontend gate remained at 87 passing tests. Final total: 373 passing tests, with zero failures.
- `npm run tauri:build -- --bundles app` passed on macOS: release binary and `src-tauri/target/release/bundle/macos/XP FLOW.app` were produced. Tauri warned that bundle identifier `com.xpflow.app` ends in `.app`; this did not block the bundle.
- `npm run tauri:dev` launched `target/debug/xp-flow`; `open -a` launched the packaged binary. Screen Recording permission was denied by macOS, so no pixel-level native visual QA was possible. Window launch alone is not visual QA.
- The native development process logged successful database migrations and a configuration warning for unset `TIKTOK_CLIENT_KEY`; no provider token value was printed. The packaged process remained alive after launch.

## Actual publication path

`PublicationDetailsDrawer` → `usePublishNow` / `publishingApi.publishNow` → Tauri `publish_now` → `PublishingEngineService::publish_now` → SQLite `try_claim_due` → `execute_inner` → credential acquisition, rendered metadata and consent checks → configured `PlatformPublisher` → persisted attempt and upload session → provider upload/finalize → `Processing` → periodic `poll_processing` → provider status → guarded SQLite `Published` transition → TanStack Query invalidation/polling and Tauri progress events in the UI.

The scheduler reaches the same claim/execute path via `scan_and_claim_due`. Startup calls `recover_interrupted` before periodic scans. YouTube production wiring uses `YouTubeUploader::new()` with Google endpoints. TikTok and Kwai use their real uploaders only when configured; otherwise `StubPublisher` returns a visible non-retryable error. `FakePublisher` is test-only. No production path reports simulated success.

YouTube creates a resumable URL, sends 8 MiB chunks, obtains the video ID from the final response and polls the `videos` endpoint for processing success. A transferred response enters `Processing`; bytes sent alone do not set `Published`. The session stores the remote video ID before the publication status transition. The client defaults privacy to `private` unless configured metadata supplies another value.

## Persistence and safety findings

- SQLite's conditional `try_claim_due` prevents concurrent claims of the same publication. `execution_key` is retained by `COALESCE` on retry and tested across repository reads. It is a local identity; YouTube does not accept it as a remote idempotency key. A deliberately fresh repost flow is not implemented.
- Attempts have a unique `(publication_id, attempt_number)` constraint and persisted provider, timestamps, status, errors and remote operation ID. The attempt table has no separate execution-key column; the publication row carries that identity.
- The resumable session is now persisted with a conservative `transferring` state before the first upload request. The former code stored an `initialized` session and only updated it after upload returned, so a crash after a provider write could be misread as safe to restart. An ambiguous upload error is now `UNKNOWN_REMOTE_RESULT`, is not automatically requeued, and a direct backend `publish_now` call also refuses it.
- An unavailable YouTube resumable session (`404`) after a possible write is now treated as an unknown remote result. A missing session URL cannot prove the final video was never created. A focused uploader test covers this case.
- Processing status polls now close the corresponding persisted attempt on confirmed success/failure. Recovery does the same when it confirms terminal remote success/failure.
- Live progress events are transient. The session's byte offset is currently checkpointed when `upload_media` returns, not after every acknowledged chunk. Restart recovery uses the provider's resumable session probe rather than trusting that offset. A session URL must remain valid for automatic reconciliation; unresolved results fail closed.
- Rate state and retry timing are stored in SQLite. Publish scans run every 30 seconds and processing polls every 60 seconds. `Retry-After` delay-seconds is parsed; HTTP-date is not supported and falls back to normal retry policy.

## Authentication and manual verification

YouTube uses the system browser, PKCE and a loopback callback, then keychain-backed local credentials and SQLite account state. TikTok/Kwai OAuth exchange uses the Auth Broker. The automated suite covers connect, denial/failure, refresh races, broker exchange/replay/revoke and credential error mapping. The native OAuth callback, a real Google account, an actual uploaded video, provider processing, restart after real remote creation and a Windows runtime were **not** verified in this pass. No live credentials or provider account actions were used.

For a real YouTube smoke test: launch the packaged app; connect a Google account and grant the requested upload scopes; import a short owned video; configure a private YouTube publication; inspect metadata and readiness; click Publish now and confirm; observe upload and processing; verify the video ID and processing result in Google and Publication Details; quit and reopen the app; check publication status, execution key and attempt history in SQLite and the UI. Test a deliberate interruption with a disposable private video only after the normal path succeeds. Do not interpret a progress event or complete byte transfer as remote success.

## Deferred release gates and known limits

- RG-01/RG-02 require a real provider credential/consent and a disposable video for live API confirmation.
- RG-03 needs native macOS visual access. The permission preflight reported Screen Recording denied during this pass.
- RG-04 needs Windows runtime evidence; it was not manually exercised on this macOS machine.
- This checkout has no `.github/workflows` directory, so no repository Windows CI result was available as substitute evidence.
- There is no explicit safe reconciliation action for a failed `UNKNOWN_REMOTE_RESULT`; the backend now blocks blind retry. Manual provider inspection may be needed to resolve such a publication.
- There is no explicit fresh-execution repost command. This is a future domain action, not part of this verification pass.

These release gates remain pending in [release-validation.md](release-validation.md). Phase 5.2 engineering is complete.

## Continued verification on 2026-09-18

The repository changes above were preserved without a reset, checkout, stash, clean or revert. A new macOS capture preflight and direct app capture both still failed. The preflight returned exit code 2 with: `Screen Recording is still not granted. Open System Settings > Privacy & Security > Screen Recording and enable it for your terminal (and Codex if needed), then rerun your screenshot command.` The app capture returned exit code 1 with the same permission denial. Accessibility inspection also failed with macOS error `-1728` (`osascript` was not permitted assistive access). No native screen or visual state was inspected and no screenshot evidence was produced.

The packaged application was running. The development runtime launched again, applied database migrations and emitted no immediate panic. The native WebKit process held an established connection to Vite on port 1420, confirming that the development WebView loaded the frontend; this alone does not prove a successful Tauri IPC request. A read-only SQLite check found one workspace and zero channels, videos, publications or YouTube accounts. No media or authenticated account was available for a controlled upload, so OAuth and provider publication were not started. A disposable video path and a time for the user's Google consent were requested; no credential, token or remote publication was created by this verification.

The frontend gate passed again with 87 tests. The Auth Broker gate passed again with 25 tests. The desktop Rust format, clippy and test gate passed with 261 tests, and `cargo build --release` passed. The existing macOS Tauri bundle from the prior pass remains the latest verified bundle for this unchanged source tree. Live provider, visual, restart-after-publication and Windows evidence remain **NOT VERIFIED**.

## Additional preparation on 2026-09-18

A further macOS capture preflight still returned exit code 2 and `Screen Recording is still not granted`; direct `screencapture -x` returned `could not create image from display` and produced no image. The native development process remained running, while a read-only database check still found zero channels, videos, publications and YouTube accounts. No OAuth or remote upload was started.

The local media ingestion dependency was absent, so Homebrew `ffmpeg` 9.0.1_1 (including `ffprobe`) was installed outside the repository. A synthetic 5-second 640×360 MP4 was prepared at `/tmp/xpflow-phase52-private-smoke.mp4`, with SHA-256 `3830546c6f98b943ffbc3826ebce820d44c0e46ebc4af5bc932040ab45b3dd2e`. It has **not** been uploaded or imported. User approval for one private upload of that exact file and Google consent in the native app were requested. The repository source and tests were not changed by this preparation.

The complete gates were rerun: frontend typecheck/lint/format, 87 tests and production build passed; desktop Rust fmt/clippy, 261 tests and release build passed; Auth Broker fmt/clippy and 25 tests passed; `npm run tauri:build -- --bundles app` produced a fresh macOS bundle. The packaged application then launched from that bundle, logged successful migrations and remained alive; SQLite `PRAGMA quick_check` returned `ok`. A further native capture attempt still failed with `Screen Recording is still not granted`. This does not establish a successful frontend-to-Tauri IPC request or visual QA. No provider data or publication was added to the database.
