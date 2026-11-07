-- Separate inbound and outbound webhook secrets for clarity
--
-- BEFORE: Single `secret` field used for two incompatible purposes:
--   1. External service secrets (Twitch HMAC key, GitHub secret) - for INBOUND webhooks
--   2. VortiDeck vdwh_ secrets - for OUTBOUND webhooks (signing webhooks we send)
--
-- AFTER: Two separate fields:
--   - `inbound_secret`: External service secret for verifying webhooks FROM Twitch/GitHub/etc
--   - `outbound_secret`: VortiDeck-generated vdwh_ secret for signing webhooks TO user URLs

-- Add new outbound_secret field
ALTER TABLE webhooks ADD COLUMN outbound_secret TEXT;

-- Rename existing secret to inbound_secret (for external service verification)
ALTER TABLE webhooks RENAME COLUMN secret TO inbound_secret;

-- Create index for faster lookups
CREATE INDEX idx_webhooks_outbound_secret ON webhooks(outbound_secret)
