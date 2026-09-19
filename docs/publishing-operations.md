# Publishing Operations (Phase 7)

The Queue is backed by the paginated publication query. Filters for platform,
channel, account, title or remote ID, and “Requires attention” are applied in
SQLite rather than by loading the whole queue into the frontend. Attention
includes failed, blocked, authentication-required, rate-limited, and unknown
remote results.

Scheduled timestamps remain UTC in persistence and are rendered through the
workspace timezone utilities. Scheduler claims remain conditional on the
publication still being scheduled, due, unlocked, and unclaimed, so changing
or cancelling a queued item cannot silently race a worker claim.

Queue operations retain the Phase 6 safety rules: Retry continues an existing
execution, Reconcile is read-only remote inspection, and Repost is a confirmed
new execution. Bulk external operations such as Repost and ambiguous
Reconcile are intentionally not exposed.

The publishing concurrency setting is persisted and validated. The current
worker semaphore is sized during application startup; changing the value is
safe and persistent, but takes effect after restart so active uploads are never
interrupted. This behavior is surfaced in Settings → Publishing.

Dashboard counters use bounded backend queue data and distinguish scheduled,
active (uploading/processing), published-today, and attention items. Today
uses the workspace-local calendar range for daily status. Activity remains an
audit trail and does not receive per-chunk progress events.

Release gates RG-01 through RG-04 remain pending in
[release-validation.md](release-validation.md).
