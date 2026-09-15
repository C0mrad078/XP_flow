# Queue, Scheduler and Calendar (Phase 3)

This document covers the persistent queue engine, scheduler, calendar and
priority system added in Phase 3. See `docs/architecture.md` for how this
fits into the rest of the backend; this file goes one level deeper on the
parts that are new. See `docs/media-library.md` for Phase 2's ingestion
pipeline, which this phase builds on top of but does not change.

**What this phase does not do** (section 111 of the brief, unchanged):
real OAuth/uploading for YouTube/TikTok/Kwai, actually publishing a video
anywhere, comments sync, social analytics, or AI. A publication can be
scheduled all the way to "this exact UTC instant is booked" — nothing
ever consumes that booking to call a real platform API.

## 1. Domain model and the decisions behind it

The brief was explicit that the `Publication`/`QueueItem`/`ScheduleSlot`
relationship should avoid redundant fields and be documented. The
relationship settled on:

- **`Publication`** (`domain::publication`) is the source of truth for
  _what_ is being published, _where_, and _its current lifecycle state_.
  Phase 3 adds `workspace_id` (so queries can be workspace-scoped without a
  join), `priority` (seeded from the source video's priority at creation
  time but independently mutable afterward — bumping a publication's
  priority must never silently change the video's own priority, and vice
  versa), and `locked` (protects a publication's `scheduled_at` from the
  auto-scheduler and "Rebuild Schedule").
- **`QueueItem`** (`domain::queue_item`) only owns what is genuinely
  specific to _manual queue membership_: its existence and its manual
  `position`. It does **not** duplicate `Publication::priority` — the
  Phase 1 shape had a redundant `priority` column that was never a second
  source of truth, just a footgun waiting to drift from the real one, so
  it was removed. A `QueueItem` row exists only while a publication is
  under active manual-queue management: created on `Ready -> Queued`, kept
  through `Scheduled` (a scheduled item is still "in the queue"), and
  removed on `Cancelled`/`Archived`. `position` is only meaningful for
  _unscheduled_ items — once a publication has a `scheduled_at`, its order
  is derived from that timestamp instead.
- **`ScheduleSlot`** (`domain::schedule_slot`) models a channel's recurring
  weekly publishing pattern. `platform` is `Option<Platform>` rather than
  a required column plus a separate "applies to all platforms" flag or a
  second table: `None` is a channel-default slot that applies to any
  platform without its own slots on that weekday; `Some(p)` is
  platform-specific and takes priority over the default for that weekday.
- **`ScheduleException`** (`domain::schedule_exception`, new) models a
  one-off override to the recurring pattern for a specific date. Only
  `Skip` is implemented — a full "different slot set just for this one
  date" system was deliberately deferred as a materially bigger feature
  with no near-term operator need; the brief explicitly allows a "basic,
  practical" exception system rather than the maximal one.
- **`Channel`** gains `status` (`Active`/`Paused`, section 23) — a paused
  channel keeps all of its data untouched and is simply skipped by every
  scheduling operation until resumed.
- **`Workspace`** gains `timezone` (an IANA identifier, section 19,
  validated against the compiled `chrono-tz` database on write).
- **`PlatformAccount`** gains `default_target: bool` (section 49) rather
  than a new `channel_platform_targets` table — the table already existed
  for exactly this purpose and reusing it avoids a second schema concept
  for the same idea.

## 2. Timezones (section 19-20)

Every `DateTime<Utc>` is what gets persisted and compared; every
wall-clock schedule slot and every calendar display resolves through the
_workspace's_ configured IANA timezone, never the host OS's. This is not
optional decoration — the brief is explicit that relying on the machine's
local timezone is a bug, since the operator's machine and the content
schedule's intended timezone are not guaranteed to match.

`domain::scheduling::find_next_available_slot` is the pure function that
does this conversion (`chrono_tz::Tz::from_local_datetime`), and it is
DST-safe by construction:

- A local time that falls in a **"spring forward" gap** (a wall-clock time
  that never occurs, e.g. 02:30 on the day a zone jumps from 02:00 to
  03:00) has no valid UTC mapping — that slot is skipped for that date
  rather than silently shifted to some other time.
