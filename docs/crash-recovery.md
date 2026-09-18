# Crash recovery

What happens when XP FLOW is killed — crash, forced quit, OS shutdown — while a publication is mid-upload, and how
it never turns into a duplicate remote post or a publication silently stuck forever.

## The two questions recovery has to answer

A process killed mid-upload leaves a `Publication` row stuck at `status = 'uploading'` or `'processing'`, with no
way to tell, from that status alone, "how far did this actually get?" Recovery exists to answer two separate
questions honestly, in order:

1. **Is this claim actually abandoned**, or is another process still legitimately working on it?
2. **If abandoned, did the provider ever receive any bytes** — and if so, how much, and did it perhaps already
   fully succeed before the crash?

Guessing wrong on either question is exactly how a duplicate post or a lost publication happens, so neither is
ever assumed — both are checked against real state.

## Detecting an abandoned claim

`try_claim_due` sets `lease_expires_at` (`now + CLAIM_LEASE_DURATION`, 45 minutes — generous relative to how long a
real upload realistically takes, since a crash is recovered correctly regardless of how long the lease is) at claim
time. `PublicationRepository::list_with_expired_leases(workspace_id, now)` finds every row still `Uploading`/
`Processing` whose lease has passed — that's the candidate list, not a guess based on wall-clock time since the app
last ran.

`PublishingEngineService::recover_interrupted(workspace_id)` runs this scan; `lib.rs` calls it once at startup
_before_ the periodic scan starts claiming anything new (section 88/123), and nothing else calls
`recover_one` directly — recovery only ever acts on a lease that has actually expired.

## Resolving what actually happened

For each expired claim, `recover_one` loads the most recent `UploadSession` for that publication and checks
`RemoteUploadState::safe_to_restart()`:

- **`NotStarted` / `Initialized` / `RemoteFailed`** — nothing was ever durably transferred (or the provider
  explicitly reported failure). The claim is released, the publication marked `Failed`, and it's requeued through
  the normal retry chain. No provider call is made — there's nothing to verify.
- **Anything else** (`Transferring`, `Transferred`, `RemoteProcessing`, `RemoteSucceeded`, `RemoteUnknown`) — a
  remote write may already exist. `recover_one` loads the real video file from disk and calls
  `PlatformPublisher::recover_upload(access_token, session, video)`, which is where the actual verification
  happens, per provider:
  - **YouTube**: an empty `PUT` with `Content-Range: bytes */TOTAL` against the resumable session URL returns
    either a `308` with the real committed byte range (resume from there), a `200`/`201` with the finished video
    resource (it was already done — capture the video id, don't re-upload), or a `404` (session unavailable; the remote
    result remains unknown, so automatic restart is refused).
  - **TikTok**: no byte-range probe exists in its API, so the transfer's own `publish_id` status is checked first;
    if that's inconclusive, resuming continues from the last chunk this process itself confirmed a `2xx` for
    (never a chunk that was only queued to send).
  - **Kwai**: `api/upload/resume` returns the server's own confirmed `fragment_index`/`fragment_index_bytes`
    directly — the most precise of the three — which is what recovery trusts over any locally cached progress.

Whatever `recover_upload` determines — resumed and completed, confirmed already succeeded, confirmed failed, or
still genuinely unresolvable — is written back through the same guarded `update_execution_state` path every other
status transition uses. Nothing here is a special-cased write.

## The one case recovery refuses to guess on

If `recover_upload` itself returns an error (a network failure calling the verification endpoint, or — as
documented for Kwai — no verification endpoint exists at all for a publish attempt that never captured a
`photo_id`), the publication is marked `Failed` with `PublishError::UnknownRemoteResult` and its claim released
**without** being requeued for automatic retry. `UnknownRemoteResult::is_retryable()` is `false` by design (see
`docs/publishing-engine.md`): an automatic retry here is exactly the scenario that creates a duplicate post if the
original request actually landed. That publication sits in `Failed` until a human checks the connected account and
retries manually — a deliberately conservative failure mode, not an oversight.

## What this is verified against

`an_interrupted_upload_is_recovered_and_completes_exactly_once` (in `publishing_engine_service`'s test module)
forces a real expired-lease scenario — a session manually re-armed as `Transferring` plus a claim whose
`lease_expires_at` is in the past — and asserts that no matter which path `recover_interrupted` takes, at most one
attempt ever ends up marked `Succeeded`. This is the concurrency-safety property the whole claim/lease/session
design exists to guarantee, exercised directly rather than inferred from reading the code.
