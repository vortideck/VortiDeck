//! Auth Commands - Thin Tauri Wrappers
//!
//! This module contains Tauri command wrappers that delegate to the business logic in services.
//! These commands handle Tauri-specific concerns like events and app handles.

use async_trait::async_trait;
use tauri::{AppHandle, Emitter};
use vortideck_core::services::auth::oauth::{HttpClient, HttpResponse};
use vortideck_core::services::auth::{OAuthError, OAuthService, TokenStorage, TokenStorageError};

/// Tauri HTTP client implementation using tauri-plugin-http
pub struct TauriHttpClient {
    app_handle: AppHandle,
}

impl TauriHttpClient {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

#[async_trait]
impl HttpClient for TauriHttpClient {
    async fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        json_body: &serde_json::Value,
    ) -> Result<HttpResponse, String> {
        use tauri_plugin_http::reqwest;

        // Build the HTTP client using Tauri's reqwest
        let client = reqwest::ClientBuilder::new()
            .cookie_store(false)
            .build()
            .map_err(|e| format!("Failed to build Tauri HTTP client: {}", e))?;

        // Create the request
        let mut request = client.post(url);

        // Add headers
        for (key, value) in headers {
            request = request.header(*key, *value);
        }

        // Send the request with JSON body
        let response = request
            .json(json_body)
            .send()
            .await
            .map_err(|e| format!("Failed to send request: {}", e))?;

        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .map_err(|e| format!("Failed to get response text: {}", e))?;

        Ok(HttpResponse { status, body })
    }
}

/// Exchange authorization code for tokens
#[tauri::command]
pub async fn exchange_auth_code(
    code: String,
    login_id: String,
    app_handle: AppHandle,
) -> Result<(), String> {
    let api_url =
        std::env::var("API_URL").unwrap_or_else(|_| "https://backend.vortideck.com".to_string());
    let client = TauriHttpClient::new(app_handle.clone());
    match OAuthService::exchange_auth_code(&client, &api_url, &code, &login_id).await {
        Ok(result) => {
            // 📊 Track Google OAuth login
            crate::analytics::capture(
                &app_handle,
                crate::analytics::AnalyticsEvent::new("user_logged_in").prop("method", "google"),
            );

            // Emit success event (payload-less for security)
            if let Err(e) = app_handle.emit("auth_token_saved", ()) {
                eprintln!("Failed to emit auth_token_saved event: {}", e);
            }

            // Emit to central EventBus
            use tauri::Manager;
            if let Some(platform_context) =
                app_handle.try_state::<std::sync::Arc<vortideck_core::core::PlatformContext>>()
            {
                let _ =
                    platform_context
                        .event_bus
                        .emit(vortideck_core::events::EventPayload::Auth(
                            vortideck_core::events::AuthEventPayload::TokenSaved,
                        ));
            }

            Ok(())
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Get refresh token from secure storage
#[tauri::command]
pub fn get_refresh_token() -> Result<String, String> {
    TokenStorage::get_refresh_token().map_err(|e| e.to_string())
}

/// Get access token from secure storage
#[tauri::command]
pub fn get_access_token() -> Result<String, String> {
    TokenStorage::get_access_token().map_err(|e| e.to_string())
}

/// Save refresh token to secure storage and emit auth_token_saved event
#[tauri::command]
pub fn set_refresh_token(refresh_token: String, app_handle: AppHandle) -> Result<String, String> {
    match TokenStorage::set_refresh_token(&refresh_token) {
        Ok(_) => {
            // Emit event to trigger relay connection (and other listeners)
            if let Err(e) = app_handle.emit("auth_token_saved", ()) {
                eprintln!("Failed to emit auth_token_saved event: {}", e);
            }

            // Emit to central EventBus
            use tauri::Manager;
            if let Some(platform_context) =
                app_handle.try_state::<std::sync::Arc<vortideck_core::core::PlatformContext>>()
            {
                let _ =
                    platform_context
                        .event_bus
                        .emit(vortideck_core::events::EventPayload::Auth(
                            vortideck_core::events::AuthEventPayload::TokenSaved,
                        ));
            }

            Ok("Refresh token saved successfully".to_string())
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Refresh access token using stored refresh token
#[tauri::command]
pub async fn refresh_access_token(app_handle: AppHandle) -> Result<String, String> {
    let api_url =
        std::env::var("API_URL").unwrap_or_else(|_| "https://backend.vortideck.com".to_string());
    let client = TauriHttpClient::new(app_handle);
    OAuthService::refresh_access_token(&client, &api_url)
        .await
        .map_err(|e| e.to_string())
}

/// Logout user by clearing all tokens
#[tauri::command]
pub fn logout_user(app_handle: AppHandle) -> Result<(), String> {
    OAuthService::logout_user().map_err(|e| e.to_string())?;

    // Emit event to trigger relay connection teardown (and other listeners)
    let _ = app_handle.emit("auth_token_cleared", ());

    // Emit to central EventBus
    use tauri::Manager;
    if let Some(platform_context) =
        app_handle.try_state::<std::sync::Arc<vortideck_core::core::PlatformContext>>()
    {
        let _ = platform_context
            .event_bus
            .emit(vortideck_core::events::EventPayload::Auth(
                vortideck_core::events::AuthEventPayload::TokenCleared,
            ));
    }

    Ok(())
}

// Alternative versions using reqwest for backward compatibility if needed
/// Exchange authorization code for tokens using reqwest client
#[tauri::command]
pub async fn exchange_auth_code_with_reqwest(
    code: String,
    login_id: String,
    app_handle: AppHandle,
) -> Result<(), String> {
    let api_url =
        std::env::var("API_URL").unwrap_or_else(|_| "https://backend.vortideck.com".to_string());
    match OAuthService::exchange_auth_code_with_reqwest(&api_url, &code, &login_id).await {
        Ok(result) => {
            // 📊 Track Google OAuth login
            crate::analytics::capture(
                &app_handle,
                crate::analytics::AnalyticsEvent::new("user_logged_in").prop("method", "google"),
            );

            // Emit success event (payload-less for security)
            if let Err(e) = app_handle.emit("auth_token_saved", ()) {
                eprintln!("Failed to emit auth_token_saved event: {}", e);
            }

            // Emit to central EventBus
            use tauri::Manager;
            if let Some(platform_context) =
                app_handle.try_state::<std::sync::Arc<vortideck_core::core::PlatformContext>>()
            {
                let _ =
                    platform_context
                        .event_bus
                        .emit(vortideck_core::events::EventPayload::Auth(
                            vortideck_core::events::AuthEventPayload::TokenSaved,
                        ));
            }

            Ok(())
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Refresh access token using reqwest client
#[tauri::command]
pub async fn refresh_access_token_with_reqwest() -> Result<String, String> {
    let api_url =
        std::env::var("API_URL").unwrap_or_else(|_| "https://backend.vortideck.com".to_string());
    OAuthService::refresh_access_token_with_reqwest(&api_url)
        .await
        .map_err(|e| e.to_string())
}
