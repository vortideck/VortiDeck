-- Drop indexes first
DROP INDEX IF EXISTS `idx_webhook_events_webhook_id`;
DROP INDEX IF EXISTS `idx_webhook_events_created_at`;
DROP INDEX IF EXISTS `idx_webhook_events_status`;
DROP INDEX IF EXISTS `idx_webhooks_service`;
DROP INDEX IF EXISTS `idx_webhooks_enabled`;

-- Drop tables (events first due to foreign key)
DROP TABLE IF EXISTS `webhook_events`;
DROP TABLE IF EXISTS `webhooks`;
