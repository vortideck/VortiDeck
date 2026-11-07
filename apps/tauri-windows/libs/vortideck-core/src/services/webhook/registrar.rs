///! Webhook Registration Service (OUTBOUND operations only)
///!
///! This service handles registering VortiDeck webhooks WITH external services like Twitch, GitHub, Discord.
///! It does NOT handle receiving or verifying incoming webhooks - that's done by server.rs and verification.rs.
///!
///! ## Responsibilities:
///! - Register webhook subscriptions with external APIs (POST)
///! - Check if webhooks still exist (GET)
///! - Unregister webhooks when no longer needed (DELETE)
///! - Generate secrets for external services to use when signing webhooks
///!
///! ## NOT Responsible For:
///! - Receiving webhook HTTP requests (see server.rs)
///! - Verifying webhook signatures (see verification.rs for internal, server.rs for external)
///! - Handling webhook challenges (see server.rs)
///!
///! ## Architecture:
///! ```
///! WebhookRegistrar → External Service API (Twitch, GitHub, etc.)
///!   ↓
///! External Service → server.rs → verification → WebhookService → FlowExecutor
///! ```
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::services::api::{delete, get, post, ApiResult};
use crate::services::webhook::presets::{self, WebhookPreset};

/// Result of webhook registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookRegistration {
    /// Subscription ID returned by external service
    pub subscription_id: String,

    /// Secret used for signature verification
    pub secret: String,

    /// Full webhook URL that was registered
    pub webhook_url: String,
}

/// OAuth token provider trait - implement this to provide tokens from auth service
#[async_trait::async_trait]
pub trait OAuthTokenProvider: Send + Sync {
    /// Get OAuth access token for a service
    async fn get_token(&self, service: &str) -> Result<String, String>;

    /// Refresh token if expired
    async fn refresh_token(&self, service: &str) -> Result<String, String>;
}

/// Generic webhook registrar - works with any service that has a preset
pub struct WebhookRegistrar {
    /// OAuth token provider for authentication
    token_provider: Option<Arc<dyn OAuthTokenProvider>>,
}

impl WebhookRegistrar {
    /// Create new webhook registrar
    pub fn new() -> Self {
        Self {
            token_provider: None,
        }
    }

    /// Create with OAuth token provider
    pub fn with_token_provider(token_provider: Arc<dyn OAuthTokenProvider>) -> Self {
        Self {
            token_provider: Some(token_provider),
        }
    }

