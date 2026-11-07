-- Add analytics_consent_given flag to track whether user has made an explicit choice.
-- Reset analytics_enabled to false so existing users see the consent banner.
ALTER TABLE app_config ADD COLUMN analytics_consent_given BOOLEAN NOT NULL DEFAULT 0;
UPDATE app_config SET analytics_enabled = 0;
