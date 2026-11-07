-- Remove execution settings columns from app_config table
ALTER TABLE app_config DROP COLUMN max_concurrent_buttons;
ALTER TABLE app_config DROP COLUMN max_concurrent_webhooks;
ALTER TABLE app_config DROP COLUMN max_concurrent_macros;
ALTER TABLE app_config DROP COLUMN max_button_timeout_ms;
