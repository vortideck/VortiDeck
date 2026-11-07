/// Error types for the streaming service
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StreamingError {
    // WebSocket errors
    #[error("WebSocket connection failed: {0}")]
    ConnectionFailed(String),

    #[error("WebSocket already connected")]
    AlreadyConnected,

    #[error("WebSocket not connected")]
    NotConnected,

    #[error("Failed to send WebSocket command: {0}")]
    CommandSendFailed(String),

    #[error("WebSocket message parsing failed: {0}")]
    MessageParseFailed(String),

    // Message store errors
    #[error("Failed to store message: {0}")]
    StorageError(String),

    #[error("Message store lock poisoned: {0}")]
    LockPoisoned(String),

    #[error("Account {0} has too many messages (limit: {1})")]
    TooManyMessages(u64, usize),

    #[error("Database error: {0}")]
    DatabaseError(String),

    // Session errors
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Session conflict: account already has an active session")]
    SessionConflict,

    #[error("Invalid session status transition from {0} to {1}")]
    InvalidStatusTransition(String, String),

    #[error("Failed to serialize session data: {0}")]
    SerializationFailed(String),

    #[error("Failed to deserialize session data: {0}")]
    DeserializationFailed(String),

    // General errors
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Type alias for streaming results
pub type StreamingResult<T> = Result<T, StreamingError>;

/// Convert StreamingError to String (for Tauri commands)
impl From<StreamingError> for String {
    fn from(err: StreamingError) -> String {
        err.to_string()
    }
}
