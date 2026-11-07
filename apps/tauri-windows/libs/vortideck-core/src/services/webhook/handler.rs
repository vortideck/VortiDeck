//! Platform-agnostic webhook request handler
//! Contains all business logic for processing webhooks without HTTP framework dependencies

use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::Arc;

use super::handler_types::{HttpMethod, WebhookRequest, WebhookResponse};
use super::types::ResponseMode;
use super::{presets::get_preset, WebhookService, WebhookTestSessionAuthority};
use crate::db::models::Webhook;

type HmacSha256 = Hmac<Sha256>;

/// Event sent when a webhook is received and should trigger a flow
#[derive(Debug)]
pub struct WebhookTriggerEvent {
    pub webhook_id: String,
    /// Stable provider delivery/message ID when one was supplied.
    pub event_id: Option<String>,
    pub payload: Value,
    /// Confirms that the canonical envelope was durably accepted before any
    /// workflow side effect begins.
    pub acceptance_channel: tokio::sync::oneshot::Sender<WebhookIngressAcceptance>,
    /// Optional channel to send flow execution result back (for lastNode response mode)
    pub response_channel: Option<tokio::sync::oneshot::Sender<Value>>,
}

#[derive(Debug)]
pub enum WebhookIngressAcceptance {
    Accepted { event_id: String },
    Duplicate { event_id: String },
    Rejected { message: String },
}

#[cfg(test)]
mod tests {
    use super::{WebhookHandler, WebhookIngressAcceptance};
    use crate::services::webhook::{
        WebhookRequest, WebhookService, WebhookStorage, WebhookTestSessionAuthority,
    };
    use diesel::r2d2::{self, ConnectionManager};
    use diesel::SqliteConnection;
    use diesel_migrations::MigrationHarness;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;

    fn service() -> Arc<WebhookService> {
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = r2d2::Pool::builder()
            .max_size(1)
            .build(manager)
            .expect("test pool");
        pool.get()
            .expect("test database")
            .run_pending_migrations(crate::db::db::MIGRATIONS)
            .expect("test migrations");
        Arc::new(WebhookService::new(Arc::new(WebhookStorage::new(pool))))
    }

    fn request(webhook_id: String, headers: HashMap<String, String>) -> WebhookRequest {
        WebhookRequest {
            webhook_id,
            method: super::HttpMethod::POST,
            headers,
            body: br#"{"event":"test"}"#.to_vec(),
            remote_ip: Some("127.0.0.1".to_string()),
        }
    }

    #[test]
    fn extracts_stable_webhook_delivery_id_case_insensitively() {
        let headers = HashMap::from([(
            "Twitch-Eventsub-Message-Id".to_string(),
            " delivery-123 ".to_string(),
        )]);

        assert_eq!(
            WebhookHandler::source_event_id("twitch", &headers).as_deref(),
            Some("delivery-123")
        );
    }

    #[test]
    fn does_not_invent_deduplication_identity_without_provider_header() {
        let headers = HashMap::from([
            ("content-type".to_string(), "application/json".to_string()),
            ("x-request-id".to_string(), "unstable-proxy-id".to_string()),
        ]);
        assert_eq!(WebhookHandler::source_event_id("github", &headers), None);
    }

    #[test]
    fn manual_sender_can_explicitly_supply_idempotency_identity() {
        let headers = HashMap::from([(
            "idempotency-key".to_string(),
            "manual-delivery-1".to_string(),
        )]);
        assert_eq!(
            WebhookHandler::source_event_id("manual", &headers).as_deref(),
            Some("manual-delivery-1")
        );
        assert_eq!(WebhookHandler::source_event_id("github", &headers), None);
    }

