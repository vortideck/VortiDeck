-- This file should undo anything in `up.sql`

-- Drop indexes first
DROP INDEX IF EXISTS `idx_flows_updated_at`;
DROP INDEX IF EXISTS `idx_flows_created_at`;
DROP INDEX IF EXISTS `idx_flows_name`;

-- Drop flows table
DROP TABLE IF EXISTS `flows`;
