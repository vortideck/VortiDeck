-- Create http_requests table for storing reusable HTTP request templates
CREATE TABLE http_requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name TEXT NOT NULL,
    method TEXT NOT NULL,
    url TEXT NOT NULL,
    headers TEXT,
    body TEXT,
    timeout_ms INTEGER DEFAULT 30000,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
