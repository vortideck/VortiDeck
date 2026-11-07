-- Create the unified saved_actions table
CREATE TABLE `saved_actions`(
	`id` INTEGER NOT NULL PRIMARY KEY,
	`integration_id` TEXT NOT NULL,
	`name` TEXT NOT NULL,
	`action` TEXT NOT NULL,
	`params` TEXT NOT NULL
);

-- Copy all existing obs_actions to the new table with integration_id = 'obs'
INSERT INTO `saved_actions` (`id`, `integration_id`, `name`, `action`, `params`)
SELECT `id`, 'obs', `name`, `action`, `params` FROM `obs_actions`;

-- Drop the old obs_actions table
DROP TABLE `obs_actions`;
