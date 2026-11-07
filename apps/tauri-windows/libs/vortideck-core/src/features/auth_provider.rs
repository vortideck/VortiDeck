//! Auth provider implementation for VortiDeck
//!
//! This module provides token-based authentication for premium feature validation.

use crate::features::validator::AuthProvider;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PremiumResponse {
    pub is_premium: bool,
    pub subscription_type: Option<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Configuration for the auth provider
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub backend_url: String,
    pub premium_endpoint: String,
    pub refresh_endpoint: String,
    pub timeout_seconds: u64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            backend_url: "https://api.vortideck.com".to_string(),
            premium_endpoint: "/api/v1/auth/premium-status".to_string(),
            refresh_endpoint: "/api/v1/auth/token/refresh".to_string(),
            timeout_seconds: 10,
        }
    }
}

/// VortiDeck auth provider that integrates with existing StateManager auth system
pub struct VortiDeckAuthProvider {
    /// Configuration for API endpoints
    config: AuthConfig,
    /// HTTP client for API calls
    client: reqwest::Client,
    /// Current token data
    token_data: Arc<RwLock<Option<TokenData>>>,
}

impl VortiDeckAuthProvider {
    /// Create new auth provider with default config
    pub fn new() -> Self {
        Self::with_config(AuthConfig::default())
    }

    /// Create new auth provider with custom config
    pub fn with_config(config: AuthConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            config,
            client,
            token_data: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize with existing tokens from StateManager
    pub async fn initialize_tokens(&self, access_token: String, refresh_token: String) {
        let mut tokens = self.token_data.write().await;
        *tokens = Some(TokenData {
            access_token,
            refresh_token,
            expires_at: chrono::Utc::now() + chrono::Duration::hours(1), // Default 1 hour expiry
        });
    }

    /// Update token expiry time
    pub async fn update_token_expiry(&self, expires_at: chrono::DateTime<chrono::Utc>) {
        let mut tokens = self.token_data.write().await;
        if let Some(ref mut token_data) = tokens.as_mut() {
            token_data.expires_at = expires_at;
        }
    }
}

#[async_trait]
impl AuthProvider for VortiDeckAuthProvider {
    /// Validate premium status from JWT token (no backend call needed)
    async fn validate_premium_status(&self, access_token: &str) -> Result<bool, String> {
        // Update JWT provider with current token and extract premium status
        use crate::features::jwt_features::{is_user_premium, update_jwt_token};

        // Update the JWT provider with the current token
        update_jwt_token(access_token)?;

        // Return premium status directly from JWT token
        Ok(is_user_premium())
    }

    /// Refresh access token using refresh token
    async fn refresh_access_token(&self, refresh_token: &str) -> Result<String, String> {
        let url = format!(
            "{}{}",
            self.config.backend_url, self.config.refresh_endpoint
        );

        #[derive(Deserialize)]
        struct RefreshResponse {
            access_token: String,
            refresh_token: Option<String>,
            expires_in: Option<u64>,
        }

        // Match your auth.rs approach: POST with JSON body and X-Client-Type header
        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-Client-Type", "native")
            .json(&serde_json::json!({ "refresh": refresh_token }))
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        match response.status().as_u16() {
            200 => {
                let refresh_response: RefreshResponse = response
                    .json()
                    .await
                    .map_err(|e| format!("JSON parse error: {}", e))?;

                // Update stored tokens
                let mut tokens = self.token_data.write().await;
                let expires_at = chrono::Utc::now()
                    + chrono::Duration::seconds(refresh_response.expires_in.unwrap_or(3600) as i64);

                *tokens = Some(TokenData {
                    access_token: refresh_response.access_token.clone(),
                    refresh_token: refresh_response
                        .refresh_token
                        .unwrap_or_else(|| refresh_token.to_string()),
                    expires_at,
                });

                Ok(refresh_response.access_token)
            }
            401 => Err("Refresh token expired or invalid".to_string()),
            _ => Err(format!("Server error: {}", response.status())),
        }
    }

    /// Get current access token
    async fn get_access_token(&self) -> Option<String> {
        let tokens = self.token_data.read().await;
        tokens.as_ref().map(|t| t.access_token.clone())
    }

    /// Get current refresh token
    async fn get_refresh_token(&self) -> Option<String> {
        let tokens = self.token_data.read().await;
        tokens.as_ref().map(|t| t.refresh_token.clone())
    }

