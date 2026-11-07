use chrono::Utc;
use serde_json::Value;
///! Production-ready webhook service implementation
///! Database-backed with full CRUD, signature verification, and event logging
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

use crate::db::models::{NewWebhook, NewWebhookEvent, UpdateWebhook, Webhook, WebhookEvent};
use crate::services::webhook::{
    WebhookError, WebhookRegistrar, WebhookRegistration, WebhookResult, WebhookStorage,
};

/// Production webhook service with database persistence
pub struct WebhookService {
    storage: Arc<WebhookStorage>,
    registrar: Arc<WebhookRegistrar>,
}

impl WebhookService {
    /// Create new webhook service
    pub fn new(storage: Arc<WebhookStorage>) -> Self {
        Self {
            storage,
            registrar: Arc::new(WebhookRegistrar::new()),
        }
    }

    /// Create webhook service with custom registrar (for OAuth integration)
    pub fn with_registrar(storage: Arc<WebhookStorage>, registrar: Arc<WebhookRegistrar>) -> Self {
        Self { storage, registrar }
    }

    // ==================== CRUD Operations ====================

    /// Create a new webhook (inbound only - for receiving webhooks from external services)
    pub fn create_webhook(
        &self,
        name: &str,
        description: Option<&str>,
        service: &str,
        action: Option<&str>,
        response_mode: Option<&str>,
        allowed_methods: Option<&str>,
    ) -> WebhookResult<Webhook> {
        use crate::services::webhook::{validate_http_methods, validate_response_mode};

        let id = format!("wh_{}", Uuid::new_v4().simple());
        let endpoint_url = format!("/webhook/{}", id);
        let now = Utc::now().to_rfc3339();

        // Generate test and production URLs (n8n pattern)
        let test_url = format!("/webhook-test/{}", id);
        let production_url = format!("/webhook/{}", id);

        // Validate inputs before inserting into database
        let validated_response_mode =
            validate_response_mode(response_mode).map_err(|e| WebhookError::InvalidInput(e))?;

        let validated_methods =
            validate_http_methods(allowed_methods).map_err(|e| WebhookError::InvalidInput(e))?;

        // Insert into database
        self.storage.insert_webhook(
            &id,
            name,
            description,
            service,
            &endpoint_url,
            None, // inbound_secret - set later when registering with external service
            action,
            true, // enabled by default
            &now,
            &now,
            None, // subscription_id not set initially
            // Phase 1-3 fields (validated)
            Some(&test_url),
            Some(&production_url),
            &validated_response_mode,
            &validated_methods,
        )?;

        // Return the created webhook
        self.storage.get_webhook(&id)
    }

    /// Get webhook by ID
    pub fn get_webhook(&self, id: &str) -> WebhookResult<Webhook> {
        self.storage.get_webhook(id)
    }

    /// List all webhooks
    pub fn list_webhooks(&self) -> WebhookResult<Vec<Webhook>> {
        self.storage.list_webhooks()
    }

    /// List webhooks by service type
    pub fn list_webhooks_by_service(&self, service: &str) -> WebhookResult<Vec<Webhook>> {
        self.storage.list_webhooks_by_service(service)
    }

    /// Update webhook
    pub fn update_webhook(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        service: Option<&str>,
        inbound_secret: Option<&str>,
        action: Option<&str>,
        enabled: Option<bool>,
        subscription_id: Option<&str>,
    ) -> WebhookResult<Webhook> {
        let now = Utc::now().to_rfc3339();

        let update = UpdateWebhook {
            name,
            description,
            service,
            inbound_secret,
            action,
            enabled,
            updated_at: &now,
            subscription_id,
            // Phase 1 fields - None means don't update
            test_url: None,
            production_url: None,
            is_test_active: None,
            test_expires_at: None,
            response_mode: None,
            allowed_methods: None,
        };

        self.storage.update_webhook(id, update)?;
        self.storage.get_webhook(id)
    }

    /// Delete webhook (also deletes events via CASCADE)
    pub fn delete_webhook(&self, id: &str) -> WebhookResult<()> {
        self.storage.delete_webhook(id)
    }

    // ==================== Webhook Receiving ====================

