-- Add execution settings columns to app_config table
ALTER TABLE app_config ADD COLUMN max_concurrent_buttons INTEGER NOT NULL DEFAULT 10;
ALTER TABLE app_config ADD COLUMN max_concurrent_webhooks INTEGER NOT NULL DEFAULT 10;
ALTER TABLE app_config ADD COLUMN max_concurrent_macros INTEGER NOT NULL DEFAULT 10;
ALTER TABLE app_config ADD COLUMN max_button_timeout_ms INTEGER NOT NULL DEFAULT 300000;
