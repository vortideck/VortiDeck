///! Webhook error types
///! Based on Svix's comprehensive error handling patterns
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WebhookError {
    // Signature verification errors
    #[error("Missing required header: {0}")]
    MissingHeader(String),

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Invalid timestamp format")]
    InvalidTimestamp,

    #[error("Timestamp too old: exceeded {0} second tolerance")]
    TimestampTooOld(i64),

    #[error("Timestamp is in the future")]
    FutureTimestamp,

    #[error("Invalid secret format: {0}")]
    InvalidSecret(String),

    // Webhook configuration errors
    #[error("Webhook not found: {0}")]
    NotFound(String),

    #[error("Webhook is disabled: {0}")]
    WebhookDisabled(String),

    #[error("Missing target URL for outgoing webhook")]
    MissingTargetUrl,

    #[error("Invalid webhook configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    // Payload errors
    #[error("Invalid payload format: {0}")]
    InvalidPayload(String),

    #[error("Payload too large: {0} bytes (max: {1} bytes)")]
    PayloadTooLarge(usize, usize),

    // Database errors
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Failed to serialize/deserialize: {0}")]
    SerializationError(String),

    // HTTP errors
    #[error("HTTP request failed: {0}")]
    HttpError(String),

    #[error("Webhook send failed: {0}")]
    SendFailed(String),

    #[error("Timeout after {0} seconds")]
    Timeout(u64),

    // Action execution errors
    #[error("Failed to parse action: {0}")]
    ActionParseError(String),

    #[error("Failed to execute action: {0}")]
    ActionExecutionError(String),

    // Rate limiting
    #[error("Rate limit exceeded: {0} requests per minute")]
    RateLimitExceeded(u32),

    // Generic error
    #[error("Webhook error: {0}")]
    Other(String),
}

impl From<diesel::result::Error> for WebhookError {
    fn from(err: diesel::result::Error) -> Self {
        match err {
            diesel::result::Error::NotFound => {
                WebhookError::NotFound("Webhook not found in database".to_string())
            }
            _ => WebhookError::DatabaseError(err.to_string()),
        }
    }
}

impl From<serde_json::Error> for WebhookError {
    fn from(err: serde_json::Error) -> Self {
        WebhookError::SerializationError(err.to_string())
    }
}

impl From<reqwest::Error> for WebhookError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            WebhookError::Timeout(30) // Default timeout
        } else {
            WebhookError::HttpError(err.to_string())
        }
    }
}

/// Result type for webhook operations
pub type WebhookResult<T> = Result<T, WebhookError>;
