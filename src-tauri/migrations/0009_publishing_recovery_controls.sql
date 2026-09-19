-- Phase 6: operator reconciliation and explicit repost lineage.
ALTER TABLE publications ADD COLUMN repost_of_publication_id TEXT REFERENCES publications(id);
ALTER TABLE publications ADD COLUMN reconciliation_token TEXT;
ALTER TABLE publications ADD COLUMN reconciliation_started_at TEXT;
ALTER TABLE publications ADD COLUMN reconciliation_result TEXT;
ALTER TABLE publications ADD COLUMN reconciled_at TEXT;

-- Normal queue dedupe remains in force. Reposts are explicit, linked rows;
-- one direct child per source is the database backstop against double-clicks.
DROP INDEX idx_publications_active_dedupe;
CREATE UNIQUE INDEX idx_publications_active_dedupe
    ON publications(video_id, channel_id, platform)
    WHERE status NOT IN ('cancelled', 'archived', 'duplicate')
      AND repost_of_publication_id IS NULL;
CREATE UNIQUE INDEX idx_publications_repost_source
    ON publications(repost_of_publication_id)
    WHERE repost_of_publication_id IS NOT NULL;
CREATE INDEX idx_publications_reconciliation
    ON publications(reconciliation_started_at)
    WHERE reconciliation_token IS NOT NULL;
