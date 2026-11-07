-- Revert the execution settings column renaming

ALTER TABLE app_config RENAME COLUMN max_concurrent_actions TO max_concurrent_buttons;
ALTER TABLE app_config RENAME COLUMN max_action_timeout_ms TO max_button_timeout_ms;
ALTER TABLE app_config ADD COLUMN max_concurrent_webhooks INTEGER NOT NULL DEFAULT 10;
ALTER TABLE app_config ADD COLUMN max_concurrent_macros INTEGER NOT NULL DEFAULT 10;
