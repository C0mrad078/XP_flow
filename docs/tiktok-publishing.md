# TikTok publishing

`infrastructure::connectors::tiktok::uploader::TikTokUploader` implements `PlatformPublisher` for TikTok's Content
Posting API, talking directly to `open.tiktokapis.com`. The Auth Broker is only ever used for the confidential
OAuth token exchange (`docs/auth-broker.md`) and to issue the bearer access token this uploader uses — it never
sees the video bytes or the publish call itself (local-first: desktop → TikTok directly).

## Why TikTok is different: express consent

TikTok's Content Posting API terms require the posting app to have the user's explicit, per-post approval before
transmitting — not just a granted OAuth scope. `requires_express_consent(Platform::TikTok)` (the only platform this
is true for) makes this a hard gate in the engine: a TikTok publication whose current rendered metadata isn't
covered by a `PublicationConsent` row fails closed with `PublishError::ConsentRequired` before any network call,
including before creating a `PublicationAttempt`. Consent is recorded via the `record_publication_consent` command
and covers metadata by content hash, not a boolean flag — editing a title after approval makes the old consent stop
covering it automatically (`PublicationConsent::covers`), with no separate "invalidated" state to remember to set.
See `docs/publishing-engine.md`.

## The flow

1. **Creator info** (`POST /v2/post/publish/creator_info/query/`) — fetched before every init call to find out
   which privacy levels this specific connected creator account is actually allowed to post at, and whether
   duet/stitch/comments are already disabled account-wide. If the metadata's requested `privacy_level` isn't in
   the account's allowed list, this falls back to the most restrictive allowed option (`SELF_ONLY` if available)
   rather than either erroring or silently posting more broadly than requested.
2. **Init** (`POST /v2/post/publish/video/init/`) with `post_info` (title, privacy level, duet/stitch/comment
   toggles, cover timestamp) and `source_info` (`source: "FILE_UPLOAD"`, `video_size`, `chunk_size`,
   `total_chunk_count`, from the chunk planner below). Returns `publish_id` and a 1-hour-valid `upload_url`, both
   captured onto the `UploadSession` before any bytes are sent.
3. **Chunk upload** — sequential `PUT`s to `upload_url` with `Content-Range: bytes {first}-{last}/{total}`.
4. **Status** (`POST /v2/post/publish/status/fetch/` with `publish_id`) — `"PUBLISH_COMPLETE"`/
   `"SEND_TO_USER_INBOX"` → `RemoteSucceeded`, `"FAILED"` → `RemoteFailed`, everything else (including
   `"PROCESSING_UPLOAD"`/`"PROCESSING_DOWNLOAD"` and any value not in this list) → `RemoteProcessing`. The full
   status enum wasn't independently confirmable against current official docs at the time this was written; this
   mapping stays conservative — never guesses success on an unrecognized string.

## Chunk planning

TikTok's Media Transfer Guide documents: 5–64 MiB per chunk, the final chunk may run up to 128 MiB to absorb the
remainder, 4 GiB max total size, 1,000 chunks max, and anything under 5 MiB uploads as a single whole-file chunk.
`plan_chunks` (`tiktok::uploader`) is the one place this arithmetic lives — it always picks the full 64 MiB chunk
size for any file at or above the 5 MiB floor, which keeps every chunk count comfortably under the 1,000 cap for
anything up to the 4 GiB total limit.

## Validation and error mapping

- `validate_metadata`: caption non-empty, ≤2,200 UTF-16 code units (TikTok's documented cap).
- API errors carry a `code` string; `classify_api_error` maps the documented ones directly —
  `access_token_invalid`/`scope_not_authorized` → `AuthExpired`, `rate_limit_exceeded` → `RateLimited`,
  `unaudited_client_can_only_post_to_private_accounts` → `PlatformNotApproved` (an app that hasn't cleared TikTok's
  own review is restricted to private-only posting), `invalid_param`/`invalid_file_upload` → `InvalidMedia`.

## Recovery

TikTok exposes no byte-range status-check endpoint the way YouTube does. Recovery instead checks the publish status
by `publish_id` first (if the transfer had already reached that point, the outcome is known directly); otherwise it
resumes from the last chunk offset this process itself confirmed with a `2xx` response — never a chunk that was
merely queued to send. See `docs/crash-recovery.md`.

## Testing

9 tests run against a local `wiremock` server: init capturing `publish_id`/`upload_url`, the privacy-level fallback
behavior, a single-chunk upload, conservative status mapping across known/unknown values, the unaudited-app error
mapping to `PlatformNotApproved`, metadata validation, and the chunk planner's boundary cases (under the 5 MiB
floor, the 4 GiB cap, the 64 MiB chunk size for a large file).

## Real-provider verification status

Not exercised against a live TikTok account — no credentials were available in this environment. All test coverage
is against `wiremock`, not the real API. TikTok's own docs were reachable and used directly for this uploader
(unlike Kwai's — see `docs/kwai-publishing.md`), but live behavior can still diverge from documentation in ways
only a real account would surface (app review status, rate limit specifics, exact status-string enumeration).
