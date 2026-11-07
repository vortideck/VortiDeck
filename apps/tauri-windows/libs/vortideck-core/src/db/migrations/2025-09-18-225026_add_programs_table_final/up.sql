-- Create programs table for saving reusable program actions
CREATE TABLE `programs`(
	`id` INTEGER NOT NULL PRIMARY KEY,
	`name` TEXT NOT NULL,
	`executable` TEXT NOT NULL,
	`args` TEXT,
	`working_dir` TEXT,
	`env_vars` TEXT,
	`timeout_ms` INTEGER,
	`show_terminal` BOOL NOT NULL
);
