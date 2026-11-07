use reqwest::Method;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

use super::client::HttpClient;
use super::error::ApiResult;
use super::response::ApiResponse;

/// Fluent request builder for easy API calls
pub struct RequestBuilder {
    client: HttpClient,
    method: Method,
    url: String,
    headers: HashMap<String, String>,
    body: Option<Value>,
    timeout: Option<Duration>,
    protect_credentials: bool,
}

impl RequestBuilder {
    /// Create a new request builder
    pub fn new(method: Method, url: impl Into<String>) -> Self {
        Self {
            client: (*HttpClient::global()).clone(),
            method,
            url: url.into(),
            headers: HashMap::new(),
            body: None,
            timeout: None,
            protect_credentials: false,
        }
    }

    /// Add a single header
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Add multiple headers
    pub fn headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers.extend(headers);
        self
    }

    /// Add Bearer token authentication
    pub fn bearer_token(self, token: impl Into<String>) -> Self {
        self.header("Authorization", format!("Bearer {}", token.into()))
    }

    /// Add API key header
    pub fn api_key(self, key: impl Into<String>) -> Self {
        self.header("X-API-Key", key.into())
    }

    /// Add custom authorization header
    pub fn auth(self, auth: impl Into<String>) -> Self {
        self.header("Authorization", auth.into())
    }

    /// Add JSON body
    pub fn json(mut self, body: Value) -> Self {
        self.body = Some(body);
        self.header("Content-Type", "application/json")
    }

    /// Set custom timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set timeout in milliseconds
    pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout = Some(Duration::from_millis(timeout_ms));
        self
    }

    /// Execute the request
    pub(crate) fn protect_credentials(mut self, enabled: bool) -> Self {
        self.protect_credentials = enabled;
        self
    }

    pub async fn send(self) -> ApiResult<ApiResponse> {
        let protected = self.protect_credentials
            || self
                .headers
                .keys()
                .any(|name| super::credentials::requires_protection(name));
        self.client
            .request_with_policy(
                self.method,
                &self.url,
                Some(self.headers),
                self.body,
                self.timeout,
                protected,
            )
            .await
    }
}

/// Convenience functions for creating requests

/// Create a GET request
pub fn get(url: impl Into<String>) -> RequestBuilder {
    RequestBuilder::new(Method::GET, url)
}

/// Create a POST request
pub fn post(url: impl Into<String>) -> RequestBuilder {
    RequestBuilder::new(Method::POST, url)
}

/// Create a PUT request
pub fn put(url: impl Into<String>) -> RequestBuilder {
    RequestBuilder::new(Method::PUT, url)
}

/// Create a DELETE request
pub fn delete(url: impl Into<String>) -> RequestBuilder {
    RequestBuilder::new(Method::DELETE, url)
}

/// Create a PATCH request
pub fn patch(url: impl Into<String>) -> RequestBuilder {
    RequestBuilder::new(Method::PATCH, url)
}
