-- This file should undo anything in `up.sql`




ALTER TABLE `deck_settings` DROP COLUMN `active_theme`;

ALTER TABLE `decks` ADD COLUMN `background` TEXT;




