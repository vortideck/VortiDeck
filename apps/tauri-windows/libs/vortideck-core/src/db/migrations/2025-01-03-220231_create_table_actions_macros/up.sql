-- Your SQL goes here


ALTER TABLE `buttons` DROP COLUMN `action`;
ALTER TABLE `buttons` ADD COLUMN `action_id` INTEGER;



CREATE TABLE `actions`(
	`id` INTEGER NOT NULL PRIMARY KEY,
	`action_type` TEXT NOT NULL,
	`macro_id` INTEGER,
	`file_path` TEXT,
	`custom_data` TEXT
);

CREATE TABLE `macros`(
	`id` INTEGER NOT NULL PRIMARY KEY,
	`macro_chain` TEXT NOT NULL
);

