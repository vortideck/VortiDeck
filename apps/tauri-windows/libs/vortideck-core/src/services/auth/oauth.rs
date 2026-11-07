//! OAuth Service - Authentication Flows
//!
//! This module handles OAuth code exchange and token refresh operations.
//! It provides pure functions without Tauri dependencies.

use super::token_storage::{TokenStorage, TokenStorageError};
use async_trait::async_trait;
use serde_json::json;
use std::env;
use std::error::Error;

/// HTTP response abstraction
#[derive(Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

/// HTTP client abstraction trait
#[async_trait]
pub trait HttpClient {
    async fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        json_body: &serde_json::Value,
    ) -> Result<HttpResponse, String>;
}

/// Reqwest HTTP client implementation
pub struct ReqwestClient {
    client: reqwest::Client,
}

impl ReqwestClient {
    pub fn new() -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .cookie_store(false)
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        Ok(Self { client })
    }
}

// Note: TauriHttpClient implementation should be in the Tauri adapter layer,
// not in the core library to maintain platform independence.
//
// Example implementation for your Tauri adapter:
//
// use vortideck_core::services::auth::oauth::{HttpClient, HttpResponse};
// use tauri_plugin_http::reqwest;
//
// pub struct TauriHttpClient {
//     app_handle: tauri::AppHandle,
// }
//
// impl TauriHttpClient {
//     pub fn new(app_handle: tauri::AppHandle) -> Self {
//         Self { app_handle }
//     }
// }
//
// #[async_trait]
// impl HttpClient for TauriHttpClient {
//     async fn post_json(
//         &self,
//         url: &str,
//         headers: &[(&str, &str)],
//         json_body: &serde_json::Value,
//     ) -> Result<HttpResponse, String> {
//         let client = reqwest::ClientBuilder::new()
//             .cookie_store(false)
//             .build()
//             .map_err(|e| format!("Failed to build Tauri HTTP client: {}", e))?;
//
//         let mut request = client.post(url);
//         for (key, value) in headers {
//             request = request.header(*key, *value);
//         }
//
//         let response = request
//             .json(json_body)
//             .send()
//             .await
//             .map_err(|e| format!("Failed to send request: {}", e))?;
//
//         let status = response.status().as_u16();
//         let body = response.text().await.map_err(|e| format!("Failed to get response text: {}", e))?;
//
//         Ok(HttpResponse { status, body })
//     }
// }

#[async_trait]
impl HttpClient for ReqwestClient {
    async fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        json_body: &serde_json::Value,
    ) -> Result<HttpResponse, String> {
        let mut request = self.client.post(url);

        // Add headers
        for (key, value) in headers {
            request = request.header(*key, *value);
        }

        let response = request
            .json(json_body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = response.status().as_u16();
        let body = response.text().await.map_err(|e| e.to_string())?;

        Ok(HttpResponse { status, body })
    }
}

/// OAuth operation results
#[derive(Debug, Clone)]
pub struct OAuthResult {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
}

/// OAuth errors
#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    #[error("API URL not configured")]
    ApiUrlNotSet,
    #[error("HTTP request failed: {0}")]
    HttpError(String),
    #[error("Failed to parse response: {0}")]
    ParseError(String),
    #[error("Token storage error: {0}")]
    StorageError(#[from] TokenStorageError),
    #[error("Authentication failed: HTTP {0}")]
    AuthenticationFailed(u16),
}

/// OAuth service
pub struct OAuthService;

