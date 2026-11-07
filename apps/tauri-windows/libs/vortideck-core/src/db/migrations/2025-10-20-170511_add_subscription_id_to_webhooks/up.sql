-- Add subscription_id column to webhooks table
-- This stores the external service's subscription/webhook ID (e.g., Twitch EventSub subscription ID)
ALTER TABLE webhooks ADD COLUMN subscription_id TEXT;
