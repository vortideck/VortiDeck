-- Add icon_updated_at field to buttons table
-- This field stores the Unix timestamp when the icon was last updated
-- Used for efficient icon change detection and caching
ALTER TABLE buttons ADD COLUMN icon_updated_at INTEGER;
