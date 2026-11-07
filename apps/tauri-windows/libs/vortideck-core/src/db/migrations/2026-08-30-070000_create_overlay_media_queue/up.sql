CREATE TABLE overlay_media_queue (
    id TEXT PRIMARY KEY NOT NULL,
    invocation_id TEXT NOT NULL,
    queue_kind TEXT NOT NULL,
    command TEXT NOT NULL,
    payload TEXT NOT NULL,
    status TEXT NOT NULL,
    receipt_phase TEXT,
    provider_receipt_id TEXT,
    error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    started_at TEXT,
    completed_at TEXT
);

CREATE INDEX idx_overlay_media_queue_pending
    ON overlay_media_queue(queue_kind, status, created_at);
