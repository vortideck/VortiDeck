-- Rollback: Merge inbound_secret and outbound_secret back into single secret field
DROP INDEX idx_webhooks_outbound_secret;
ALTER TABLE webhooks RENAME COLUMN inbound_secret TO secret;
ALTER TABLE webhooks DROP COLUMN outbound_secret
