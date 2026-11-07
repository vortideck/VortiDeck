-- Add desktop_id column to app_config table for persistent relay desktop identification
-- This ensures desktop_id survives restarts and maintains E2EE relay connections
ALTER TABLE app_config ADD COLUMN desktop_id TEXT;
