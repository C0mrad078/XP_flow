# YouTube publishing

`infrastructure::connectors::youtube::uploader::YouTubeUploader` implements `PlatformPublisher` for YouTube, talking
directly to `googleapis.com` — no broker involvement (the broker only exists for TikTok/Kwai's confidential OAuth
exchange; YouTube's OAuth client is already public/desktop-flow, per `docs/platform-authentication.md`). Built
against Google's documented resumable upload protocol, verified against the official docs at the time this was
written — not assumed from prior training knowledge.

## The flow

1. **Initialize** (`POST .../upload/youtube/v3/videos?uploadType=resumable&part=snippet,status`, with
   `X-Upload-Content-Length`/`X-Upload-Content-Type` headers and the video's `snippet`/`status` JSON body). Success
   returns a `Location` header — the resumable session URL — captured onto the `UploadSession` immediately, before
   any bytes are sent.
2. **Upload**, in 8 MiB chunks (`CHUNK_SIZE`; every chunk but the last must be a multiple of 256 KiB per Google's
   protocol), each a `PUT` to the session URL with `Content-Range: bytes {first}-{last}/{total}`:
   - `308 Resume Incomplete` on every non-final chunk — the expected steady-state response.
   - `200`/`201` with the created video resource on the final chunk — the video id is captured directly from this
     response body.
   - `404` after a transfer may have begun — the session is unavailable, but the final write might still have
     succeeded. It is treated as `UnknownRemoteResult` and is not blindly restarted.
   - `5xx` — `PublishError::ProviderServerError`, retryable.
3. **Finalize** is a no-op passthrough — the final chunk's response _is_ the created resource; there's no separate
   "publish" call the way TikTok/Kwai have one.
4. **Status polling** (`GET .../youtube/v3/videos?part=status,processingDetails&id=...`) reads
   `processingDetails.processingStatus`: `"succeeded"` → `RemoteSucceeded`, `"failed"`/`"terminated"` →
   `RemoteFailed`, anything else (including an unrecognized future value) → `RemoteProcessing`, never guessed as
   done.
5. **Recovery** sends an empty `PUT` with `Content-Range: bytes */TOTAL` to the session URL before resuming
   anything — Google's own documented way to ask "how many bytes have you actually received," never trusting a
   locally cached `bytes_committed` after a crash. See `docs/crash-recovery.md`.

## Defaults and validation

- `validate_metadata`: title must be non-empty and ≤100 characters (YouTube's documented limit); description
  ≤5,000 characters.
- `initialize_upload` defaults to `privacyStatus: "private"` and a generic `categoryId: "22"` when the (not yet
  wired) metadata-template system hasn't supplied `provider_options.privacy_status`/`category_id` — never publishes
  more broadly than explicitly configured.
- `validate_media` enforces a generous 256 GB sanity bound, not YouTube's actual (verification-status-dependent)
  limit — the provider's own rejection is the authoritative check.

## Testing

7 tests run against a local `wiremock` server (`with_base_url`, test-only): init capturing the `Location` header, a
single-chunk upload, a multi-chunk upload exercising real `308` continuation with an injectable chunk size (so
coverage doesn't require an 8 MiB+ fixture file), conservative processing-status mapping across known and unknown
status strings, and recovery querying the real committed range before resuming.

## Real-provider verification status

Not exercised against a live Google account — no credentials were available in this environment. All test coverage
is against `wiremock`, not the real API.
