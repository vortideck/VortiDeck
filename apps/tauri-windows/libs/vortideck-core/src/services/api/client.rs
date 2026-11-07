use once_cell::sync::Lazy;
use reqwest::{Client, Method};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use super::error::{ApiError, ApiResult};
use super::response::ApiResponse;

/// Global HTTP client singleton with connection pooling
pub static GLOBAL_HTTP_CLIENT: Lazy<Arc<HttpClient>> = Lazy::new(|| Arc::new(HttpClient::new()));

/// Generic HTTP client with connection pooling
#[derive(Clone)]
pub struct HttpClient {
    client: Client,
    credential_client: Client,
}

impl HttpClient {
    /// Create a new HTTP client with optimized settings
    pub fn new() -> Self {
        let client = Client::builder()
            .pool_max_idle_per_host(10) // Reuse connections
            .pool_idle_timeout(Duration::from_secs(90))
            .timeout(Duration::from_secs(30)) // Default timeout
            .cookie_store(false) // No cookies needed
            .gzip(true) // Compression
            .build()
            .expect("Failed to build HTTP client");

        let credential_client = Client::builder()
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .timeout(Duration::from_secs(30))
            .cookie_store(false)
            .gzip(true)
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= 10 {
                    return attempt.error("Too many redirects");
                }
                if attempt
                    .previous()
                    .first()
                    .is_some_and(|original| original.origin() != attempt.url().origin())
                {
                    return attempt
                        .error("Credential-bearing request cannot redirect to another origin");
                }
                attempt.follow()
            }))
            .build()
            .expect("Failed to build credential HTTP client");
        Self {
            client,
            credential_client,
        }
    }

    /// Get the global singleton instance
    pub fn global() -> Arc<HttpClient> {
        GLOBAL_HTTP_CLIENT.clone()
    }

    /// Generic HTTP request
    pub async fn request(
        &self,
        method: Method,
        url: &str,
        headers: Option<HashMap<String, String>>,
        body: Option<Value>,
        timeout: Option<Duration>,
    ) -> ApiResult<ApiResponse> {
        let protected = headers.as_ref().is_some_and(|headers| {
            headers
                .keys()
                .any(|name| super::credentials::requires_protection(name))
        });
        self.request_with_policy(method, url, headers, body, timeout, protected)
            .await
    }

    pub(crate) async fn request_with_policy(
        &self,
        method: Method,
        url: &str,
        headers: Option<HashMap<String, String>>,
        body: Option<Value>,
        timeout: Option<Duration>,
        protect_credentials: bool,
    ) -> ApiResult<ApiResponse> {
        let client = if protect_credentials {
            &self.credential_client
        } else {
            &self.client
        };
        let mut request = client.request(method, url);

        // Apply custom timeout if provided
        if let Some(timeout) = timeout {
            request = request.timeout(timeout);
        }

        // Add headers
        if let Some(headers) = headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }

        // Add body if provided
        if let Some(body) = body {
            request = request.json(&body);
        }

        // Execute request with timing
        let start = std::time::Instant::now();
        let response = request.send().await.map_err(|e| {
            if e.is_timeout() {
                ApiError::Timeout
            } else if e.is_connect() {
                ApiError::ConnectionFailed(e.without_url().to_string())
            } else {
                ApiError::HttpError(e.without_url().to_string())
            }
        })?;

        let elapsed_ms = start.elapsed().as_millis() as u64;

        // Convert to ApiResponse
        let status = response.status().as_u16();
        let headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        let body = response.text().await.map_err(|e| {
            ApiError::HttpError(format!("Failed to read response: {}", e.without_url()))
        })?;

        Ok(ApiResponse {
            status,
            headers,
            body,
            elapsed_ms,
        })
    }

    /// Convenience method for GET requests
    pub async fn get(
        &self,
        url: &str,
        headers: Option<HashMap<String, String>>,
    ) -> ApiResult<ApiResponse> {
        self.request(Method::GET, url, headers, None, None).await
    }

    /// Convenience method for POST requests
    pub async fn post(
        &self,
        url: &str,
        headers: Option<HashMap<String, String>>,
        body: Value,
    ) -> ApiResult<ApiResponse> {
        self.request(Method::POST, url, headers, Some(body), None)
            .await
    }

    /// Convenience method for PUT requests
    pub async fn put(
        &self,
        url: &str,
        headers: Option<HashMap<String, String>>,
        body: Value,
    ) -> ApiResult<ApiResponse> {
        self.request(Method::PUT, url, headers, Some(body), None)
            .await
    }

    /// Convenience method for DELETE requests
    pub async fn delete(
        &self,
        url: &str,
        headers: Option<HashMap<String, String>>,
    ) -> ApiResult<ApiResponse> {
        self.request(Method::DELETE, url, headers, None, None).await
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}
