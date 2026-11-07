-- Create flows table to store flow definitions
CREATE TABLE `flows` (
    `id` TEXT NOT NULL PRIMARY KEY,
    `name` TEXT NOT NULL,
    `description` TEXT,
    `data` TEXT NOT NULL,
    `created_at` TEXT NOT NULL DEFAULT (datetime('now')),
    `updated_at` TEXT NOT NULL DEFAULT (datetime('now')),
    `created_by` TEXT,
    `tags` TEXT,
    `version` TEXT NOT NULL DEFAULT '1.0.0'
);

-- Create index for better performance on common queries
CREATE INDEX `idx_flows_name` ON `flows` (`name`);
CREATE INDEX `idx_flows_created_at` ON `flows` (`created_at`);
CREATE INDEX `idx_flows_updated_at` ON `flows` (`updated_at`);
