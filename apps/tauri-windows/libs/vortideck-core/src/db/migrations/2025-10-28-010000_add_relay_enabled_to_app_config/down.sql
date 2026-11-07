-- Remove relay_enabled column from app_config table
ALTER TABLE app_config DROP COLUMN relay_enabled;
