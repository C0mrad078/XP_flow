# Kwai publishing — a documentation-access limitation, stated honestly

`infrastructure::connectors::kwai::uploader::KwaiUploader` implements `PlatformPublisher` for Kwai. Unlike the
YouTube and TikTok uploaders, it was **not** built against Kwai's own developer documentation — that documentation
was not reachable from this environment. This page exists so nobody mistakes the Kwai uploader for the same tier
of verification as YouTube's or TikTok's.

## What it's actually grounded in

Kwai and Kuaishou share the same open platform (`open.kuaishou.com`). A third-party, unofficial Go client,
[`github.com/bububa/kwai-openapi`](https://github.com/bububa/kwai-openapi), wraps that platform's video-publishing
endpoints. `KwaiUploader` mirrors the request/response shapes that client's source code shows, verified by reading
its `api/video` and `model/video` packages directly (not summarized secondhand). That is a real, checkable source —
but it is still someone else's reverse-engineering of a private API, not Kwai's own contract, and it could be
incomplete, outdated, or simply wrong in places this project has no way to detect without live credentials.

## The flow implemented

1. `POST /openapi/photo/start_upload` (query: `app_id`, `access_token`) → `{ upload_token, endpoint }`. This is
   `initialize_upload` — a durable `UploadSession` (`remote_session_id` = `upload_token`, `remote_upload_url` =
   `https://{endpoint}`) exists before any bytes are sent, per the no-bytes-before-durable-state rule the other two
   uploaders also follow.
2. `POST https://{endpoint}/api/upload/fragment?upload_token=..&fragment_id=..`, one fragment at a time, body = raw
   bytes. This is `upload_media`.
3. `POST https://{endpoint}/api/upload/complete?upload_token=..&fragment_count=..` to close out the transfer, then
   `POST /openapi/photo/publish` (multipart form: `cover`, `caption`, `stero_type`; query: `app_id`, `access_token`,
   `upload_token`) to actually create the post. This is `finalize_publication` — and it is the exactly-once-critical
   step: nothing public exists until `publish` succeeds, so a transport failure on that specific call is mapped to
   `PublishError::UnknownRemoteResult`, never silently retried.
4. `GET /openapi/photo/list` (paginated) is the only status surface this API exposes. `get_remote_status` scans a
   bounded number of pages looking for a matching `photo_id`, checking that video's `pending` flag. If the video
   isn't found within the scan, the result is `RemoteUnknown` — never guessed.

## What is explicitly an assumption, not a verified fact

- **Fragment size** (`FRAGMENT_SIZE = 4 MiB`): no per-fragment size constraint is documented anywhere in the
  available source. This is a conservative default chosen for reasonable memory/request-duration behavior, not a
  Kwai-imposed limit.
- **Error classification** (`classify_result`): the API returns a numeric `result` code and a free-text
  `error_msg`, with no enumerated error-code table available. Classification is best-effort substring matching on
  the message text (e.g. "token" + "expire" → `AuthExpired`), not a verified mapping the way TikTok's documented
  `code` strings are.
- **Caption length**: no limit was found in the available source, so `validate_metadata` only rejects an empty
  caption — it does not enforce a maximum the way the YouTube (100 chars) and TikTok (2200 UTF-16 units) uploaders
  do.
- **Cover image**: `publish` takes a `cover` field; nothing in the available source describes a separate
  cover-upload step, so it is currently sent empty. Whether Kwai auto-generates a cover frame from the video in
  that case is unverified.
- **A known, load-bearing gap**: there is no endpoint in the available source to look up "did `upload_token` X ever
  get published" after the fact. If `finalize_publication`'s `publish` call fails ambiguously (timeout, connection
  drop) _before_ a `photo_id` is captured, `recover_upload` cannot verify the outcome at all — it returns the
  session as `RemoteUnknown` and stops there. That publication requires a human to check the connected Kwai account
  directly before anything touches it again. This is the honest, safe behavior given the gap — not a bug to be
  silently patched over with a guess.

## App credentials

Kwai's video-publish calls take `app_id` as a plain request parameter (not derived from the bearer token, unlike
TikTok's API). `KwaiPublishConfig::resolve()` reads the non-secret `KWAI_APP_ID` from the desktop's own
environment — the same public/confidential split as TikTok's `client_key`/`client_secret`
(`docs/platform-authentication.md`). `KWAI_APP_SECRET` stays broker-side only; the desktop never sees it.

## Real-provider verification status

None of the above has been exercised against a live Kwai account — no credentials were available in this
environment. All 7 `KwaiUploader` tests run against a local `wiremock` server modeling the shapes described above,
not the real API.

## Phase 5.1 re-check

Phase 5.1 explicitly asked for a fresh attempt at reaching Kwai's official documentation before reconciling or
retaining this limitation. Two independent attempts were made: `open.kuaishou.com`'s open-platform page (the same
host `KwaiUploader` talks to) returned only a client-rendered shell with no fetchable documentation content, and
`kwai.com/explore/kwai-business-api-documentation` (surfaced by search as a plausible lead) turned out to be a
general marketing page with no API reference at all. Neither attempt surfaced real, authoritative documentation.
The implementation is therefore left exactly as it was — grounded in the unofficial `bububa/kwai-openapi` client,
not rewritten based on guesswork — per the explicit instruction not to replace a working conservative
implementation with speculation just because the gap is uncomfortable.
