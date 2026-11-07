-- Restore outbound webhook fields (rollback migration)

ALTER TABLE webhooks ADD COLUMN target_url TEXT;
ALTER TABLE webhooks ADD COLUMN auth_config TEXT;
ALTER TABLE webhooks ADD COLUMN outbound_secret TEXT;

-- Recreate the index
CREATE INDEX idx_webhooks_outbound_secret ON webhooks(outbound_secret);
