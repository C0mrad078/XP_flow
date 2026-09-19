-- Phase 7: indexes for bounded operational queue filters and scheduler views.
CREATE INDEX idx_publications_workspace_account
    ON publications(workspace_id, platform_account_id);
CREATE INDEX idx_publications_workspace_status_scheduled
    ON publications(workspace_id, status, scheduled_at);