impl OAuthService {
    /// Exchange authorization code for tokens
    pub async fn exchange_auth_code<C: HttpClient>(
        client: &C,
        api_url: &str,
        code: &str,
        login_id: &str,
    ) -> Result<OAuthResult, OAuthError> {
        if api_url.is_empty() {
            return Err(OAuthError::ApiUrlNotSet);
        }

        let exchange_url = format!("{}/api/v1/auth/google-code-exchange", api_url);
        println!("Exchange URL: {}", exchange_url);

        let headers = &[
            ("Content-Type", "application/json"),
            ("X-Client-Type", "native"),
        ];

        let json_body = json!({ "code": code, "login_id": login_id });

        let response = client
            .post_json(&exchange_url, headers, &json_body)
            .await
            .map_err(|e| OAuthError::HttpError(e))?;

        if response.status >= 400 {
            return Err(OAuthError::AuthenticationFailed(response.status));
        }

        let response_data = serde_json::from_str::<serde_json::Value>(&response.body)
            .map_err(|e| OAuthError::ParseError(e.to_string()))?;

        // Extract tokens
        let access_token = response_data
            .get("access_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let refresh_token = response_data
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Save tokens
        if let Some(ref refresh) = refresh_token {
            TokenStorage::set_refresh_token(refresh)?;
        } else {
            println!("Warning: No refresh token found in response body.");
        }

        if let Some(ref access) = access_token {
            TokenStorage::set_access_token(access)?;

            // Update JWT token in the feature system
            if let Err(e) = crate::features::jwt_features::update_jwt_token(access) {
                println!("Warning: Failed to update JWT token: {}", e);
            }
        }

        println!("Successfully exchanged auth code and stored tokens.");

        Ok(OAuthResult {
            access_token,
            refresh_token,
        })
    }

    /// Refresh access token using stored refresh token
    pub async fn refresh_access_token<C: HttpClient>(
        client: &C,
        api_url: &str,
    ) -> Result<String, OAuthError> {
        if api_url.is_empty() {
            return Err(OAuthError::ApiUrlNotSet);
        }

        // Get stored refresh token
        let stored_token = TokenStorage::get_refresh_token()?;
        let token = stored_token
            .split(";")
            .next()
            .ok_or_else(|| {
                OAuthError::ParseError(
                    "Failed to parse token: no value before semicolon".to_string(),
                )
            })?
            .to_string();

        let refresh_url = format!("{}/api/v1/auth/token/refresh", api_url);

        let headers = &[
            ("Content-Type", "application/json"),
            ("X-Client-Type", "native"),
        ];

        let json_body = json!({ "refresh": token });

        let response = client
            .post_json(&refresh_url, headers, &json_body)
            .await
            .map_err(|e| {
                // Network errors - DON'T clear tokens
                println!("Network error during token refresh: {}", e);
                OAuthError::HttpError(e)
            })?;

        // Auth failures from refresh endpoint - tokens are invalid
        if response.status == 401 || response.status == 403 || response.status == 400 {
            println!(
                "Auth failure (HTTP {}) - refresh token is invalid, clearing tokens",
                response.status
            );
            // Clear tokens from keyring on definitive auth failures
            let _ = TokenStorage::clear_all_tokens();
            let _ = crate::features::jwt_features::clear_jwt_token();
            return Err(OAuthError::AuthenticationFailed(response.status));
        }

        // Server errors (5xx) - DON'T clear tokens, backend might be down
        if response.status >= 500 {
            println!(
                "Server error (HTTP {}) - preserving tokens, backend may be down",
                response.status
            );
            return Err(OAuthError::AuthenticationFailed(response.status));
        }

        // Other 4xx errors
        if response.status >= 400 {
            println!("Client error (HTTP {}) during refresh", response.status);
            return Err(OAuthError::AuthenticationFailed(response.status));
        }

        let json_response = serde_json::from_str::<serde_json::Value>(&response.body)
            .map_err(|e| OAuthError::ParseError(format!("Failed to parse response JSON: {}", e)))?;

        // Try with underscores first (standardized), fallback to without underscores
        let access_token = json_response
            .get("access_token")
            .or_else(|| json_response.get("access"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                OAuthError::ParseError("Access token not found in response".to_string())
            })?;

        let refresh_token = json_response
            .get("refresh_token")
            .or_else(|| json_response.get("refresh"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                OAuthError::ParseError("Refresh token not found in response".to_string())
            })?;

        // Save new tokens
        TokenStorage::set_refresh_token(refresh_token)?;
        TokenStorage::set_access_token(access_token)?;

        // Update JWT token in the feature system
        if let Err(e) = crate::features::jwt_features::update_jwt_token(access_token) {
            println!("Warning: Failed to update JWT token: {}", e);
        }

        Ok(access_token.to_string())
    }

    /// Logout user by clearing tokens
    pub fn logout_user() -> Result<(), OAuthError> {
        // Clear JWT token from feature system
        if let Err(e) = crate::features::jwt_features::clear_jwt_token() {
            println!("Warning: Failed to clear JWT token: {}", e);
        }

        // Clear stored tokens
        TokenStorage::clear_all_tokens()?;

        println!("User logged out successfully!");
        Ok(())
    }

    // Convenience methods for backward compatibility using reqwest client

    /// Exchange authorization code for tokens using default reqwest client
    pub async fn exchange_auth_code_with_reqwest(
        api_url: &str,
        code: &str,
        login_id: &str,
    ) -> Result<OAuthResult, OAuthError> {
        let client = ReqwestClient::new().map_err(|e| OAuthError::HttpError(e))?;
        Self::exchange_auth_code(&client, api_url, code, login_id).await
    }

    /// Refresh access token using default reqwest client
    pub async fn refresh_access_token_with_reqwest(api_url: &str) -> Result<String, OAuthError> {
        let client = ReqwestClient::new().map_err(|e| OAuthError::HttpError(e))?;
        Self::refresh_access_token(&client, api_url).await
    }
}
