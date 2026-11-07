use crate::services::webhook::errors::{WebhookError, WebhookResult};
///! Database storage layer for webhooks
///! Handles all SQLite operations for webhooks and webhook events
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};

// Import from the main app's db module
// These will be defined in apps/tauri-windows/src-tauri/src/db/models.rs
// We'll use them via the app's interface

/// Database pool type
pub type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

/// Webhook storage operations
pub struct WebhookStorage {
    pool: DbPool,
}

impl WebhookStorage {
    /// Create new storage instance
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get database connection from pool
    fn get_conn(
        &self,
    ) -> WebhookResult<r2d2::PooledConnection<ConnectionManager<SqliteConnection>>> {
        self.pool
            .get()
            .map_err(|e| WebhookError::DatabaseError(format!("Failed to get connection: {}", e)))
    }

    /// Insert a new webhook (inbound only)
    pub fn insert_webhook(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        service: &str,
        endpoint_url: &str,
        inbound_secret: Option<&str>,
        action: Option<&str>,
        enabled: bool,
        created_at: &str,
        updated_at: &str,
        subscription_id: Option<&str>,
        // Phase 1 fields
        test_url: Option<&str>,
        production_url: Option<&str>,
        response_mode: &str,
        allowed_methods: &str,
    ) -> WebhookResult<()> {
        use crate::db::schema::webhooks;

        let mut conn = self.get_conn()?;

        let new_webhook = crate::db::models::NewWebhook {
            id,
            name,
            description,
            service,
            endpoint_url,
            inbound_secret,
            action,
            enabled,
            created_at,
            updated_at,
            subscription_id,
            // Phase 1 fields
            test_url,
            production_url,
            is_test_active: false, // Default to false
            test_expires_at: None, // No expiry initially
            response_mode,
            allowed_methods,
        };

        diesel::insert_into(webhooks::table)
            .values(&new_webhook)
            .execute(&mut conn)
            .map_err(|e| WebhookError::DatabaseError(format!("Failed to insert webhook: {}", e)))?;

        Ok(())
    }

    /// Get webhook by ID
    pub fn get_webhook(&self, webhook_id: &str) -> WebhookResult<crate::db::models::Webhook> {
        use crate::db::schema::webhooks::dsl::*;

        let mut conn = self.get_conn()?;

        webhooks
            .filter(id.eq(webhook_id))
            .first::<crate::db::models::Webhook>(&mut conn)
            .map_err(|e| match e {
                diesel::result::Error::NotFound => {
                    WebhookError::NotFound(format!("Webhook not found: {}", webhook_id))
                }
                _ => WebhookError::DatabaseError(format!("Failed to get webhook: {}", e)),
            })
    }

    /// List all webhooks
    pub fn list_webhooks(&self) -> WebhookResult<Vec<crate::db::models::Webhook>> {
        use crate::db::schema::webhooks::dsl::*;

        let mut conn = self.get_conn()?;

        webhooks
            .order(created_at.desc())
            .load::<crate::db::models::Webhook>(&mut conn)
            .map_err(|e| WebhookError::DatabaseError(format!("Failed to list webhooks: {}", e)))
    }

    /// List webhooks by service
    pub fn list_webhooks_by_service(
        &self,
        service_name: &str,
    ) -> WebhookResult<Vec<crate::db::models::Webhook>> {
        use crate::db::schema::webhooks::dsl::*;

        let mut conn = self.get_conn()?;

        webhooks
            .filter(service.eq(service_name))
            .filter(enabled.eq(true))
            .order(created_at.desc())
            .load::<crate::db::models::Webhook>(&mut conn)
            .map_err(|e| {
                WebhookError::DatabaseError(format!("Failed to list webhooks by service: {}", e))
            })
    }

    /// Update webhook
    pub fn update_webhook(
        &self,
        webhook_id: &str,
        update: crate::db::models::UpdateWebhook,
    ) -> WebhookResult<()> {
        use crate::db::schema::webhooks::dsl::*;

        let mut conn = self.get_conn()?;

        diesel::update(webhooks.filter(id.eq(webhook_id)))
            .set(&update)
            .execute(&mut conn)
            .map_err(|e| WebhookError::DatabaseError(format!("Failed to update webhook: {}", e)))?;

        Ok(())
    }

    /// Delete webhook (also deletes associated events via CASCADE)
    pub fn delete_webhook(&self, webhook_id: &str) -> WebhookResult<()> {
        use crate::db::schema::webhooks::dsl::*;

        let mut conn = self.get_conn()?;

        let deleted = diesel::delete(webhooks.filter(id.eq(webhook_id)))
            .execute(&mut conn)
            .map_err(|e| WebhookError::DatabaseError(format!("Failed to delete webhook: {}", e)))?;

        if deleted == 0 {
            return Err(WebhookError::NotFound(format!(
                "Webhook not found: {}",
                webhook_id
            )));
        }

        Ok(())
    }

    /// Insert webhook event log
    pub fn insert_event(
        &self,
        event_id: &str,
        webhook_id: &str,
        event_type: &str,
        status: &str,
        payload: Option<&str>,
        response_code: Option<i32>,
        response_body: Option<&str>,
        duration_ms: Option<i32>,
        error_message: Option<&str>,
        created_at: &str,
    ) -> WebhookResult<()> {
        use crate::db::schema::webhook_events;

        let mut conn = self.get_conn()?;

        let new_event = crate::db::models::NewWebhookEvent {
            id: event_id,
            webhook_id,
            event_type,
            status,
            payload,
            response_code,
            response_body,
            duration_ms,
            error_message,
            created_at,
        };

        diesel::insert_into(webhook_events::table)
            .values(&new_event)
            .execute(&mut conn)
            .map_err(|e| WebhookError::DatabaseError(format!("Failed to insert event: {}", e)))?;

        Ok(())
    }

