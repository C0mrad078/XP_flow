# Media Library (Phase 2)

This document covers the local media ingestion pipeline, folder watching,
duplicate detection and cache management added in Phase 2. See
`docs/architecture.md` for how this fits into the rest of the backend; this
file goes one level deeper on the parts that are new.

## 1. The ingestion pipeline

Every way a video enters XP FLOW — manual "Import Videos", drag-and-drop, a
watched folder's filesystem events, or folder reconciliation — converges on
exactly one function: `MediaIngestionService::ingest_path`
(`src-tauri/src/application/media_ingestion_service.rs`). This is
deliberate (section 13 of the Phase 2 brief): there is nowhere else in the
codebase that creates a `Video` row, so validation, duplicate detection and
thumbnail generation can never drift out of sync between entry points.

```text
path
  │
  ▼
extension check (mp4/mov/mkv/webm only)
  │
  ▼
already indexed at this exact path? ──yes──▶ AlreadyIndexed (no-op)
  │ no
  ▼
FFprobe (MediaProbeService)
  │
  ▼
SHA-256 content hash, streamed (ContentHashService)
  │
  ▼
existing video with this hash? ──yes──┬─▶ old file still exists ─▶ Duplicate
  │ no                                 └─▶ old file gone ─▶ Moved (updates the old row's path)
  ▼
classify Valid/Invalid from probe (duration/width/height > 0)
  │
  ▼
thumbnail (ThumbnailService, ~10% into the clip, never frame 0)
  │
  ▼
perceptual hash of the thumbnail (PerceptualHashService) — best effort
  │
  ▼
persist Video row
  │
  ▼
compare perceptual hash against the workspace's last 500 videos ─▶ DuplicateMatch rows (match_type = possible) above a similarity threshold
```

The function returns an `IngestOutcome` (`Created` / `Duplicate` /
`Moved` / `AlreadyIndexed` / `Rejected`) rather than a `Result<Video, _>` —
"this file is already in your library" is a normal, expected outcome, not
an error, and callers (the job runner, the import command) branch on it
explicitly instead of unwrapping.

**File stability is not this function's job.** A file appearing in a
watched folder does not mean the exporting tool finished writing it
(section 14). `services::job_runner::JobRunner` runs
`infrastructure::filesystem::FileStabilityChecker::wait_until_stable`
_before_ calling `ingest_path` for anything that came from the watcher or a
one-off folder walk — it polls the file's size on an interval
(`StabilityConfig`, default 500ms × 3 consecutive unchanged reads, ~20s
ceiling) and only proceeds once the size has stopped moving and the file
opens cleanly. Manual "Import Videos" selections skip nothing — the same
check runs there too, since a user can in principle select a file another
process is still writing.

## 2. Folder watching and reconciliation

`infrastructure::watcher::FolderWatcherService` wraps `notify` +
`notify-debouncer-full`, watching every enabled, folder-backed
`VideoSource` at once. Multiple roots share one OS watcher instance;
`resolve_source` maps a changed path back to its owning source by
longest-prefix match. Debouncing (coalescing the burst of Create/Modify
events one file write typically produces into a single logical event) is
handled entirely by `notify-debouncer-full` — section 15's "do not assume
1 OS event = 1 new file" is the library's job, not ours.

Watching alone isn't enough (section 16): if XP FLOW was closed while
files landed in a folder, or an OS event was dropped, those files need to
be found some other way. `JobRunner::reconcile_source` handles this by
walking the folder (`walkdir`, `follow_links(false)` to avoid symlink
cycles, section 64) and diffing the walk against what's already indexed
for that source (`VideoRepository::list_paths_for_source`):

- A discovered path with no matching row → ingested (through the same
  `ingest_path` as everything else).
- A discovered path that matches an existing row → `last_seen_at`
  refreshed; if it had drifted to `Missing`, it flips back to `Available`
  (a volume coming back online, section 62).
- A row for this source whose path is **not** in this scan **and** no
  longer exists on disk → `availability_status = Missing`. The row is
  never deleted.

This runs once per source at app startup (`JobRunner::startup_reconciliation`)
and on a 5-minute timer thereafter (`spawn_periodic_reconciliation`,
section 17 — a backstop, not the primary detection mechanism) as well as
on demand ("Scan now" in Settings → Storage → Content Sources, or right
after a new folder source is created).