- A local time that falls in a **"fall back" ambiguous window** (a
  wall-clock time that occurs twice) deterministically resolves to the
  _earlier_ of the two instants, rather than being random or panicking.

Both cases are unit tested against `America/New_York`'s real 2024 DST
transitions (`domain::scheduling::tests::is_dst_safe_across_the_*`).
`SchedulerService::reschedule_to_date` (the calendar drag-and-drop path)
and `SchedulerService::calendar_range` share the same underlying
`resolve_local_datetime_to_utc` helper — there is exactly one place in the
codebase that converts a local wall-clock instant to UTC.

The frontend never does its own timezone math for _when_ something is
scheduled: `get_calendar_range` returns each publication's UTC
`scheduled_at` pre-resolved into `local_date`/`local_time` strings, and
`formatTimeInZone`/`formatDateInZone`/`localDateKeyInZone`
(`src/lib/formatting/date.ts`) format any other raw `scheduled_at` using
the workspace's `timezone`, explicitly _not_ the browser's default. The
Calendar's own day-grid layout (`src/features/calendar/calendar-dates.ts`)
is deliberately timezone-_unaware_ — grid layout (which dates go in which
cell) never depends on a zone, only on which calendar date a publication's
already-resolved `local_date` falls on.

## 3. The scheduling algorithm

`domain::scheduling::find_next_available_slot` is pure: given a channel's
active slots, its exceptions, the workspace timezone, a platform, an
earliest-allowed instant, a set of already-`taken` instants, and a search
horizon (in days), it returns the earliest UTC instant that satisfies
every constraint, or `None` if nothing was found within the horizon. It
takes no repository/clock dependency, which is what makes it exhaustively
unit-testable (weekday matching, channel-default-vs-platform-specific slot
resolution, skip exceptions, taken-instant avoidance, both DST edge cases,
and horizon exhaustion) without a database or the real system clock.

`SchedulerService` is the thin orchestration layer around it:

- **`auto_schedule`** — one `Queued`, unlocked publication into its
  channel's next available slot.
- **`auto_schedule_channel`** — every unlocked `Queued` publication on a
  channel, priority-first (`Urgent > High > Normal > Low`, then oldest
  first as a stable tiebreak), inside **one database transaction**
  (`PublicationRepository::bulk_update`) — all-or-nothing. An item for
  which no slot could be found within the horizon is left `Queued` and
  reported as _skipped_, not treated as a batch failure.
- **`fill_schedule_gaps`** — the same algorithm as `auto_schedule_channel`
  bounded to a shorter 30-day horizon (vs. the default 120), since its
  purpose (section 26) is closing visible _near-term_ gaps.
- **`rebuild_channel_schedule`** — recomputes `scheduled_at` for every
  _unlocked, already-Scheduled_ publication on a channel (e.g. after its
  weekly slots changed). Locked scheduled publications are treated as
  immovable obstacles everyone else must route around; a publication that
  no longer fits anywhere is unscheduled back to `Queued` rather than left
  with a stale time.

## 4. Concurrency and data integrity (sections 27/28/50/51/113)

Every invariant that matters is enforced at **two** layers: an
application-level check first (for a fast, friendly error), backstopped by
a **database-level partial unique index** that makes the invariant true
regardless of race conditions, crash timing, or a bug in the application
check:

- `idx_publications_active_dedupe` — at most one non-terminal publication
  per `(video_id, channel_id, platform)`. `cancelled`/`archived`/
  `duplicate` are excluded so a video can be re-queued to the same target
  after cancellation; `published` is _included_ (still blocks) so the same
  video can't be silently re-added while still sitting in `published`
  awaiting archival.
- `idx_publications_channel_scheduled_dedupe` — no two `Scheduled`
  publications may share the exact same `(channel_id, scheduled_at)`.
  This is what actually prevents a double-booked slot under a real
  concurrent race — `application::scheduler_service::tests::
concurrent_scheduling_to_the_same_instant_never_double_books` proves
  this directly: it fires two real concurrent `schedule_at` calls (via
  `tokio::join!`, not sequential calls) at the same channel and instant,
  and asserts exactly one wins while the database never ends up with two
  matching rows — i.e. the _database_ is what prevents the double-booking,
  not just the sequential application-level check (which has an inherent
  TOCTOU window under real concurrency).