    /// Clear tokens (on logout/expiry)
    async fn clear_tokens(&self) {
        let mut tokens = self.token_data.write().await;
        *tokens = None;
    }
}

/// Mock auth provider for testing
pub struct MockAuthProvider {
    is_premium: bool,
    should_fail: bool,
}

impl MockAuthProvider {
    pub fn new(is_premium: bool) -> Self {
        Self {
            is_premium,
            should_fail: false,
        }
    }

    pub fn with_failure() -> Self {
        Self {
            is_premium: false,
            should_fail: true,
        }
    }
}

#[async_trait]
impl AuthProvider for MockAuthProvider {
    async fn validate_premium_status(&self, _access_token: &str) -> Result<bool, String> {
        if self.should_fail {
            Err("Mock auth failure".to_string())
        } else {
            Ok(self.is_premium)
        }
    }

    async fn refresh_access_token(&self, _refresh_token: &str) -> Result<String, String> {
        if self.should_fail {
            Err("Mock refresh failure".to_string())
        } else {
            Ok("mock_new_access_token".to_string())
        }
    }

    async fn get_access_token(&self) -> Option<String> {
        if self.should_fail {
            None
        } else {
            Some("mock_access_token".to_string())
        }
    }

    async fn get_refresh_token(&self) -> Option<String> {
        if self.should_fail {
            None
        } else {
            Some("mock_refresh_token".to_string())
        }
    }

    async fn clear_tokens(&self) {
        // Mock implementation - nothing to clear
    }
}

/// StateManager-integrated auth provider that reads from existing auth state
pub struct StateManagerAuthProvider {
    auth_state: Arc<tokio::sync::RwLock<crate::core::state_manager::AuthState>>,
    config: AuthConfig,
    client: reqwest::Client,
}

impl StateManagerAuthProvider {
    pub fn new(
        auth_state: Arc<tokio::sync::RwLock<crate::core::state_manager::AuthState>>,
    ) -> Self {
        let config = AuthConfig::default();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            auth_state,
            config,
            client,
        }
    }

    pub fn with_config(
        auth_state: Arc<tokio::sync::RwLock<crate::core::state_manager::AuthState>>,
        config: AuthConfig,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            auth_state,
            config,
            client,
        }
    }
}

#[async_trait]
impl AuthProvider for StateManagerAuthProvider {
    async fn validate_premium_status(&self, access_token: &str) -> Result<bool, String> {
        // Extract premium status directly from JWT token (no backend call needed)
        use crate::features::jwt_features::{is_user_premium, update_jwt_token};

        // Update the JWT provider with the current token
        update_jwt_token(access_token)?;

        // Return premium status directly from JWT token
        Ok(is_user_premium())
    }

    async fn refresh_access_token(&self, refresh_token: &str) -> Result<String, String> {
        let url = format!(
            "{}{}",
            self.config.backend_url, self.config.refresh_endpoint
        );

        #[derive(Deserialize)]
        struct RefreshResponse {
            access_token: String,
            refresh_token: Option<String>,
        }

        // Match your auth.rs approach: POST with JSON body and X-Client-Type header
        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-Client-Type", "native")
            .json(&serde_json::json!({ "refresh": refresh_token }))
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        match response.status().as_u16() {
            200 => {
                let refresh_response: RefreshResponse = response
                    .json()
                    .await
                    .map_err(|e| format!("JSON parse error: {}", e))?;

                // Update StateManager with new tokens
                let mut auth_state = self.auth_state.write().await;
                auth_state
                    .access_tokens
                    .insert("primary".to_string(), refresh_response.access_token.clone());

                if let Some(new_refresh_token) = refresh_response.refresh_token {
                    auth_state
                        .refresh_tokens
                        .insert("primary".to_string(), new_refresh_token);
                }

                Ok(refresh_response.access_token)
            }
            401 => Err("Refresh token expired or invalid".to_string()),
            _ => Err(format!("Server error: {}", response.status())),
        }
    }

    async fn get_access_token(&self) -> Option<String> {
        let auth_state = self.auth_state.read().await;
        auth_state.access_tokens.get("primary").cloned()
    }

    async fn get_refresh_token(&self) -> Option<String> {
        let auth_state = self.auth_state.read().await;
        auth_state.refresh_tokens.get("primary").cloned()
    }

    async fn clear_tokens(&self) {
        let mut auth_state = self.auth_state.write().await;
        auth_state.access_tokens.clear();
        auth_state.refresh_tokens.clear();
    }
}
