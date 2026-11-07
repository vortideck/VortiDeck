-- Add relay_enabled column to app_config table
ALTER TABLE app_config ADD COLUMN relay_enabled BOOLEAN NOT NULL DEFAULT 1;
