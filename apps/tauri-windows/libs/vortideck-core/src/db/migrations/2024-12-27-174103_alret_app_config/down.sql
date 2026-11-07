-- This file should undo anything in `up.sql`
ALTER TABLE `app_config` DROP COLUMN `api_url`;
ALTER TABLE `app_config` DROP COLUMN `log_level`;
ALTER TABLE `app_config` DROP COLUMN `language`;
ALTER TABLE `app_config` DROP COLUMN `timezone`;
ALTER TABLE `app_config` DROP COLUMN `theme`;
ALTER TABLE `app_config` ADD COLUMN `api_url` TEXT NOT NULL;
ALTER TABLE `app_config` ADD COLUMN `log_level` TEXT NOT NULL;
ALTER TABLE `app_config` ADD COLUMN `language` TEXT NOT NULL;
ALTER TABLE `app_config` ADD COLUMN `timezone` TEXT NOT NULL;
ALTER TABLE `app_config` ADD COLUMN `theme` TEXT NOT NULL;


