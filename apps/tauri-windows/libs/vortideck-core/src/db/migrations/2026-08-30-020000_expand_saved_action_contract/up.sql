ALTER TABLE saved_actions ADD COLUMN capability_version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE saved_actions ADD COLUMN connection_selector TEXT;
ALTER TABLE saved_actions ADD COLUMN presentation TEXT NOT NULL DEFAULT '{}';
