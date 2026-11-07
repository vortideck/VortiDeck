-- Your SQL goes here
-- Your SQL goes here
CREATE TABLE peers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    client_id TEXT NOT NULL,
    ip TEXT NOT NULL,
    device_type TEXT NOT NULL,
    device TEXT NOT NULL
)