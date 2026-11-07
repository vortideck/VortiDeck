//! Platform-agnostic HTTP types for webhook handling
//! These types allow the core library to handle webhooks without depending on any HTTP framework

use serde_json::Value;
use std::collections::HashMap;

/// Platform-agnostic HTTP request representation
#[derive(Debug, Clone)]
pub struct WebhookRequest {
    /// Webhook ID from URL path
    pub webhook_id: String,

    /// HTTP method
    pub method: HttpMethod,

    /// HTTP headers (lowercase keys)
    pub headers: HashMap<String, String>,

    /// Request body as raw bytes
    pub body: Vec<u8>,

    /// Remote IP address (if available)
    pub remote_ip: Option<String>,
}

/// Platform-agnostic HTTP response representation
#[derive(Debug, Clone)]
pub struct WebhookResponse {
    /// HTTP status code
    pub status_code: u16,

    /// Response headers
    pub headers: HashMap<String, String>,

    /// Response body as JSON
    pub body: Value,
}

impl WebhookResponse {
    /// Create success response
    pub fn ok(body: Value) -> Self {
        Self {
            status_code: 200,
            headers: HashMap::new(),
            body,
        }
    }

    /// Create error response
    pub fn error(status: u16, message: &str) -> Self {
        Self {
            status_code: status,
            headers: HashMap::new(),
            body: serde_json::json!({ "error": message }),
        }
    }

    /// Create response with custom status and body
    pub fn with_status(status: u16, body: Value) -> Self {
        Self {
            status_code: status,
            headers: HashMap::new(),
            body,
        }
    }

    /// Add header to response
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }
}

/// Platform-agnostic HTTP methods
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
    HEAD,
}

impl HttpMethod {
    /// Parse HTTP method from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Some(Self::GET),
            "POST" => Some(Self::POST),
            "PUT" => Some(Self::PUT),
            "PATCH" => Some(Self::PATCH),
            "DELETE" => Some(Self::DELETE),
            "HEAD" => Some(Self::HEAD),
            _ => None,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GET => "GET",
            Self::POST => "POST",
            Self::PUT => "PUT",
            Self::PATCH => "PATCH",
            Self::DELETE => "DELETE",
            Self::HEAD => "HEAD",
        }
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
