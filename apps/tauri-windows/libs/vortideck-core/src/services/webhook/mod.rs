//! Webhook Service for VortiDeck
//!
//! Enterprise-grade webhook integration for automation:
//! - Receive webhooks from external services (GitHub, Slack, Discord, Twitch, etc.)
//! - HMAC-SHA256 signature verification for external services
//! - Database-backed persistence with event logging
//! - Execute any UniversalAction (not just flows)
//! - Rate limiting and retry logic
//!
//! ## Architecture
//! - **errors**: Comprehensive error types
//! - **storage**: Database persistence layer
//! - **service**: Main WebhookService with CRUD operations
//! - **handler**: Inbound webhook handling with external service verification
//! - **registrar**: Register webhooks WITH external services (Twitch, GitHub)
//! - **types**: Phase 1 types (ResponseMode, HttpMethod)
//! - **presets**: Service-specific webhook presets (Twitch, GitHub, Discord)

// Public modules
pub mod errors;
pub mod handler;
pub mod handler_types;
pub mod presets;
pub mod registrar;
pub mod service;
pub mod storage;
mod test_session;
pub mod types;

// Public exports
pub use errors::{WebhookError, WebhookResult};
pub use presets::{EventType, RegistrationConfig, SignatureConfig, WebhookPreset};
pub use registrar::{OAuthTokenProvider, WebhookRegistrar, WebhookRegistration};
pub use service::WebhookService;
pub use storage::{EventStats, WebhookStorage};
pub use test_session::{WebhookTestCapability, WebhookTestSessionAuthority};
pub use types::{
    deserialize_http_methods, serialize_http_methods, validate_http_methods,
    validate_response_mode, HttpMethod, ResponseMode,
};

// Platform-agnostic handler exports (replaces old WebhookServer)
pub use handler::{WebhookHandler, WebhookIngressAcceptance, WebhookTriggerEvent};
pub use handler_types::{HttpMethod as WebhookHttpMethod, WebhookRequest, WebhookResponse};

// Note: WebhookServiceState is defined below (not in a separate module)
// It's exported as part of this module's public API

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Platform-agnostic state wrapper for WebhookService
/// Used in Tauri commands to share webhook service across handlers
#[derive(Clone)]
pub struct WebhookServiceState {
    pub service: Arc<Mutex<WebhookService>>,
    pub test_sessions: Arc<WebhookTestSessionAuthority>,
}

impl WebhookServiceState {
    pub fn new(service: WebhookService) -> Self {
        // Test capabilities are process-local. Reconcile any persisted flag
        // left by a crash so the UI never advertises an authority that cannot
        // survive restart by design.
        if let Ok(webhooks) = service.list_webhooks() {
            for webhook in webhooks
                .into_iter()
                .filter(|webhook| webhook.is_test_active)
            {
                let _ = service.deactivate_test_mode(&webhook.id);
            }
        }
        Self {
            service: Arc::new(Mutex::new(service)),
            test_sessions: Arc::new(WebhookTestSessionAuthority::new()),
        }
    }
}

/// Service template for webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceTemplate {
    pub name: String,
    pub receive_events: Vec<String>,
    pub send_format: Value,
    pub requires_secret: bool,
    pub documentation: String,
}

/// Get built-in service templates
pub fn get_service_templates() -> HashMap<String, ServiceTemplate> {
    let mut templates = HashMap::new();

    templates.insert(
        "github".to_string(),
        ServiceTemplate {
            name: "GitHub".to_string(),
            receive_events: vec!["push", "pull_request", "issue", "release"]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            send_format: serde_json::json!({
                "body": "{{message}}",
                "title": "VortiDeck Notification"
            }),
            requires_secret: true,
            documentation: "https://docs.github.com/en/webhooks".to_string(),
        },
    );

    templates.insert(
        "slack".to_string(),
        ServiceTemplate {
            name: "Slack".to_string(),
            receive_events: vec!["message", "command"]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            send_format: serde_json::json!({
                "text": "{{message}}",
                "username": "VortiDeck"
            }),
            requires_secret: false,
            documentation: "https://api.slack.com/messaging/webhooks".to_string(),
        },
    );

    templates.insert(
        "discord".to_string(),
        ServiceTemplate {
            name: "Discord".to_string(),
            receive_events: vec!["message"].into_iter().map(|s| s.to_string()).collect(),
            send_format: serde_json::json!({
                "content": "{{message}}",
                "username": "VortiDeck"
            }),
            requires_secret: false,
            documentation: "https://discord.com/developers/docs/resources/webhook".to_string(),
        },
    );

    templates
}
