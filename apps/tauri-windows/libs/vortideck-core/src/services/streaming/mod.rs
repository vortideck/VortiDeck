pub(crate) mod command_policy;
pub mod commands;
/// Streaming service module
///
/// Handles persistent WebSocket connection to backend streaming service,
/// manages chat messages, sessions, and auto-resume logic.
///
/// This replaces the TypeScript implementation to provide:
/// - Persistent connection across page navigation
/// - In-memory message storage with auto-cleanup
/// - Better performance and type safety
pub mod error;
pub mod message_store;
pub mod models;
pub mod session_manager;
pub mod websocket;

pub use commands::StreamingState;
pub use error::{StreamingError, StreamingResult};
pub use message_store::MessageStore;
pub use models::*;
pub use session_manager::SessionManager;
pub use websocket::StreamingWebSocket;
pub mod stream_values;
