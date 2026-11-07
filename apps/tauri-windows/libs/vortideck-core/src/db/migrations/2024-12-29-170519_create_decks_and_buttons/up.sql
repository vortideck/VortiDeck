-- Your SQL goes here


CREATE TABLE `buttons`(
	`id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	`deck_id` INTEGER NOT NULL,
	`label` TEXT NOT NULL,
	`icon` TEXT,
	`action` TEXT NOT NULL,
	`background` TEXT,
	`position` INTEGER NOT NULL
);

CREATE TABLE `decks`(
	`id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
	`name` TEXT NOT NULL,
	`grid` TEXT NOT NULL,
	`background` TEXT
);

