-- Create focus_windows table for storing window patterns to focus
CREATE TABLE focus_windows (
    id INTEGER PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    window_title TEXT NOT NULL,
    process_name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