    #[tokio::test]
    async fn production_provider_requires_a_valid_signature() {
        let service = service();
        let webhook = service
            .create_webhook("GitHub", None, "github", None, None, None)
            .expect("create webhook");
        service
            .update_webhook(
                &webhook.id,
                None,
                None,
                None,
                Some("test-secret"),
                None,
                None,
                None,
            )
            .expect("set signing secret");
        let authority = Arc::new(WebhookTestSessionAuthority::new());
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = WebhookHandler::new(service, authority, Some(tx), None);

        let unsigned = handler
            .handle_request(request(webhook.id.clone(), HashMap::new()))
            .await;
        assert_eq!(unsigned.status_code, 401);

        let wrong = handler
            .handle_request(request(
                webhook.id.clone(),
                HashMap::from([("x-hub-signature-256".to_string(), "sha256=bad".to_string())]),
            ))
            .await;
        assert_eq!(wrong.status_code, 401);

        let mut mac = Hmac::<Sha256>::new_from_slice(b"test-secret").unwrap();
        mac.update(br#"{"event":"test"}"#);
        let signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));
        let valid_request = request(
            webhook.id,
            HashMap::from([
                ("x-hub-signature-256".to_string(), signature),
                ("x-github-delivery".to_string(), "delivery-1".to_string()),
            ]),
        );
        let accept = tokio::spawn(async move {
            let event = rx.recv().await.expect("accepted event");
            event
                .acceptance_channel
                .send(WebhookIngressAcceptance::Accepted {
                    event_id: "webhook:test:delivery-1".to_string(),
                })
                .unwrap();
        });
        let valid = handler.handle_request(valid_request).await;
        accept.await.unwrap();
        assert_eq!(valid.status_code, 202);
        assert_eq!(valid.body["status"], "accepted");
    }

    #[tokio::test]
    async fn test_ingress_requires_the_webhook_scoped_editor_capability() {
        let service = service();
        let webhook = service
            .create_webhook("Manual", None, "manual", None, None, None)
            .expect("create webhook");
        service
            .activate_test_mode(&webhook.id)
            .expect("activate lifecycle metadata");
        let authority = Arc::new(WebhookTestSessionAuthority::new());
        let capability = authority.issue(&webhook.id, Duration::from_secs(120));
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = WebhookHandler::new(service, authority.clone(), Some(tx), None);

        let missing = handler
            .handle_test_request(request(webhook.id.clone(), HashMap::new()))
            .await;
        assert_eq!(missing.status_code, 404);
        assert!(rx.try_recv().is_err());

        let accept = tokio::spawn(async move {
            let event = rx.recv().await.expect("authorized test event");
            event
                .acceptance_channel
                .send(WebhookIngressAcceptance::Accepted {
                    event_id: "webhook:test:synthetic".to_string(),
                })
                .unwrap();
        });
        let authorized = handler
            .handle_test_request(request(
                webhook.id.clone(),
                HashMap::from([(
                    "authorization".to_string(),
                    format!("Bearer {}", capability.token),
                )]),
            ))
            .await;
        accept.await.unwrap();
        assert_eq!(authorized.status_code, 202);

        authority.revoke(&webhook.id);
        let revoked = handler
            .handle_test_request(request(
                webhook.id,
                HashMap::from([(
                    "authorization".to_string(),
                    format!("Bearer {}", capability.token),
                )]),
            ))
            .await;
        assert_eq!(revoked.status_code, 404);
    }

    #[tokio::test]
    async fn http_acceptance_waits_for_durable_ingress_receipt() {
        let service = service();
        let webhook = service
            .create_webhook("Manual", None, "manual", None, None, None)
            .expect("create webhook");
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = Arc::new(WebhookHandler::new(
            service,
            Arc::new(WebhookTestSessionAuthority::new()),
            Some(tx),
            None,
        ));
        let mut request_task = tokio::spawn({
            let handler = handler.clone();
            let webhook_id = webhook.id.clone();
            async move {
                handler
                    .handle_request(request(webhook_id, HashMap::new()))
                    .await
            }
        });

        let event = rx.recv().await.expect("ingress event");
        assert!(
            tokio::time::timeout(Duration::from_millis(10), &mut request_task)
                .await
                .is_err(),
            "HTTP must not acknowledge before the durable receipt"
        );
        event
            .acceptance_channel
            .send(WebhookIngressAcceptance::Accepted {
                event_id: "webhook:manual:accepted".to_string(),
            })
            .unwrap();
        let response = request_task.await.unwrap();
        assert_eq!(response.status_code, 202);
        assert_eq!(response.body["event_id"], "webhook:manual:accepted");
    }

    #[tokio::test]
    async fn duplicate_delivery_returns_a_deterministic_receipt() {
        let service = service();
        let webhook = service
            .create_webhook("Manual", None, "manual", None, None, None)
            .expect("create webhook");
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = WebhookHandler::new(
            service,
            Arc::new(WebhookTestSessionAuthority::new()),
            Some(tx),
            None,
        );
        let accept = tokio::spawn(async move {
            let event = rx.recv().await.expect("ingress event");
            event
                .acceptance_channel
                .send(WebhookIngressAcceptance::Duplicate {
                    event_id: "webhook:manual:delivery-1".to_string(),
                })
                .unwrap();
        });

        let response = handler
            .handle_request(request(
                webhook.id,
                HashMap::from([("idempotency-key".to_string(), "delivery-1".to_string())]),
            ))
            .await;
        accept.await.unwrap();

        assert_eq!(response.status_code, 200);
        assert_eq!(response.body["status"], "duplicate");
        assert_eq!(response.body["event_id"], "webhook:manual:delivery-1");
    }
}

