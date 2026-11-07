-- Remove E2EE columns from peers table
ALTER TABLE peers DROP COLUMN mobile_public_key;
ALTER TABLE peers DROP COLUMN shared_secret;