- `idx_schedule_slots_dedupe` — no two active slots at the same
  `(channel_id, platform, day_of_week, time_of_day)`.

Bulk scheduling operations (auto-schedule-channel, rebuild, fill-gaps) use
`PublicationRepository::bulk_update`, which opens one `sqlx::Transaction`
and applies every row's `UPDATE` against it, rolling back the entire batch
on the first failure (a conflict, or any other database error) and
returning `DomainError::BulkScheduleFailed`. This means a mid-batch
conflict can never leave half a bulk operation committed and half not —
tested in `auto_schedule_channel_is_transactional_and_priority_ordered`.

## 5. Overdue is derived, never a new state (sections 78/81/82)

There is no `Overdue` variant in `PublicationStatus`, and nothing ever
mutates a publication to `Failed` just because Phase 3 has no real
uploader to actually attempt the publish. "Overdue" is computed on demand
from existing fields: `Publication::is_overdue(now)` (Rust) and
`isPublicationOverdue(publication, now)` (TypeScript,
`src/types/domain.ts`) both return `status == Scheduled && scheduled_at <
now`. `SchedulerService::due_publications` /
`PublicationRepository::list_due` is the query that would back Phase 5's
real uploader — Phase 3 only uses it to power the Today page's "Overdue"
warning list and the `list_due_publications` command; nothing in this
phase acts on it.

## 6. Queue lifecycle

`PublicationService` (`application/publication_service.rs`) owns the
`Publication` lifecycle and `QueueItem` membership:

- **`add_to_queue`** — validates the target video/channel/platform combo
  isn't already active (backstopped by the DB index above), creates the
  `Publication` (seeding its priority from the video unless overridden),
  walks it through `Imported -> Validating -> Ready -> Queued` via the
  existing state machine (no per-publication validation step exists in
  Phase 3 — the video itself was already validated on import; these calls
  are simply reaching the machine's "ready to queue" point, not skipping
  real work), and creates its `QueueItem` at the back of the manual queue.
- **`add_to_queue_bulk`** — the same, per video id, reporting a _per-item_
  outcome rather than aborting the whole batch on the first duplicate.
  This is deliberately different from the scheduler's bulk operations:
  duplicate-video skips during a bulk add are expected and benign, while a
  partial bulk _schedule_ would leave the calendar in a confusing
  half-filled state, which is why that one is transactional.
- **`cancel`** / **`archive`** — `Publication::transition` to
  `Cancelled`/`Archived` (never delete), and remove the `QueueItem` row
  since it no longer represents active queue management.
- **`reorder_queue`** — reassigns manual `position` values in the order
  supplied (drag-reorder for unscheduled items). `position` carries no
  uniqueness constraint — it's an ordering hint, not an identity — so
  sequential per-item updates are sufficient; a transient duplicate
  position can't corrupt anything worse than a temporary mis-sort.
- **`reconcile_queue`** — startup/periodic consistency sweep (section
  91/113): finds `QueueItem` rows whose publication is missing or already
  terminal (which should never happen through normal application code
  paths, but could follow a crash between two writes) and removes them,
  logging a warning every time — nothing is ever silently discarded. Runs
  once at startup (`lib.rs::bootstrap`) after the existing media
  reconciliation.

## 7. Job types

`jobs::JobType` gained four variants (`AutoSchedule`, `RebuildSchedule`,
`FillScheduleGaps`, `QueueReconciliation`) and the `jobs` table's schema
was extended to accept them, extending the existing `JobRunner`
vocabulary rather than introducing a second job engine (section 5).
`QueueReconciliation` corresponds to `PublicationService::reconcile_queue`
and is exercised at startup. The other three job types are reserved
vocabulary for a future _background/async_ trigger (e.g. a nightly
"auto-schedule everything" cron) — in Phase 3, the equivalent
user-triggered actions (the Queue's "Auto-schedule", the Channel editor's
implicit "Rebuild Schedule" after an edit, "Fill Empty Slots") call
`SchedulerService` directly as synchronous request/response commands,
since they're bounded, fast, single-channel operations the UI needs
immediate feedback from — not the kind of long-running background work
`JobRunner`'s semaphore-bounded queue exists for. This is a deliberate
scope decision: the job-type vocabulary is ready for the day an operator
wants a scheduled/background trigger, without forcing every interactive
button click through the job queue's async lifecycle today.