/// Platform-agnostic webhook handler
/// Processes webhook requests without depending on any HTTP framework
pub struct WebhookHandler {
    webhook_service: Arc<WebhookService>,
    test_sessions: Arc<WebhookTestSessionAuthority>,
    webhook_tx: Option<tokio::sync::mpsc::UnboundedSender<WebhookTriggerEvent>>,
    event_bus: Option<Arc<crate::events::EventBus>>,
}

impl WebhookHandler {
    /// Create new webhook handler
    pub fn new(
        webhook_service: Arc<WebhookService>,
        test_sessions: Arc<WebhookTestSessionAuthority>,
        webhook_tx: Option<tokio::sync::mpsc::UnboundedSender<WebhookTriggerEvent>>,
        event_bus: Option<Arc<crate::events::EventBus>>,
    ) -> Self {
        Self {
            webhook_service,
            test_sessions,
            webhook_tx,
            event_bus,
        }
    }

    /// Handle POST/PUT/etc request (production webhook)
    pub async fn handle_request(&self, request: WebhookRequest) -> WebhookResponse {
        // Get webhook from database
        let webhook = match self.webhook_service.get_webhook(&request.webhook_id) {
            Ok(wh) => wh,
            Err(_) => return WebhookResponse::error(404, "Webhook not found"),
        };

        // Check if enabled
        if !webhook.enabled {
            return WebhookResponse::error(410, "Webhook is disabled");
        }

        // Check HTTP method
        if let Err(msg) = self.check_http_method(&webhook, &request.method) {
            return WebhookResponse::error(405, &msg);
        }

        // Verify external webhook signature (if from Twitch/GitHub/Discord)
        // Only verify if service has a preset with signature verification configured
        if !webhook.service.is_empty() && webhook.service != "manual" && webhook.service != "custom"
        {
            if let Err(msg) = self.verify_external_signature(&webhook, &request) {
                return WebhookResponse::error(401, &msg);
            }
        }

        // Provider verification/ping requests are authenticated too. Handling
        // the challenge before signature verification would create an unsigned
        // provider-specific bypass.
        if let Some(challenge_response) = self.handle_challenge(&webhook, &request) {
            return challenge_response;
        }

        // Note: Additional authentication (BasicAuth, IP whitelist, etc.) can be added
        // as a separate feature if needed. For now, webhooks rely on:
        // 1. Signature verification (inbound_secret) for known services
        // 2. Obscurity of webhook URL (UUID-based path)

        // Parse payload
        let payload = match serde_json::from_slice::<Value>(&request.body) {
            Ok(p) => p,
            Err(_) => return WebhookResponse::error(400, "Webhook body must be valid JSON"),
        };

        let event_id = Self::source_event_id(&webhook.service, &request.headers);

        // Process based on response mode
        self.process_webhook(webhook, payload, event_id, false)
            .await
    }

