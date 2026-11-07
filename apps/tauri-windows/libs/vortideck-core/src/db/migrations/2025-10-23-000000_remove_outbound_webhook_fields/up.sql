-- Remove outbound webhook fields that are not needed for inbound webhooks
-- These fields were for sending webhooks, which should be done via HTTP Request nodes instead

-- First drop the index on outbound_secret
DROP INDEX IF EXISTS idx_webhooks_outbound_secret;

-- SQLite doesn't support DROP COLUMN in older versions, so we use ALTER TABLE DROP COLUMN (SQLite 3.35.0+)
ALTER TABLE webhooks DROP COLUMN target_url;
ALTER TABLE webhooks DROP COLUMN outbound_secret;
ALTER TABLE webhooks DROP COLUMN auth_config;
