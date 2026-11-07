-- Create webhooks table
CREATE TABLE `webhooks`(
	`id` TEXT NOT NULL PRIMARY KEY,
	`name` TEXT NOT NULL,
	`description` TEXT,
	`service` TEXT NOT NULL,
	`endpoint_url` TEXT NOT NULL,
	`target_url` TEXT,
	`secret` TEXT,
	`action` TEXT,
	`enabled` BOOL NOT NULL DEFAULT 1,
	`created_at` TEXT NOT NULL,
	`updated_at` TEXT NOT NULL
);

-- Create webhook_events table
CREATE TABLE `webhook_events`(
	`id` TEXT NOT NULL PRIMARY KEY,
	`webhook_id` TEXT NOT NULL,
	`event_type` TEXT NOT NULL,
	`status` TEXT NOT NULL,
	`payload` TEXT,
	`response_code` INTEGER,
	`response_body` TEXT,
	`duration_ms` INTEGER,
	`error_message` TEXT,
	`created_at` TEXT NOT NULL,
	FOREIGN KEY (`webhook_id`) REFERENCES `webhooks`(`id`) ON DELETE CASCADE
);

-- Create indexes for better query performance
CREATE INDEX `idx_webhook_events_webhook_id` ON `webhook_events`(`webhook_id`);
CREATE INDEX `idx_webhook_events_created_at` ON `webhook_events`(`created_at`);
CREATE INDEX `idx_webhook_events_status` ON `webhook_events`(`status`);
CREATE INDEX `idx_webhooks_service` ON `webhooks`(`service`);
CREATE INDEX `idx_webhooks_enabled` ON `webhooks`(`enabled`);
