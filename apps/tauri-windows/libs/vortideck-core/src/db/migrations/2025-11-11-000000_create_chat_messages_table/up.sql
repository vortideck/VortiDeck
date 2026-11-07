-- Create chat_messages table to store streaming chat messages
-- Strategy: Keep messages for the last 2 session runs only
CREATE TABLE `chat_messages` (
    `id` TEXT NOT NULL PRIMARY KEY,
    `account_id` INTEGER NOT NULL,
    `session_id` TEXT,  -- Which session template (can be NULL if session deleted)
    `session_run_index` INTEGER,  -- Which run (0, 1, 2...) within that session
    `platform` TEXT NOT NULL,  -- 'twitch', 'youtube', 'kick', etc.
    `user_id` TEXT NOT NULL,
    `user_name` TEXT NOT NULL,
    `user_display_name` TEXT NOT NULL,
    `message` TEXT NOT NULL,
    `timestamp` TEXT NOT NULL,  -- ISO 8601 timestamp
    `badges` TEXT,  -- JSON array of badge strings
    `is_moderator` INTEGER NOT NULL DEFAULT 0,  -- SQLite boolean (0/1)
    `is_subscriber` INTEGER NOT NULL DEFAULT 0,
    `is_vip` INTEGER NOT NULL DEFAULT 0,
    `platform_color` TEXT,
    `emotes` TEXT,  -- JSON array of emote objects
    `mentions` TEXT,  -- JSON array of mentioned usernames
    `created_at` TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Create indexes for efficient queries
CREATE INDEX `idx_chat_messages_account_timestamp` ON `chat_messages` (`account_id`, `timestamp`);
CREATE INDEX `idx_chat_messages_session_run` ON `chat_messages` (`session_id`, `session_run_index`);
CREATE INDEX `idx_chat_messages_timestamp` ON `chat_messages` (`timestamp`);
CREATE INDEX `idx_chat_messages_platform` ON `chat_messages` (`platform`, `account_id`);
