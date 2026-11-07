-- This file should undo anything in `up.sql`
ALTER TABLE `app_config` DROP COLUMN `api_url`;
ALTER TABLE `app_config` DROP COLUMN `log_level`;
ALTER TABLE `app_config` DROP COLUMN `language`;
ALTER TABLE `app_config` DROP COLUMN `timezone`;
ALTER TABLE `app_config` DROP COLUMN `theme`;


