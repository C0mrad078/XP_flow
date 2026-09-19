# Publishing Recovery Controls (Phase 6)

Phase 6 keeps the existing publishing engine, provider adapters, SQLite
repositories and scheduler, while making the three operator actions explicit.

## Retry

Retry continues the same logical execution. It reuses the publication's
`execution_key`, appends a new attempt, respects persisted retry and rate-limit
windows, and refuses to start when a session or provider response could have
created a remote object. Retry never creates a new publication.

## Reconcile

Reconcile is a read-only remote inspection used for
`UNKNOWN_REMOTE_RESULT`. A database lease allows one reconciliation worker at
a time and makes repeated calls idempotent. The service loads the persisted
session and account, calls the provider's reconciliation capability, and then
updates local state only from the result. It never initializes an upload,
sends media, or calls the recovery method that may resume a transfer.

YouTube queries a known video ID. If only a resumable session is known, it uses
the zero-byte resumable status probe; a partial response remains ambiguous and
an expired session is not treated as proof of absence. TikTok and Kwai inherit
the conservative provider-neutral behavior when their integrations lack a
read-only proof. An operator sees **Reconcile** instead of **Retry** for an
unknown result.

## Repost

Repost is an explicit, confirmed action that creates a new queued publication,
fresh `execution_key`, and new attempt/session history. The new row points to
`repost_of_publication_id`; the original row, remote ID and history are never
mutated. A partial unique index prevents two direct reposts from being created
by concurrent requests.

## Durable progress and recovery

Provider-acknowledged upload bytes are checkpointed monotonically in SQLite at
16 MiB or 30-second boundaries, with the final session update closing the
transfer. This reduces lost progress without writing once per progress event.
Startup recovery continues to verify remote state before resuming and keeps
ambiguous outcomes fail-closed.

Publication Details and Copy Diagnostics expose sanitized execution identity,
attempt count, remote ID, reconciliation result, retry state and errors. Upload
URLs, upload tokens and OAuth credentials are never returned to the UI.

The Phase 6 schema migration is `0009_publishing_recovery_controls.sql` and is
backward compatible with existing publications. Release gates RG-01 through
RG-04 remain tracked in [release-validation.md](release-validation.md); this
document does not claim live provider or OS validation.