    /// Handle test mode request
    pub async fn handle_test_request(&self, request: WebhookRequest) -> WebhookResponse {
        let presented_capability = Self::bearer_token(&request.headers);
        if !self
            .test_sessions
            .authorize(&request.webhook_id, presented_capability)
        {
            // Do not disclose whether a webhook or editor session exists.
            return WebhookResponse::error(404, "Test endpoint not found");
        }

        let webhook = match self.webhook_service.get_webhook(&request.webhook_id) {
            Ok(wh) => wh,
            Err(_) => return WebhookResponse::error(404, "Webhook not found"),
        };

        if !webhook.enabled {
            return WebhookResponse::error(410, "Webhook is disabled");
        }
        if let Err(message) = self.check_http_method(&webhook, &request.method) {
            return WebhookResponse::error(405, &message);
        }

        // Check if test mode is active
        if !webhook.is_test_active {
            return WebhookResponse::error(
                404,
                "Test mode not active. Activate it first via the editor.",
            );
        }

        // Check if test mode expired
        if self.webhook_service.is_test_expired(&webhook) {
            // Auto-deactivate
            let _ = self.webhook_service.deactivate_test_mode(&webhook.id);
            return WebhookResponse::error(410, "Test mode expired (120 second timeout)");
        }

        // Parse payload
        let payload = match serde_json::from_slice::<Value>(&request.body) {
            Ok(p) => p,
            Err(_) => return WebhookResponse::error(400, "Webhook body must be valid JSON"),
        };

        log::info!(
            "🧪 Test webhook {} received, executing flow with test mode flag",
            webhook.id
        );

        // Execute flow just like production, but mark as test
        // This respects response modes (onReceived, lastNode, responseNode)
        let event_id = Self::source_event_id(&webhook.service, &request.headers);
        let mut response = self
            .process_webhook(webhook.clone(), payload, event_id, true)
            .await;

        // Add test mode flag to response
        if let Some(body_obj) = response.body.as_object_mut() {
            body_obj.insert("test_mode".to_string(), serde_json::json!(true));
        }

        response
    }

    // ===== PRIVATE HELPER METHODS =====

    /// Check if HTTP method is allowed for this webhook
    fn check_http_method(&self, webhook: &Webhook, method: &HttpMethod) -> Result<(), String> {
        // Deserialize allowed methods from JSON string
        let allowed_methods: Vec<String> = serde_json::from_str(&webhook.allowed_methods)
            .unwrap_or_else(|_| vec!["POST".to_string()]);

        let method_str = method.as_str();
        let is_allowed = allowed_methods
            .iter()
            .any(|m| m.to_uppercase() == method_str);

        if is_allowed {
            Ok(())
        } else {
            Err(format!(
                "Method {} not allowed. Allowed methods: {}",
                method_str,
                allowed_methods.join(", ")
            ))
        }
    }

    /// Verify external webhook signature (Twitch, GitHub, Discord)
    fn verify_external_signature(
        &self,
        webhook: &Webhook,
        request: &WebhookRequest,
    ) -> Result<(), String> {
        // Get preset for this service
        let preset = get_preset(&webhook.service)
            .ok_or_else(|| format!("No preset found for service: {}", webhook.service))?;

        let sig_config = preset
            .signature_verification
            .as_ref()
            .ok_or_else(|| "Service does not support signature verification".to_string())?;

        // Get signature header
        let signature = request
            .headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == sig_config.header_name.to_lowercase())
            .map(|(_, v)| v.as_str())
            .ok_or_else(|| format!("Missing signature header: {}", sig_config.header_name))?;

        // Get secret (inbound_secret for external services)
        let secret = webhook
            .inbound_secret
            .as_ref()
            .ok_or_else(|| "Webhook missing inbound secret".to_string())?;

        // Build message to verify (based on preset config)
        let message = self.build_verification_message(
            &sig_config.message_components,
            &request.headers,
            &request.body,
        );