    /// Get webhook events for a specific webhook
    pub fn get_webhook_events(
        &self,
        webhook_id: &str,
        limit: i64,
    ) -> WebhookResult<Vec<crate::db::models::WebhookEvent>> {
        use crate::db::schema::webhook_events::dsl::*;

        let mut conn = self.get_conn()?;

        webhook_events
            .filter(webhook_id.eq(webhook_id))
            .order(created_at.desc())
            .limit(limit)
            .load::<crate::db::models::WebhookEvent>(&mut conn)
            .map_err(|e| {
                WebhookError::DatabaseError(format!("Failed to get webhook events: {}", e))
            })
    }

    /// Get recent webhook events (all webhooks)
    pub fn get_recent_events(
        &self,
        limit: i64,
    ) -> WebhookResult<Vec<crate::db::models::WebhookEvent>> {
        use crate::db::schema::webhook_events::dsl::*;

        let mut conn = self.get_conn()?;

        webhook_events
            .order(created_at.desc())
            .limit(limit)
            .load::<crate::db::models::WebhookEvent>(&mut conn)
            .map_err(|e| WebhookError::DatabaseError(format!("Failed to get recent events: {}", e)))
    }

    /// Get event counts by status
    pub fn get_event_stats(&self, webhook_id_filter: Option<&str>) -> WebhookResult<EventStats> {
        use crate::db::schema::webhook_events::dsl::*;
        use diesel::dsl::count;

        let mut conn = self.get_conn()?;

        // Build base query
        let mut query = webhook_events.into_boxed();

        // Filter by webhook_id if provided
        if let Some(wh_id) = webhook_id_filter {
            query = query.filter(webhook_id.eq(wh_id));
        }

        // Get total count
        let total = query
            .select(count(id))
            .first::<i64>(&mut conn)
            .map_err(|e| WebhookError::DatabaseError(format!("Failed to count events: {}", e)))?;

        // Get success count
        let mut success_query = webhook_events.into_boxed();
        if let Some(wh_id) = webhook_id_filter {
            success_query = success_query.filter(webhook_id.eq(wh_id));
        }
        let success = success_query
            .filter(status.eq("success"))
            .select(count(id))
            .first::<i64>(&mut conn)
            .map_err(|e| {
                WebhookError::DatabaseError(format!("Failed to count success events: {}", e))
            })?;

        // Get error count
        let mut error_query = webhook_events.into_boxed();
        if let Some(wh_id) = webhook_id_filter {
            error_query = error_query.filter(webhook_id.eq(wh_id));
        }
        let errors = error_query
            .filter(status.eq("error"))
            .select(count(id))
            .first::<i64>(&mut conn)
            .map_err(|e| {
                WebhookError::DatabaseError(format!("Failed to count error events: {}", e))
            })?;

        // Get received count
        let mut received_query = webhook_events.into_boxed();
        if let Some(wh_id) = webhook_id_filter {
            received_query = received_query.filter(webhook_id.eq(wh_id));
        }
        let received = received_query
            .filter(event_type.eq("received"))
            .select(count(id))
            .first::<i64>(&mut conn)
            .map_err(|e| {
                WebhookError::DatabaseError(format!("Failed to count received events: {}", e))
            })?;

        // Get sent count
        let mut sent_query = webhook_events.into_boxed();
        if let Some(wh_id) = webhook_id_filter {
            sent_query = sent_query.filter(webhook_id.eq(wh_id));
        }
        let sent = sent_query
            .filter(event_type.eq("sent"))
            .select(count(id))
            .first::<i64>(&mut conn)
            .map_err(|e| {
                WebhookError::DatabaseError(format!("Failed to count sent events: {}", e))
            })?;

        Ok(EventStats {
            total,
            success,
            errors,
            received,
            sent,
        })
    }

    /// Get count of events created today for a specific webhook
    pub fn get_today_events_count(&self, webhook_id_filter: &str) -> WebhookResult<i64> {
        use crate::db::schema::webhook_events::dsl::*;
        use chrono::Utc;
        use diesel::dsl::count;

        let mut conn = self.get_conn()?;

        // Calculate start of today (00:00:00 UTC)
        let today_start = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .to_rfc3339();

        // Count events for this webhook created today, excluding test events
        // Test events have "_test" suffix (e.g., "received_test")
        let count_result = webhook_events
            .filter(webhook_id.eq(webhook_id_filter))
            .filter(created_at.ge(today_start))
            .filter(event_type.not_like("%_test")) // Exclude test events
            .select(count(id))
            .first::<i64>(&mut conn)
            .map_err(|e| {
                WebhookError::DatabaseError(format!("Failed to count today's events: {}", e))
            })?;

        Ok(count_result)
    }

    /// Delete old events (cleanup)
    pub fn delete_old_events(&self, days_old: i64) -> WebhookResult<usize> {
        use crate::db::schema::webhook_events::dsl::*;
        use chrono::Utc;

        let mut conn = self.get_conn()?;

        let cutoff_date = Utc::now() - chrono::Duration::days(days_old);
        let cutoff_str = cutoff_date.to_rfc3339();

        let deleted = diesel::delete(webhook_events.filter(created_at.lt(cutoff_str)))
            .execute(&mut conn)
            .map_err(|e| {
                WebhookError::DatabaseError(format!("Failed to delete old events: {}", e))
            })?;

        Ok(deleted)
    }
}

/// Event statistics
#[derive(Debug, Clone)]
pub struct EventStats {
    pub total: i64,
    pub success: i64,
    pub errors: i64,
    pub received: i64,
    pub sent: i64,
}

impl EventStats {
    pub fn success_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.success as f64 / self.total as f64) * 100.0
        }
    }
}
