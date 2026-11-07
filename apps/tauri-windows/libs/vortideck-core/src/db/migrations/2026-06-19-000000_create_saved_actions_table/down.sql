-- Recreate the old obs_actions table
CREATE TABLE `obs_actions`(
	`id` INTEGER NOT NULL PRIMARY KEY,
	`name` TEXT NOT NULL,
	`action` TEXT NOT NULL,
	`params` TEXT NOT NULL
);

-- Copy any obs actions back to obs_actions
INSERT INTO `obs_actions` (`id`, `name`, `action`, `params`)
SELECT `id`, `name`, `action`, `params` FROM `saved_actions` WHERE `integration_id` = 'obs';

-- Drop the unified saved_actions table
DROP TABLE `saved_actions`;
