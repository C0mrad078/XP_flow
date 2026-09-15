-- Closes a race window in MediaIngestionService::ingest_path (section 99
-- quality review): two files with identical content ingested concurrently
-- (the 3-permit semaphore allows up to 3 ingestions at once) could both
-- pass the `get_by_hash` duplicate check before either had committed its
-- `INSERT`, producing two rows with the same content_hash. A partial
-- unique index makes the second INSERT fail at the database level instead
-- — the application layer already treats a create() failure as
-- IngestOutcome::Rejected, so the concurrent ingestion simply reports
-- "try again" rather than silently duplicating the video.
CREATE UNIQUE INDEX idx_videos_workspace_content_hash ON videos(workspace_id, content_hash)
    WHERE content_hash IS NOT NULL;
