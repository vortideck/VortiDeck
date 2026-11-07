-- This file should undo anything in `up.sql`
-- Remove subscription_id column from webhooks table
ALTER TABLE webhooks DROP COLUMN subscription_id;
