///! Webhook types and enums for Phase 1 features
///! Based on n8n architecture patterns
use serde::{Deserialize, Serialize};

/// Response mode determines how webhook responds to requests
/// Based on n8n's response mode patterns
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResponseMode {
    /// Respond immediately with 200 OK (fastest, default)
    /// Use for: Async webhooks, fire-and-forget
    OnReceived,

    /// Wait for workflow to complete, return last node's output
    /// Use for: Synchronous APIs, need result data
    LastNode,

    /// Use a dedicated "Respond to Webhook" node for custom response
    /// Use for: Complex response logic, conditional responses
    ResponseNode,

    /// Stream response during workflow execution (AI agents)
    /// Use for: Long-running LLM responses, real-time data
    Streaming,
}

impl ResponseMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OnReceived => "onReceived",
            Self::LastNode => "lastNode",
            Self::ResponseNode => "responseNode",
            Self::Streaming => "streaming",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "onReceived" => Ok(Self::OnReceived),
            "lastNode" => Ok(Self::LastNode),
            "responseNode" => Ok(Self::ResponseNode),
            "streaming" => Ok(Self::Streaming),
            _ => Err(format!("Invalid response mode: {}", s)),
        }
    }
}

impl Default for ResponseMode {
    fn default() -> Self {
        Self::OnReceived
    }
}

/// Deserialize ResponseMode from string (for database storage)
pub fn deserialize_response_mode(s: &str) -> ResponseMode {
    ResponseMode::from_str(s).unwrap_or_default()
}

/// Authentication configuration for webhooks
/// Based on n8n's authentication methods
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AuthConfig {
    /// No authentication (public webhook)
    None,

    /// Basic HTTP authentication
    BasicAuth {
        username: String,
        password_hash: String, // bcrypt hash
    },

    /// Header-based authentication
    HeaderAuth {
        header_name: String,
        expected_value: String, // Can be hashed token
    },

    /// Bearer token authentication
    BearerToken {
        token_hash: String, // bcrypt hash
    },

    /// IP address whitelist
    IpWhitelist {
        allowed_ips: Vec<String>, // ["192.168.1.1", "10.0.0.0/24"]
    },
}

impl AuthConfig {
    /// Serialize to JSON string for database storage
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| format!("Failed to serialize auth config: {}", e))
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("Failed to deserialize auth config: {}", e))
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self::None
    }
}

/// HTTP methods supported by webhooks
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
    HEAD,
}

impl HttpMethod {
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

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_uppercase().as_str() {
            "GET" => Ok(Self::GET),
            "POST" => Ok(Self::POST),
            "PUT" => Ok(Self::PUT),
            "PATCH" => Ok(Self::PATCH),
            "DELETE" => Ok(Self::DELETE),
            "HEAD" => Ok(Self::HEAD),
            _ => Err(format!("Invalid HTTP method: {}", s)),
        }
    }
}

/// Helper to serialize/deserialize Vec<HttpMethod> to/from JSON
pub fn serialize_http_methods(methods: &[HttpMethod]) -> Result<String, String> {
    serde_json::to_string(methods).map_err(|e| format!("Failed to serialize HTTP methods: {}", e))
}

pub fn deserialize_http_methods(json: &str) -> Result<Vec<HttpMethod>, String> {
    serde_json::from_str(json).map_err(|e| format!("Failed to deserialize HTTP methods: {}", e))
}

/// Validate and normalize response mode string
pub fn validate_response_mode(mode: Option<&str>) -> Result<String, String> {
    let mode_str = mode.unwrap_or("onReceived");
    // This will return an error if the mode is invalid
    let validated = ResponseMode::from_str(mode_str)?;
    Ok(validated.as_str().to_string())
}

/// Validate auth config JSON string
pub fn validate_auth_config(json: Option<&str>) -> Result<Option<String>, String> {
    match json {
        Some(config_str) => {
            // Try to parse it to ensure it's valid
            AuthConfig::from_json(config_str)?;
            Ok(Some(config_str.to_string()))
        }
        None => Ok(None),
    }
}

/// Validate and normalize HTTP methods JSON string
pub fn validate_http_methods(json: Option<&str>) -> Result<String, String> {
    let json_str = json.unwrap_or(r#"["POST"]"#);

    // Try to parse the JSON to ensure it's valid
    let methods = deserialize_http_methods(json_str)?;

    // Ensure at least one method is specified
    if methods.is_empty() {
        return Err("At least one HTTP method must be allowed".to_string());
    }

    // Re-serialize to ensure consistent formatting
    serialize_http_methods(&methods)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_mode_serialization() {
        let mode = ResponseMode::OnReceived;
        assert_eq!(mode.as_str(), "onReceived");

        let parsed = ResponseMode::from_str("lastNode").unwrap();
        assert_eq!(parsed, ResponseMode::LastNode);
    }

    #[test]
    fn test_auth_config_json() {
        let auth = AuthConfig::BasicAuth {
            username: "admin".to_string(),
            password_hash: "$2b$12$...".to_string(),
        };

        let json = auth.to_json().unwrap();
        let parsed = AuthConfig::from_json(&json).unwrap();

        match parsed {
            AuthConfig::BasicAuth { username, .. } => {
                assert_eq!(username, "admin");
            }
            _ => panic!("Wrong auth type"),
        }
    }

    #[test]
    fn test_http_methods() {
        let methods = vec![HttpMethod::GET, HttpMethod::POST];
        let json = serialize_http_methods(&methods).unwrap();
        let parsed = deserialize_http_methods(&json).unwrap();

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0], HttpMethod::GET);
    }
}
