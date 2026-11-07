-- SQLite-compatible column removal by table rebuild
CREATE TABLE focus_windows_new (
    id INTEGER PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    window_title TEXT NOT NULL,
    process_name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO focus_windows_new (id, name, window_title, process_name, created_at)
SELECT id, name, window_title, process_name, created_at
FROM focus_windows;

DROP TABLE focus_windows;
ALTER TABLE focus_windows_new RENAME TO focus_windows;
