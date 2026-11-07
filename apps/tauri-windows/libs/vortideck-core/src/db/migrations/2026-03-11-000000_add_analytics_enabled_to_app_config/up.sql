-- Add analytics_enabled column to app_config table (default false = opt-in consent model)
ALTER TABLE app_config ADD COLUMN analytics_enabled BOOLEAN NOT NULL DEFAULT 0;
