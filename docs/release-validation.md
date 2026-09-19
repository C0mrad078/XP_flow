# Release Validation

These are production readiness gates. A development phase can be complete while a release gate remains pending. Record real evidence here before changing a gate to **PASSED**; automated adapter tests or a successful build do not substitute for a live provider or native runtime check. Phase 5.2 engineering is complete; the gates below are deferred release validation, not unfinished Phase 5.2 tasks.

| Gate  | Scope                    | Status                         | Evidence                                                                                                |
| ----- | ------------------------ | ------------------------------ | ------------------------------------------------------------------------------------------------------- |
| RG-01 | Live Google OAuth        | **PENDING RELEASE VALIDATION** | No real account authorization was completed during Phase 5.2.                                           |
| RG-02 | Live YouTube publication | **PENDING RELEASE VALIDATION** | No remote video was created during Phase 5.2.                                                           |
| RG-03 | Native macOS visual QA   | **PENDING RELEASE VALIDATION** | The native app launched, but screen capture and visual inspection were unavailable.                     |
| RG-04 | Windows runtime QA       | **PENDING RELEASE VALIDATION** | No Windows runtime or CI evidence was available.                                                        |
| RG-05 | Live provider analytics  | **PENDING RELEASE VALIDATION** | Analytics adapters are covered by automated tests; no credential-backed analytics session has been run. |

## RG-01 — Live Google OAuth

On a real account, verify XP FLOW opens the system browser, receives the loopback callback, persists the connected account and credential reference, and reflects that account in the native UI. Exercise refresh and reconnect/revoke behavior when appropriate without exposing or recording tokens. Record the account identity in a privacy-safe form, dates, runtime and result.

## RG-02 — Live YouTube publication

With a user-approved disposable video, publish **once** as private or unlisted through the existing XP FLOW flow. Verify the returned remote video ID, metadata and privacy at Google, provider processing, authoritative `Published` transition, persisted ID and attempt history, and the UI result. Restart XP FLOW and verify the state, execution key and ID remain correct and no duplicate video or republish occurs. Record evidence without printing credentials or repeating the video ID unnecessarily. The detailed preparation and observation procedure remains in [phase-5.2-verification.md](phase-5.2-verification.md).

## RG-03 — Native macOS visual QA

Inspect the actual Tauri app at supported window sizes: Dashboard, Today, Publishing Queue, Publication Details, Settings → Publishing, account connection, authentication, progress, retry, failure, success, diagnostics and consent states where reachable. Record screenshots or a manual inspection log, concrete defects and their fixes. Browser-only rendering is insufficient.

## RG-04 — Windows runtime QA

Run the packaged app on Windows. Check startup, database and credential storage, provider authentication callback, publishing UI and recovery behavior. Record Windows version, build artifact and results. A macOS build does not satisfy this gate.

## Gate handling

Each gate remains **PENDING RELEASE VALIDATION** until its specific evidence is recorded and reviewed. A failed gate creates a release issue or a focused bugfix. Do not infer success from tests, a local SQLite status, a provider mock or another operating system. Production readiness requires all applicable gates to pass.
