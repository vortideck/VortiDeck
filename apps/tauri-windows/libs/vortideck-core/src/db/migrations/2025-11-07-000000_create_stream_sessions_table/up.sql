-- Create stream_sessions table to store reusable streaming session configurations
CREATE TABLE `stream_sessions` (
    `id` TEXT NOT NULL PRIMARY KEY,
    `name` TEXT NOT NULL,
    `description` TEXT,
    `platforms` TEXT NOT NULL, -- JSON array of PlatformStream objects
    `commands` TEXT NOT NULL, -- JSON array of SessionCommand objects
    `settings` TEXT NOT NULL, -- JSON object of SessionSettings
    `status` TEXT NOT NULL, -- 'draft', 'waiting_for_live', 'active', 'paused', 'inactive'
    `created_at` TEXT NOT NULL DEFAULT (datetime('now')),
    `updated_at` TEXT NOT NULL DEFAULT (datetime('now')),
    `current_run_started_at` TEXT,
    `paused_at` TEXT,
    `duration` INTEGER, -- Duration in milliseconds for current run
    `message_count` INTEGER DEFAULT 0, -- Messages in current run
    `command_executions` INTEGER DEFAULT 0, -- Commands executed in current run
    `runs` TEXT, -- JSON array of StreamRun objects (history)
    `total_runs` INTEGER DEFAULT 0,
    `last_used_at` TEXT,
    `total_duration` INTEGER DEFAULT 0, -- Sum of all run durations in milliseconds
    `total_messages` INTEGER DEFAULT 0, -- Sum of all messages across all runs
    `total_commands` INTEGER DEFAULT 0 -- Sum of all commands across all runs
);

-- Create indexes for better performance
CREATE INDEX `idx_stream_sessions_status` ON `stream_sessions` (`status`);
CREATE INDEX `idx_stream_sessions_created_at` ON `stream_sessions` (`created_at`);
CREATE INDEX `idx_stream_sessions_last_used_at` ON `stream_sessions` (`last_used_at`);
