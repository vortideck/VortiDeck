-- This file should undo anything in `up.sql`


ALTER TABLE `buttons` DROP COLUMN `action_id`;
ALTER TABLE `buttons` ADD COLUMN `action` TEXT;



DROP TABLE IF EXISTS `actions`;
DROP TABLE IF EXISTS `macros`;
