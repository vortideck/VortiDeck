-- Remove execution_mode and execution_config columns from buttons table
ALTER TABLE buttons DROP COLUMN execution_mode;
ALTER TABLE buttons DROP COLUMN execution_config;
