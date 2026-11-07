-- Revert: remove consent flag and restore old opt-out default
UPDATE app_config SET analytics_enabled = 1;
-- SQLite doesn't support DROP COLUMN before 3.35.0, but Diesel handles this
