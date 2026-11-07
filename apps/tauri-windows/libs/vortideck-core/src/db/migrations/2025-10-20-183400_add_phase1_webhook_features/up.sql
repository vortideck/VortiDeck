-- Phase 1 Webhook Features: Test/Production URLs, Response Modes, Authentication
-- Based on n8n architecture analysis

-- Add test_url and production_url fields
ALTER TABLE webhooks ADD COLUMN test_url TEXT;
ALTER TABLE webhooks ADD COLUMN production_url TEXT;

-- Add test activation tracking
ALTER TABLE webhooks ADD COLUMN is_test_active BOOL NOT NULL DEFAULT 0;
ALTER TABLE webhooks ADD COLUMN test_expires_at TEXT;

-- Add response mode (onReceived, lastNode, responseNode, streaming)
ALTER TABLE webhooks ADD COLUMN response_mode TEXT NOT NULL DEFAULT 'onReceived';

-- Add authentication configuration (JSON: type, config)
ALTER TABLE webhooks ADD COLUMN auth_config TEXT;

-- Add HTTP methods support (JSON array: ["GET", "POST"])
ALTER TABLE webhooks ADD COLUMN allowed_methods TEXT NOT NULL DEFAULT '["POST"]';

-- Add indexes for performance
CREATE INDEX idx_webhooks_test_active ON webhooks(is_test_active);
CREATE INDEX idx_webhooks_test_expires ON webhooks(test_expires_at);
