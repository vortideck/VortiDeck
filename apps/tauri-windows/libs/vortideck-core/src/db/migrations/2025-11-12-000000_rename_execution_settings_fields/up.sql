-- Rename execution settings columns to be more generic
-- max_concurrent_buttons → max_concurrent_actions (covers all action types)
-- max_concurrent_webhooks, max_concurrent_macros → removed (unused)
-- max_button_timeout_ms → max_action_timeout_ms (covers all action types)

ALTER TABLE app_config RENAME COLUMN max_concurrent_buttons TO max_concurrent_actions;
ALTER TABLE app_config RENAME COLUMN max_button_timeout_ms TO max_action_timeout_ms;
ALTER TABLE app_config DROP COLUMN max_concurrent_webhooks;
ALTER TABLE app_config DROP COLUMN max_concurrent_macros;
