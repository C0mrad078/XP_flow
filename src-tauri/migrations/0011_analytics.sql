CREATE TABLE publication_metric_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    publication_id TEXT NOT NULL REFERENCES publications(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    captured_at TEXT NOT NULL,
    views INTEGER,
    likes INTEGER,
    comments INTEGER,
    shares INTEGER,
    availability TEXT NOT NULL,
    error_code TEXT
);
CREATE INDEX idx_publication_metrics_range ON publication_metric_snapshots(publication_id, captured_at);

CREATE TABLE channel_metric_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    channel_id TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    captured_at TEXT NOT NULL,
    followers INTEGER,
    total_views INTEGER,
    availability TEXT NOT NULL,
    error_code TEXT
);
CREATE INDEX idx_channel_metrics_range ON channel_metric_snapshots(channel_id, captured_at);