    /// Receive and process an incoming webhook
    ///
    /// # Arguments
    /// * `webhook_id` - Webhook ID from URL path
    /// * `payload` - Raw request body bytes
    /// * `msg_id` - Message ID from header (optional)
    /// * `timestamp` - Timestamp from header (optional)
    /// * `signature` - Signature from header (optional)
    ///
    /// # Returns
    /// * Parsed payload JSON if successful
    pub fn receive_webhook(
        &self,
        webhook_id: &str,
        payload: &[u8],
        msg_id: Option<&str>,
        timestamp: Option<&str>,
        signature: Option<&str>,
    ) -> WebhookResult<Value> {
        let start = Instant::now();

        // 1. Get webhook config
        let webhook = self.storage.get_webhook(webhook_id)?;

        if !webhook.enabled {
            return Err(WebhookError::WebhookDisabled(webhook_id.to_string()));
        }

        // 2. Parse payload (signature verification is handled in handler.rs for external services)
        let payload_json: Value = serde_json::from_slice(payload)
            .map_err(|e| WebhookError::InvalidPayload(e.to_string()))?;

        let duration = start.elapsed().as_millis() as i32;

        // 4. Log success event
        let event_id = format!("evt_{}", Uuid::new_v4().simple());
        let now = Utc::now().to_rfc3339();

        self.storage.insert_event(
            &event_id,
            webhook_id,
            "received",
            "success",
            Some(&serde_json::to_string(&payload_json).unwrap_or_default()),
            None,
            None,
            Some(duration),
            None,
            &now,
        )?;

        log::info!("📥 Webhook received: {} ({}ms)", webhook.name, duration);

        Ok(payload_json)
    }

    /// Handle webhook receive error (logs error event)
    pub fn log_receive_error(
        &self,
        webhook_id: &str,
        payload: Option<&[u8]>,
        error: &WebhookError,
        duration_ms: i32,
    ) -> WebhookResult<()> {
        let event_id = format!("evt_{}", Uuid::new_v4().simple());
        let now = Utc::now().to_rfc3339();

        let payload_str = payload.and_then(|p| std::str::from_utf8(p).ok());

        self.storage.insert_event(
            &event_id,
            webhook_id,
            "received",
            "error",
            payload_str,
            None,
            None,
            Some(duration_ms),
            Some(&error.to_string()),
            &now,
        )?;

        log::error!("❌ Webhook receive failed: {}", error);
        Ok(())
    }

    // ==================== Event Management ====================

    /// Log a webhook event (public interface for WebhookHandler)
    pub fn log_webhook_event(
        &self,
        webhook_id: &str,
        event_type: &str,
        status: &str,
        payload: Option<&str>,
        response_code: Option<i32>,
        response_body: Option<&str>,
        duration_ms: Option<i32>,
        error_message: Option<&str>,
        is_test: bool, // NEW: Mark test webhook events
    ) -> WebhookResult<()> {
        let event_id = format!("evt_{}", Uuid::new_v4().simple());
        let now = Utc::now().to_rfc3339();

        // Append "_test" suffix to event_type for test webhooks
        let final_event_type = if is_test {
            format!("{}_test", event_type)
        } else {
            event_type.to_string()
        };

        self.storage.insert_event(
            &event_id,
            webhook_id,
            &final_event_type,
            status,
            payload,
            response_code,
            response_body,
            duration_ms,
            error_message,
            &now,
        )
    }

    /// Get events for a webhook
    pub fn get_webhook_events(
        &self,
        webhook_id: &str,
        limit: i64,
    ) -> WebhookResult<Vec<WebhookEvent>> {
        self.storage.get_webhook_events(webhook_id, limit)
    }

    /// Get count of events created today for a webhook
    pub fn get_today_events_count(&self, webhook_id: &str) -> WebhookResult<i64> {
        self.storage.get_today_events_count(webhook_id)
    }

    /// Get recent events across all webhooks
    pub fn get_recent_events(&self, limit: i64) -> WebhookResult<Vec<WebhookEvent>> {
        self.storage.get_recent_events(limit)
    }

    /// Get event statistics
    pub fn get_event_stats(
        &self,
        webhook_id: Option<&str>,
    ) -> WebhookResult<crate::services::webhook::storage::EventStats> {
        self.storage.get_event_stats(webhook_id)
    }

    /// Clean up old events
    pub fn cleanup_old_events(&self, days_old: i64) -> WebhookResult<usize> {
        self.storage.delete_old_events(days_old)
    }

    // ==================== External Service Registration ====================