## 3. Idempotency and crash recovery

Every ingest triggered by the watcher goes through the `jobs` table
first: `JobRunner::handle_watch_event` builds a dedupe key from the file
path (`ingest:<path>`) and calls `JobRepository::enqueue`. A partial
unique index (`migrations/0002_media_library.sql`) rejects a second
`pending`/`running` job with the same key, so three duplicate filesystem
events for one file result in exactly one ingestion (section 58) — the
`enqueue` implementation detects the collision (the returned job's id
doesn't match the one just attempted) and the caller simply returns.

At startup, `JobRunner::recover_orphaned_jobs` resets any job still
`running` from a previous process to `failed` (section 84/85/86) — it
could never complete anyway. Because a `Video` row is only written at the
very end of `ingest_path`, a process killed mid-ingestion simply leaves no
row behind; the next reconciliation pass picks the file up again from
scratch. Nothing needs bespoke "resume a partial import" logic.

Concurrency is bounded by a single `tokio::sync::Semaphore` (3 permits)
shared across watcher dispatch, manual import and reconciliation
(section 56/87) — XP FLOW never runs more than a handful of FFprobe/
FFmpeg processes at once regardless of how many files show up together.

## 4. Duplicate detection

**Exact duplicates** (`content_hash` — SHA-256, streamed in 1MiB chunks,
section 26/27) are never persisted as a second `Video` row. If the
existing row's file still exists, `ingest_path` returns `Duplicate` and
nothing is written; if it doesn't, that's treated as the same file having
moved and the existing row is updated in place (section 32) rather than
creating an orphaned duplicate.

**Near-duplicates** use a lightweight, explicitly-scoped technique
(section 28): a 64-bit difference-hash (dHash) computed from the video's
_thumbnail_ — resize to 9×8 grayscale, compare each pixel to its right
neighbor, pack the results into a hex string
(`infrastructure::hashing::DHashPerceptualHashService`). This is one
frame's fingerprint, not real video perceptual hashing, and every match is
stored in `duplicate_matches` with a similarity score
(1 − Hamming distance⁄64) rather than treated as certain — the Content
Details panel shows "possible duplicate," never "duplicate." Comparison is
bounded to the workspace's most recent 500 videos so it can never become
an O(library size) scan on every import.

## 5. Local preview

The frontend never receives a filesystem path for playback (section 89).
`commands::media_protocol` registers a custom `xpflowmedia://` URI scheme;
`<video src="https://xpflowmedia.localhost/video/<id>">` resolves the id to
a path via the database on the Rust side and streams bytes back, honoring
HTTP `Range` requests (bounded, suffix, and open-ended forms) so seeking
never requires loading a whole 4K file into memory (section 88).
Thumbnails are served the same way at `.../thumbnail/<id>`.

## 6. Cache layout

```text
XP FLOW/
├── xpflow.db
├── cache/
│   ├── thumbnails/   one <video-id>.jpg per video, regenerable on demand
│   └── temp/         scratch space — safe to clear at any time
└── logs/
```

Nothing here ever lives beside the user's original video files
(section 65/66). Settings → Storage reports the size of each
(`get_cache_info`) and can clear `cache/temp/` (`clear_temp_cache`) — the
thumbnail cache is never touched by that action; regenerating a thumbnail
happens per-video, on request.

## 7. What's still a placeholder

- **Bundled FFmpeg/FFprobe binaries** (section 20): the resolution order
  (env override → sidecar next to the executable → system `PATH`,
  `infrastructure::media::resolver`) is implemented and correct, but no
  binaries are actually bundled in this repository — a packaged build
  needs to place `ffmpeg`/`ffprobe` next to the executable (or a `bin/`
  subfolder) for the sidecar path to resolve.
- **Move detection across volumes**: only same-content-hash-at-a-new-path
  within a normal scan is handled — cross-volume tracking is explicitly
  out of scope (section 32).
- **Add-to-queue** on a video card is visibly disabled with a "coming in
  Phase 3" tooltip rather than faked (section 35/97) — the real queue and
  scheduler land in a later phase.
