-- Your SQL goes here
ALTER TABLE `app_config` ADD COLUMN `api_url` TEXT NOT NULL;
ALTER TABLE `app_config` ADD COLUMN `log_level` TEXT NOT NULL;
ALTER TABLE `app_config` ADD COLUMN `language` TEXT NOT NULL;
ALTER TABLE `app_config` ADD COLUMN `timezone` TEXT NOT NULL;
ALTER TABLE `app_config` ADD COLUMN `theme` TEXT NOT NULL;