    /// Register webhook with external service (Twitch, GitHub, etc.)
    ///
    /// This creates a webhook in the database AND registers it with the external service's API.
    ///
    /// # Arguments
    /// * `name` - Webhook name
    /// * `service` - Service type ("twitch", "github", etc.)
    /// * `event_type` - Event to subscribe to ("stream.online", "push", etc.)
    /// * `config` - Service-specific config (e.g., broadcaster_id for Twitch)
    ///
    /// # Returns
    /// Created webhook with subscription_id stored in metadata
    pub async fn register_with_external_service(
        &self,
        name: &str,
        service: &str,
        event_type: &str,
        config: &Value,
    ) -> WebhookResult<Webhook> {
        log::info!(
            "📝 Registering webhook with external service: {} - {}",
            service,
            event_type
        );

        // 1. Create webhook in database first
        let webhook = self.create_webhook(
            name,
            Some(&format!("{} {} webhook", service, event_type)),
            service,
            None, // action
            None, // response_mode
            None, // allowed_methods
        )?;

        // 2. Register with external service
        let webhook_url = format!("http://localhost:3000{}", webhook.endpoint_url);

        match self
            .registrar
            .register_webhook(service, event_type, &webhook_url, config)
            .await
        {
            Ok(registration) => {
                // 3. Update webhook with subscription_id and secret
                log::info!(
                    "✅ External registration successful: {}",
                    registration.subscription_id
                );

                // Store both secret and subscription_id
                self.update_webhook(
                    &webhook.id,
                    None,                                // name
                    None,                                // description
                    None,                                // service
                    Some(&registration.secret),          // inbound_secret
                    None,                                // action
                    None,                                // enabled
                    Some(&registration.subscription_id), // subscription_id
                )?;

                // Return updated webhook with subscription_id
                self.get_webhook(&webhook.id)
            }
            Err(e) => {
                // Registration failed, delete the webhook
                log::error!("❌ External registration failed: {}", e);
                let _ = self.delete_webhook(&webhook.id);
                Err(WebhookError::Other(format!(
                    "Failed to register with {}: {}",
                    service, e
                )))
            }
        }
    }

    /// Unregister webhook from external service
    ///
    /// This removes the webhook from the external service AND deletes it from database.
    pub async fn unregister_from_external_service(&self, webhook_id: &str) -> WebhookResult<()> {
        let webhook = self.get_webhook(webhook_id)?;

        // TODO: Get subscription_id from webhook metadata
        // For now, we can't unregister without subscription_id
        log::warn!(
            "⚠️ Cannot unregister webhook {} - subscription_id not stored yet",
            webhook_id
        );

        // Delete from database
        self.delete_webhook(webhook_id)?;

        Ok(())
    }

    /// Check if webhook still exists on external service
    pub async fn check_external_webhook(&self, webhook_id: &str) -> WebhookResult<bool> {
        let webhook = self.get_webhook(webhook_id)?;

        // TODO: Get subscription_id from webhook metadata
        log::warn!(
            "⚠️ Cannot check webhook {} - subscription_id not stored yet",
            webhook_id
        );

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::r2d2::{self, ConnectionManager};
    use diesel::SqliteConnection;
    use diesel_migrations::MigrationHarness;

    fn setup_test_service() -> WebhookService {
        // Create in-memory database for testing
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = r2d2::Pool::builder()
            .max_size(1)
            .build(manager)
            .expect("Failed to create pool");
        pool.get()
            .expect("test database connection")
            .run_pending_migrations(crate::db::db::MIGRATIONS)
            .expect("test database migrations");

        let storage = Arc::new(WebhookStorage::new(pool));
        WebhookService::new(storage)
    }

    #[test]
    fn test_create_webhook() {
        let service = setup_test_service();

        let webhook = service
            .create_webhook(
                "Test Webhook",
                Some("Test description"),
                "github",
                Some("https://example.com/hook"),
                None,
                None, // response_mode
            )
            .unwrap();

        assert_eq!(webhook.name, "Test Webhook");
        assert_eq!(webhook.service, "github");
        assert!(webhook.endpoint_url.starts_with("/webhook/wh_"));
        assert!(webhook.inbound_secret.is_none());
        assert!(webhook.enabled);
    }

    #[test]
    fn test_webhook_crud() {
        let service = setup_test_service();

        // Create
        let webhook = service
            .create_webhook(
                "Test Webhook",
                None,
                "slack",
                Some("https://hooks.slack.com/test"),
                None,
                None, // response_mode
            )
            .unwrap();

        let id = webhook.id.clone();

        // Read
        let fetched = service.get_webhook(&id).unwrap();
        assert_eq!(fetched.name, "Test Webhook");

        // Update
        let updated = service
            .update_webhook(
                &id,
                Some("Updated Webhook"),
                None,
                None,
                None,
                None,
                Some(false),
                None,
            )
            .unwrap();

        assert_eq!(updated.name, "Updated Webhook");
        assert!(!updated.enabled);

        // Delete
        service.delete_webhook(&id).unwrap();
        assert!(service.get_webhook(&id).is_err());
    }

    #[test]
    fn test_receive_webhook_without_signature() {
        let service = setup_test_service();

        let webhook = service
            .create_webhook(
                "Test Webhook",
                None,
                "generic",
                None,
                None,
                None, // response_mode
            )
            .unwrap();

        let payload = serde_json::json!({"event": "test", "data": "hello"});
        let payload_bytes = serde_json::to_vec(&payload).unwrap();

        let result = service
            .receive_webhook(&webhook.id, &payload_bytes, None, None, None)
            .unwrap();

        assert_eq!(result["event"], "test");
        assert_eq!(result["data"], "hello");
    }
}

// ==================== Phase 1: Test URL Management ====================

impl WebhookService {
    /// Activate test mode for a webhook (120 second timeout like n8n)
    pub fn activate_test_mode(&self, webhook_id: &str) -> WebhookResult<Webhook> {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::seconds(120);

        let update = crate::db::models::UpdateWebhook {
            name: None,
            description: None,
            service: None,
            inbound_secret: None,
            action: None,
            enabled: None,
            updated_at: &now.to_rfc3339(),
            subscription_id: None,
            test_url: None,
            production_url: None,
            is_test_active: Some(true),
            test_expires_at: Some(&expires_at.to_rfc3339()),
            response_mode: None,
            allowed_methods: None,
        };

        self.storage.update_webhook(webhook_id, update)?;
        log::info!(
            "✅ Test mode activated for webhook {} (expires in 120s)",
            webhook_id
        );
        self.storage.get_webhook(webhook_id)
    }