    /// Register webhook with external service using preset configuration
    ///
    /// # Arguments
    /// * `service` - Service name (e.g., "twitch", "kick", "github")
    /// * `event_type` - Event type to subscribe to (e.g., "stream.online")
    /// * `webhook_url` - The webhook URL that will receive events
    /// * `config` - Additional configuration (e.g., broadcaster_id for Twitch)
    ///
    /// # Returns
    /// WebhookRegistration with subscription_id and secret
    pub async fn register_webhook(
        &self,
        service: &str,
        event_type: &str,
        webhook_url: &str,
        config: &Value,
    ) -> Result<WebhookRegistration, String> {
        log::info!(
            "🔗 Registering webhook: service={}, event={}",
            service,
            event_type
        );

        // Get preset for this service
        let preset = presets::get_preset(service)
            .ok_or_else(|| format!("No preset found for service: {}", service))?;

        // Get registration config
        let reg_config = preset.registration.as_ref().ok_or_else(|| {
            format!(
                "Service '{}' does not support automatic registration",
                service
            )
        })?;

        // Generate secret for signature verification
        let secret = self.generate_secret();

        // Build context for template rendering
        let context = json!({
            "event_type": event_type,
            "webhook_url": webhook_url,
            "secret": secret,
            "config": config,
        });

        // Render body template
        let body = presets::render_template(&reg_config.body_template, &context)
            .map_err(|e| format!("Failed to render registration template: {}", e))?;

        log::debug!(
            "📤 Registration request body: {}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );

        // Get OAuth token if provider is available
        let token = if let Some(provider) = &self.token_provider {
            Some(provider.get_token(service).await?)
        } else {
            None
        };

        // Make API request
        let response = self
            .make_registration_request(
                &reg_config.api_url,
                &reg_config.method,
                &reg_config.auth_type,
                token.as_deref(),
                body,
            )
            .await?;

        // Extract subscription ID from response
        let subscription_id = self.extract_subscription_id(&response, service)?;

        log::info!(
            "✅ Webhook registered successfully: subscription_id={}",
            subscription_id
        );

        Ok(WebhookRegistration {
            subscription_id,
            secret,
            webhook_url: webhook_url.to_string(),
        })
    }

    /// Check if webhook still exists on external service
    pub async fn check_webhook_exists(
        &self,
        service: &str,
        subscription_id: &str,
    ) -> Result<bool, String> {
        log::debug!(
            "🔍 Checking webhook existence: service={}, id={}",
            service,
            subscription_id
        );

        let preset = presets::get_preset(service)
            .ok_or_else(|| format!("No preset found for service: {}", service))?;

        let reg_config = preset
            .registration
            .as_ref()
            .ok_or_else(|| format!("Service '{}' does not support webhook checking", service))?;

        // Construct check URL (usually GET /webhooks/{id})
        let check_url = format!(
            "{}/{}",
            reg_config.api_url.trim_end_matches('/'),
            subscription_id
        );

        // Get OAuth token
        let token = if let Some(provider) = &self.token_provider {
            Some(provider.get_token(service).await?)
        } else {
            None
        };

        // Make GET request
        match self.make_check_request(&check_url, token.as_deref()).await {
            Ok(_) => {
                log::debug!("✅ Webhook exists");
                Ok(true)
            }
            Err(e) if e.contains("404") || e.contains("not found") => {
                log::debug!("❌ Webhook not found");
                Ok(false)
            }
            Err(e) => Err(e),
        }
    }

    /// Unregister webhook from external service
    pub async fn unregister_webhook(
        &self,
        service: &str,
        subscription_id: &str,
    ) -> Result<(), String> {
        log::info!(
            "🗑️ Unregistering webhook: service={}, id={}",
            service,
            subscription_id
        );

        let preset = presets::get_preset(service)
            .ok_or_else(|| format!("No preset found for service: {}", service))?;

        let reg_config = preset
            .registration
            .as_ref()
            .ok_or_else(|| format!("Service '{}' does not support unregistration", service))?;

        // Render unregister URL template
        let context = json!({
            "subscription_id": subscription_id,
        });

        let unregister_url = presets::render_template_string(&reg_config.unregister_url, &context)
            .map_err(|e| format!("Failed to render unregister URL: {}", e))?;

        // Get OAuth token
        let token = if let Some(provider) = &self.token_provider {
            Some(provider.get_token(service).await?)
        } else {
            None
        };

        // Make DELETE request
        self.make_unregister_request(&unregister_url, token.as_deref())
            .await?;

        log::info!("✅ Webhook unregistered successfully");

        Ok(())
    }

    /// Generate random secret for signature verification
    fn generate_secret(&self) -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut rng = rand::thread_rng();

        (0..32)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// Make registration API request
    async fn make_registration_request(
        &self,
        url: &str,
        method: &str,
        auth_type: &str,
        token: Option<&str>,
        body: Value,
    ) -> Result<Value, String> {
        log::debug!("📤 Making registration request: {} {}", method, url);

        // Build request based on method
        let mut request = match method.to_uppercase().as_str() {
            "POST" => post(url),
            "PUT" => crate::services::api::put(url),
            _ => return Err(format!("Unsupported HTTP method: {}", method)),
        };

        // Add authentication based on type
        if let Some(token) = token {
            request = match auth_type {
                "bearer" => request.bearer_token(token),
                "token" => request.auth(format!("Token {}", token)),
                "api_key" => request.api_key(token),
                _ => request.bearer_token(token), // Default to bearer
            };
        }

        // Add body and send
        let response = request
            .json(body)
            .timeout_ms(10000) // 10 second timeout
            .send()
            .await
            .map_err(|e| format!("Registration request failed: {}", e))?;

        // Check status
        if !response.is_success() {
            return Err(format!(
                "Registration failed with status {}: {}",
                response.status,
                response.text()
            ));
        }

        // Parse JSON response
        response
            .json_value()
            .map_err(|e| format!("Failed to parse registration response: {}", e))
    }

    /// Make check request (GET)
    async fn make_check_request(&self, url: &str, token: Option<&str>) -> Result<Value, String> {
        log::debug!("🔍 Checking webhook: GET {}", url);

        let mut request = get(url);

        // Add authentication
        if let Some(token) = token {
            request = request.bearer_token(token);
        }

        // Send request
        let response = request
            .timeout_ms(5000)
            .send()
            .await
            .map_err(|e| format!("Check request failed: {}", e))?;

        // Check if exists (200 = exists, 404 = not found)
        if response.status == 404 {
            return Err("404 Not Found".to_string());
        }

        if !response.is_success() {
            return Err(format!("Check failed with status {}", response.status));
        }

        response
            .json_value()
            .map_err(|e| format!("Failed to parse check response: {}", e))
    }

    /// Make unregister request (DELETE)
    async fn make_unregister_request(&self, url: &str, token: Option<&str>) -> Result<(), String> {
        log::debug!("🗑️ Unregistering webhook: DELETE {}", url);

        let mut request = delete(url);

        // Add authentication
        if let Some(token) = token {
            request = request.bearer_token(token);
        }

        // Send request
        let response = request
            .timeout_ms(5000)
            .send()
            .await
            .map_err(|e| format!("Unregister request failed: {}", e))?;

        // Accept 2xx or 404 (already deleted) as success
        if response.is_success() || response.status == 404 {
            Ok(())
        } else {
            Err(format!("Unregister failed with status {}", response.status))
        }
    }

    /// Extract subscription ID from registration response
    fn extract_subscription_id(&self, response: &Value, service: &str) -> Result<String, String> {
        // Try common patterns
        if let Some(id) = response.get("id").and_then(|v| v.as_str()) {
            return Ok(id.to_string());
        }

        if let Some(id) = response
            .get("data")
            .and_then(|d| d.get("id"))
            .and_then(|v| v.as_str())
        {
            return Ok(id.to_string());
        }

        if let Some(id) = response.get("subscription_id").and_then(|v| v.as_str()) {
            return Ok(id.to_string());
        }

        Err(format!(
            "Could not extract subscription ID from response for service '{}'",
            service
        ))
    }
}

impl Default for WebhookRegistrar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_secret() {
        let registrar = WebhookRegistrar::new();
        let secret = registrar.generate_secret();

        assert_eq!(secret.len(), 32);
        assert!(secret.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_extract_subscription_id() {
        let registrar = WebhookRegistrar::new();

        // Test direct id
        let response1 = json!({"id": "test123"});
        assert_eq!(
            registrar
                .extract_subscription_id(&response1, "test")
                .unwrap(),
            "test123"
        );

        // Test nested id
        let response2 = json!({"data": {"id": "test456"}});
        assert_eq!(
            registrar
                .extract_subscription_id(&response2, "test")
                .unwrap(),
            "test456"
        );

        // Test subscription_id field
        let response3 = json!({"subscription_id": "test789"});
        assert_eq!(
            registrar
                .extract_subscription_id(&response3, "test")
                .unwrap(),
            "test789"
        );
    }
}
