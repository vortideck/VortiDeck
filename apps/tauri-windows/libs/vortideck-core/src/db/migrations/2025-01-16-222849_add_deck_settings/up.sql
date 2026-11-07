-- Your SQL goes here







CREATE TABLE `deck_settings`(
	`id` INTEGER NOT NULL PRIMARY KEY,
	`button_spacing` FLOAT NOT NULL,
	`grid_margin` FLOAT NOT NULL,
	`group_buttons` BOOL NOT NULL,
	`maximize_button_space` BOOL NOT NULL,
	`grid` TEXT NOT NULL,
	`deck_id` INTEGER NOT NULL
);

