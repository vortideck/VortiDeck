-- This file should undo anything in `up.sql`
-- Rollback Phase 1 webhook features

-- Drop indexes
DROP INDEX IF EXISTS idx_webhooks_test_expires;
DROP INDEX IF EXISTS idx_webhooks_test_active;

-- Drop columns
ALTER TABLE webhooks DROP COLUMN allowed_methods;
ALTER TABLE webhooks DROP COLUMN auth_config;
ALTER TABLE webhooks DROP COLUMN response_mode;
ALTER TABLE webhooks DROP COLUMN test_expires_at;
ALTER TABLE webhooks DROP COLUMN is_test_active;
ALTER TABLE webhooks DROP COLUMN production_url;
ALTER TABLE webhooks DROP COLUMN test_url;