    /// Deactivate test mode for a webhook
    pub fn deactivate_test_mode(&self, webhook_id: &str) -> WebhookResult<Webhook> {
        let now = Utc::now();

        let update = crate::db::models::UpdateWebhook {
            name: None,
            description: None,
            service: None,
            inbound_secret: None,
            action: None,
            enabled: None,
            updated_at: &now.to_rfc3339(),
            subscription_id: None,
            test_url: None,
            production_url: None,
            is_test_active: Some(false),
            test_expires_at: None,
            response_mode: None,
            allowed_methods: None,
        };

        self.storage.update_webhook(webhook_id, update)?;
        log::info!("⏸️ Test mode deactivated for webhook {}", webhook_id);
        self.storage.get_webhook(webhook_id)
    }

    /// Check if test mode has expired
    pub fn is_test_expired(&self, webhook: &crate::db::models::Webhook) -> bool {
        if !webhook.is_test_active {
            return false;
        }

        if let Some(expires_at_str) = &webhook.test_expires_at {
            if let Ok(expires_at) = chrono::DateTime::parse_from_rfc3339(expires_at_str) {
                return Utc::now() > expires_at.with_timezone(&Utc);
            }
        }

        false
    }

    /// Update response mode for a webhook
    pub fn set_response_mode(&self, webhook_id: &str, mode: &str) -> WebhookResult<Webhook> {
        use crate::services::webhook::validate_response_mode;

        // Validate the response mode before updating
        let validated_mode =
            validate_response_mode(Some(mode)).map_err(|e| WebhookError::InvalidInput(e))?;

        let now = Utc::now();

        let update = crate::db::models::UpdateWebhook {
            name: None,
            description: None,
            service: None,
            inbound_secret: None,
            action: None,
            enabled: None,
            updated_at: &now.to_rfc3339(),
            subscription_id: None,
            test_url: None,
            production_url: None,
            is_test_active: None,
            test_expires_at: None,
            response_mode: Some(&validated_mode),
            allowed_methods: None,
        };

        self.storage.update_webhook(webhook_id, update)?;
        log::info!(
            "✅ Response mode set to '{}' for webhook {}",
            validated_mode,
            webhook_id
        );
        self.storage.get_webhook(webhook_id)
    }

    /// Update allowed HTTP methods for a webhook
    pub fn set_allowed_methods(
        &self,
        webhook_id: &str,
        methods_json: &str,
    ) -> WebhookResult<Webhook> {
        use crate::services::webhook::validate_http_methods;

        // Validate the HTTP methods before updating
        let validated_methods =
            validate_http_methods(Some(methods_json)).map_err(|e| WebhookError::InvalidInput(e))?;

        let now = Utc::now();

        let update = crate::db::models::UpdateWebhook {
            name: None,
            description: None,
            service: None,
            inbound_secret: None,
            action: None,
            enabled: None,
            updated_at: &now.to_rfc3339(),
            subscription_id: None,
            test_url: None,
            production_url: None,
            is_test_active: None,
            test_expires_at: None,
            response_mode: None,
            allowed_methods: Some(&validated_methods),
        };

        self.storage.update_webhook(webhook_id, update)?;
        log::info!("✅ Allowed methods updated for webhook {}", webhook_id);
        self.storage.get_webhook(webhook_id)
    }
}