## 8. Calendar

`SchedulerService::calendar_range` takes a workspace, an optional channel
filter, and a `[start, end]` `NaiveDate` range, resolves `start`'s local
midnight and `end + 1 day`'s local midnight to UTC (via the same
DST-safe helper as the rest of the scheduler), queries every `Scheduled`
publication in that UTC window, and returns each with its `scheduled_at`
pre-resolved into `local_date`/`local_time`.

The frontend (`src/features/calendar/`) computes a Monday-start
month-grid (42 cells, including leading/trailing days from adjacent
months) or a 7-day week grid using plain, timezone-unaware date math
(`calendar-dates.ts`), fetches that range, and buckets the results by
`local_date`. Publications render as `dnd-kit` draggable chips; dropping
one on a different day cell calls `reschedule_publication_to_date`, which
preserves the publication's local time-of-day and moves only its date —
disabled outright (both the chip's `useDraggable` and the backend
service) for locked publications. The drag is optimistic — the chip jumps
immediately via a TanStack Query cache patch — and rolls back
automatically if the backend rejects the move (a paused channel, a slot
conflict, or a lock), always reconciling with the server afterward in
case the exact resolved time shifted (e.g. a DST edge).

## 9. Channel weekly schedule editor

`ScheduleSlotService` (`application/schedule_slot_service.rs`) is pure CRUD
orchestration around `ScheduleSlot`/`ScheduleException` — the algorithm
that actually _uses_ this data lives in `domain::scheduling`. The Channels
screen's schedule drawer (`src/features/channels/channel-schedule-drawer.tsx`)
lets an operator add/remove per-weekday slots (channel-default or
platform-specific), toggle a slot active/paused without deleting it, copy
one day's slots to every other day, and manage skip-date exceptions. The
drawer shows a simple "active slot count" as a next-7-days capacity
estimate — correct because any 7 consecutive days cover each weekday
exactly once, so the total _active_ slot count equals the slot count in
any 7-day window (skip exceptions inside that window are not subtracted,
a documented simplification rather than a bug).

## 10. Startup recovery

`lib.rs::bootstrap` runs `PublicationService::reconcile_queue` once at
startup (after the existing Phase 2 media reconciliation), logging a
warning activity event and a `tracing::warn!` if it found and removed any
orphaned `QueueItem` rows. "Overdue" detection needs no recovery step of
its own since it is derived on read (§5) — there is no stale persisted
state to reconcile.

## 11. What's still a placeholder or deliberately deferred

- **Real publishing**: nothing in this phase calls a platform API. The
  due-publication query (`list_due_publications`) exists specifically so a
  future phase's real uploader has a clean, already-tested seam to
  consume, but Phase 3 only uses it for the "Overdue" UI signal.
- **Custom per-date schedules** (§1): only "skip this date" is
  implemented; a full different-slot-set-for-one-date system was
  deliberately deferred.
- **Background/async triggering** for auto-schedule/rebuild/fill-gaps
  (§7): the `JobType` vocabulary exists; nothing enqueues it yet.
- **Command palette / context menu Phase 3 actions**: Queue/Calendar
  navigation is already reachable via the command palette's existing
  generic "Go to <screen>" entries (every `nav-items.ts` route gets one
  automatically); no _channel-scoped_ action (auto-schedule, rebuild) was
  added to the palette since none of them have a sensible contextless
  invocation from a global command surface — a "Publish Now" queue-item
  context-menu entry was likewise not added since Phase 3 has no
  publishing action for it to trigger (it would be a disabled placeholder
  for a feature two phases away, not a queue-affecting control).
