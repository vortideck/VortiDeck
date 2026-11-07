-- Add E2EE key columns to peers table
ALTER TABLE peers ADD COLUMN mobile_public_key TEXT;
ALTER TABLE peers ADD COLUMN shared_secret TEXT;

-- Clear all existing device tokens to force re-pairing with E2EE
UPDATE peers SET device_token = NULL WHERE device_token IS NOT NULL;