        // Compute HMAC-SHA256
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|_| "Invalid secret".to_string())?;
        mac.update(message.as_bytes());
        // Parse signature (handle different formats)
        let expected = if signature.starts_with("sha256=") {
            // Twitch/GitHub format
            &signature[7..]
        } else {
            // Discord format (raw hex)
            signature
        };
        let expected =
            hex::decode(expected).map_err(|_| "Signature verification failed".to_string())?;
        mac.verify_slice(&expected)
            .map_err(|_| "Signature verification failed".to_string())
    }

    /// Build message for verification from components
    fn build_verification_message(
        &self,
        components: &[String],
        headers: &HashMap<String, String>,
        body: &[u8],
    ) -> String {
        components
            .iter()
            .map(|comp| {
                if comp == "body" {
                    String::from_utf8_lossy(body).to_string()
                } else {
                    // It's a header name
                    headers
                        .iter()
                        .find(|(k, _)| k.to_lowercase() == comp.to_lowercase())
                        .map(|(_, v)| v.clone())
                        .unwrap_or_default()
                }
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Handle challenge requests (Twitch EventSub, GitHub ping)
    fn handle_challenge(
        &self,
        webhook: &Webhook,
        request: &WebhookRequest,
    ) -> Option<WebhookResponse> {
        // Parse body as JSON
        let body: Value = serde_json::from_slice(&request.body).ok()?;

        match webhook.service.as_str() {
            "twitch" => {
                // Check for Twitch challenge
                let msg_type = request
                    .headers
                    .iter()
                    .find(|(k, _)| k.to_lowercase() == "twitch-eventsub-message-type")
                    .map(|(_, v)| v.as_str())?;

                if msg_type == "webhook_callback_verification" {
                    let challenge = body.get("challenge")?.as_str()?;
                    log::info!("📋 Twitch challenge received for webhook {}", webhook.id);

                    return Some(WebhookResponse::ok(serde_json::json!({
                        "challenge": challenge
                    })));
                }
            }

            "github" => {
                // Check for GitHub ping event
                let event_type = request
                    .headers
                    .iter()
                    .find(|(k, _)| k.to_lowercase() == "x-github-event")
                    .map(|(_, v)| v.as_str())?;

                if event_type == "ping" {
                    log::info!("📋 GitHub ping received for webhook {}", webhook.id);

                    return Some(WebhookResponse::ok(serde_json::json!({
                        "status": "ok",
                        "message": "GitHub ping acknowledged"
                    })));
                }
            }

            _ => {}
        }

        None
    }

    /// Process webhook based on response mode
    async fn process_webhook(
        &self,
        webhook: Webhook,
        payload: Value,
        event_id: Option<String>,
        is_test: bool,
    ) -> WebhookResponse {
        use crate::services::webhook::types::deserialize_response_mode;

        let response_mode = deserialize_response_mode(&webhook.response_mode);

        if response_mode == ResponseMode::Streaming {
            return WebhookResponse::error(501, "Streaming mode not implemented yet");
        }

        let Some(webhook_tx) = &self.webhook_tx else {
            return WebhookResponse::error(503, "Webhook workflow ingress is unavailable");
        };
        let (acceptance_tx, acceptance_rx) = tokio::sync::oneshot::channel();
        let (response_tx, response_rx) = if response_mode == ResponseMode::OnReceived {
            (None, None)
        } else {
            let (tx, rx) = tokio::sync::oneshot::channel();
            (Some(tx), Some(rx))
        };
        let event = WebhookTriggerEvent {
            webhook_id: webhook.id.clone(),
            event_id: event_id.clone(),
            payload: payload.clone(),
            acceptance_channel: acceptance_tx,
            response_channel: response_tx,
        };
        if webhook_tx.send(event).is_err() {
            return WebhookResponse::error(503, "Webhook workflow ingress is unavailable");
        }

        let acceptance =
            match tokio::time::timeout(std::time::Duration::from_secs(3), acceptance_rx).await {
                Ok(Ok(acceptance)) => acceptance,
                Ok(Err(_)) => {
                    return WebhookResponse::error(503, "Webhook acceptance channel closed")
                }
                Err(_) => return WebhookResponse::error(503, "Webhook persistence timed out"),
            };
        let accepted_event_id = match acceptance {
            WebhookIngressAcceptance::Accepted { event_id } => event_id,
            WebhookIngressAcceptance::Duplicate { event_id } => {
                return WebhookResponse::ok(serde_json::json!({
                    "status": "duplicate",
                    "event_id": event_id,
                    "webhook_id": webhook.id,
                }))
            }
            WebhookIngressAcceptance::Rejected { message } => {
                return WebhookResponse::error(503, &message)
            }
        };

        let payload_str = serde_json::to_string(&payload).unwrap_or_default();
        let _ = self.webhook_service.log_webhook_event(
            &webhook.id,
            "received",
            "accepted",
            Some(&payload_str),
            Some(if response_mode == ResponseMode::OnReceived {
                202
            } else {
                200
            }),
            None,
            None,
            None,
            is_test,
        );
        self.emit_observation(&webhook, payload, event_id, is_test);

        if response_mode == ResponseMode::OnReceived {
            return WebhookResponse {
                status_code: 202,
                body: serde_json::json!({
                    "status": "accepted",
                    "event_id": accepted_event_id,
                    "webhook_id": webhook.id,
                }),
                headers: HashMap::new(),
            };
        }

        let response_rx = response_rx.expect("response modes create a receiver");
        match tokio::time::timeout(std::time::Duration::from_secs(30), response_rx).await {
            Ok(Ok(result)) if response_mode == ResponseMode::ResponseNode => {
                WebhookResponse::ok(result)
            }
            Ok(Ok(result)) => WebhookResponse::ok(serde_json::json!({
                "status": "ok",
                "result": result,
                "event_id": accepted_event_id,
                "webhook_id": webhook.id,
            })),
            Ok(Err(_)) => WebhookResponse::error(500, "Flow response channel closed"),
            Err(_) if response_mode == ResponseMode::ResponseNode => WebhookResponse::error(
                504,
                "No Respond to Webhook node executed within 30s timeout",
            ),
            Err(_) => WebhookResponse::error(504, "Flow execution timeout (30s)"),
        }
    }

    fn emit_observation(
        &self,
        webhook: &Webhook,
        payload: Value,
        event_id: Option<String>,
        is_test: bool,
    ) {
        let Some(event_bus) = &self.event_bus else {
            return;
        };
        use crate::events::types::{EventPayload, WebhookEventPayload};
        use chrono::Utc;

        let flow_id = webhook.action.as_ref().and_then(|action_str| {
            serde_json::from_str::<crate::db::models::Action>(action_str)
                .ok()
                .and_then(|action| match action {
                    crate::db::models::Action::FlowId(id) => Some(id),
                    _ => None,
                })
        });
        let _ = event_bus.emit(EventPayload::Webhook(
            WebhookEventPayload::WebhookReceived {
                webhook_id: webhook.id.clone(),
                timestamp: Utc::now().timestamp_millis(),
                event_id,
                service: Some(webhook.service.clone()),
                flow_id,
                payload: Some(payload),
                is_test,
            },
        ));
    }

    fn source_event_id(service: &str, headers: &HashMap<String, String>) -> Option<String> {
        // Delivery identity is provider-specific. An arbitrary request ID is
        // not necessarily stable across retries and must not suppress a real
        // event. Manual/custom senders opt in through Idempotency-Key.
        let candidates: &[&str] = match service {
            "twitch" => &["twitch-eventsub-message-id"],
            "github" => &["x-github-delivery"],
            "manual" | "custom" => &["idempotency-key"],
            _ => &[],
        };

        candidates.iter().find_map(|candidate| {
            headers
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(candidate))
                .map(|(_, value)| value.trim())
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
    }

    fn bearer_token(headers: &HashMap<String, String>) -> Option<&str> {
        headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
            .and_then(|(_, value)| value.trim().strip_prefix("Bearer "))
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }
}
