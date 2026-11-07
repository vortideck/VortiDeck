-- This file should undo anything in `up.sql`

-- Drop indexes first
DROP INDEX IF EXISTS `idx_stream_sessions_last_used_at`;
DROP INDEX IF EXISTS `idx_stream_sessions_created_at`;
DROP INDEX IF EXISTS `idx_stream_sessions_status`;

-- Drop stream_sessions table
DROP TABLE IF EXISTS `stream_sessions`;
