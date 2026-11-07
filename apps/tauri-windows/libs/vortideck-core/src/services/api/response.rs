use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::error::{ApiError, ApiResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub elapsed_ms: u64,
}

impl ApiResponse {
    /// Parse body as JSON
    pub fn json<T: for<'de> Deserialize<'de>>(&self) -> ApiResult<T> {
        serde_json::from_str(&self.body)
            .map_err(|e| ApiError::ParseError(format!("Failed to parse JSON: {}", e)))
    }

    /// Parse body as generic JSON value
    pub fn json_value(&self) -> ApiResult<Value> {
        self.json()
    }

    /// Get body as text
    pub fn text(&self) -> &str {
        &self.body
    }

    /// Check if successful (2xx)
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Check if client error (4xx)
    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.status)
    }

    /// Check if server error (5xx)
    pub fn is_server_error(&self) -> bool {
        self.status >= 500
    }

    /// Get header value
    pub fn header(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }
}
