-- This file should undo anything in `up.sql`


ALTER TABLE `buttons` DROP COLUMN `action`;
ALTER TABLE `buttons` ADD COLUMN `action` TEXT NOT NULL;



