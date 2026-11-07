/// Command implementations for the streaming service
///
/// These are Tauri-independent function implementations that provide
/// the core streaming functionality. The Tauri layer will wrap these
/// with #[tauri::command] macros.
use super::{ChatMessage, MessageStore, SessionManager, StreamSession, StreamingWebSocket};
use crate::db::{db::DbPool, Database};
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared streaming service state
#[derive(Clone)]
pub struct StreamingState {
    pub message_store: Arc<MessageStore>,
    pub session_manager: Arc<SessionManager>,
    pub websocket: Arc<RwLock<Option<StreamingWebSocket>>>,
    pub event_bus: Option<Arc<crate::events::EventBus>>,
    pub workflow_ingress:
        Option<crate::services::flow::event_router::DurableStreamEventIngressState>,
}

impl StreamingState {
    pub fn new() -> Self {
        // DEPRECATED: Use with_db() instead
        panic!("StreamingState::new() is deprecated. Use StreamingState::with_db() instead.");
    }

    pub fn with_db(db_pool: Arc<DbPool>) -> Self {
        let database = Arc::new(Database::new(&db_pool));

        Self {
            message_store: Arc::new(MessageStore::new(database.clone())),
            session_manager: Arc::new(SessionManager::with_db(db_pool)),
            websocket: Arc::new(RwLock::new(None)),
            event_bus: None,
            workflow_ingress: None,
        }
    }

    pub fn with_event_bus(mut self, event_bus: Arc<crate::events::EventBus>) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    pub fn with_workflow_ingress(
        mut self,
        ingress: crate::services::flow::event_router::DurableStreamEventIngressState,
    ) -> Self {
        self.workflow_ingress = Some(ingress);
        self
    }

    /// Load sessions from database on startup
    pub async fn load_sessions(&self) -> Result<usize, String> {
        self.session_manager
            .load_from_db()
            .await
            .map_err(|e| format!("Failed to load sessions: {}", e))
    }

    pub async fn is_connected(&self) -> bool {
        self.websocket
            .read()
            .await
            .as_ref()
            .is_some_and(StreamingWebSocket::is_connected)
    }

    /// Provider-neutral transport boundary used by streaming capabilities.
    /// A successful return means the local relay channel accepted the payload;
    /// it does not claim that a remote platform applied it.
    pub async fn send_payload(&self, payload: serde_json::Value) -> Result<(), String> {
        let websocket = self.websocket.read().await;
        let websocket = websocket
            .as_ref()
            .ok_or_else(|| "Streaming relay is not connected".to_string())?;
        if !websocket.is_connected() {
            return Err("Streaming relay is reconnecting or offline".to_string());
        }
        websocket
            .send(payload.to_string())
            .map_err(|error| error.to_string())
    }
}

impl Default for StreamingState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// WebSocket Commands
// ============================================================================

/// Connect to the streaming WebSocket
pub async fn streaming_connect(
    url: String,
    access_token: Option<String>,
    state: &StreamingState,
) -> Result<(), String> {
    let mut ws = StreamingWebSocket::new(
        state.message_store.clone(),
        state.session_manager.clone(),
        url,
        access_token,
    );

    if let Some(event_bus) = &state.event_bus {
        ws = ws.with_event_bus(event_bus.clone());
    }
    if let Some(ingress) = &state.workflow_ingress {
        ws = ws.with_workflow_ingress(ingress.clone());
    }

    ws.connect().await.map_err(|e| e.to_string())?;

    // Store the WebSocket instance
    *state.websocket.write().await = Some(ws);

    Ok(())
}

/// Disconnect from the streaming WebSocket
pub async fn streaming_disconnect(state: &StreamingState) -> Result<(), String> {
    let mut ws_lock = state.websocket.write().await;
    if let Some(mut ws) = ws_lock.take() {
        ws.disconnect().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Update WebSocket access token
pub async fn streaming_update_token(token: String, state: &StreamingState) -> Result<(), String> {
    let ws_lock = state.websocket.read().await;
    if let Some(ws) = ws_lock.as_ref() {
        ws.update_token(token).map_err(|e| e.to_string())?;
    } else {
        return Err("WebSocket not connected".to_string());
    }
    Ok(())
}

// ============================================================================
// Message Store Commands
// ============================================================================

/// Get chat messages for a specific account
pub async fn streaming_get_messages(
    account_id: u64,
    limit: Option<usize>,
    state: &StreamingState,
) -> Result<Vec<ChatMessage>, String> {
    state
        .message_store
        .get_messages(account_id, limit)
        .await
        .map_err(|e| e.to_string())
}

/// Get chat messages for multiple accounts (for multi-platform sessions)
pub async fn streaming_get_messages_for_accounts(
    account_ids: Vec<u64>,
    limit: Option<usize>,
    state: &StreamingState,
) -> Result<Vec<ChatMessage>, String> {
    state
        .message_store
        .get_messages_for_accounts(account_ids, limit)
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// Session Manager Commands
// ============================================================================

/// Get all sessions
pub async fn streaming_get_all_sessions(
    state: &StreamingState,
) -> Result<Vec<Arc<StreamSession>>, String> {
    state
        .session_manager
        .get_all_sessions()
        .await
        .map_err(|e| e.to_string())
}

/// Get session by ID
pub async fn streaming_get_session(
    session_id: String,
    state: &StreamingState,
) -> Result<Arc<StreamSession>, String> {
    state
        .session_manager
        .get_session(&session_id)
        .await
        .map_err(|e| e.to_string())
}

/// Create a new session
pub async fn streaming_create_session(
    session: StreamSession,
    state: &StreamingState,
) -> Result<Arc<StreamSession>, String> {
    state
        .session_manager
        .create_session(session)
        .await
        .map_err(|e| e.to_string())
}

/// Update existing session
pub async fn streaming_update_session(
    session_id: String,
    updates: HashMap<String, serde_json::Value>,
    state: &StreamingState,
) -> Result<Arc<StreamSession>, String> {
    state
        .session_manager
        .update_session(&session_id, updates)
        .await
        .map_err(|e| e.to_string())
}

/// Delete session
pub async fn streaming_delete_session(
    session_id: String,
    state: &StreamingState,
) -> Result<(), String> {
    state
        .session_manager
        .delete_session(&session_id)
        .await
        .map_err(|e| e.to_string())
}

/// Activate session (move to waiting_for_live state)
pub async fn streaming_activate_session(
    session_id: String,
    state: &StreamingState,
) -> Result<Arc<StreamSession>, String> {
    state
        .session_manager
        .activate_session(&session_id)
        .await
        .map_err(|e| e.to_string())
}

/// Start session (move to active state when stream goes live)
pub async fn streaming_start_session(
    session_id: String,
    state: &StreamingState,
) -> Result<Arc<StreamSession>, String> {
    state
        .session_manager
        .start_session(&session_id)
        .await
        .map_err(|e| e.to_string())
}

/// Pause session
pub async fn streaming_pause_session(
    session_id: String,
    state: &StreamingState,
) -> Result<Arc<StreamSession>, String> {
    state
        .session_manager
        .pause_session(&session_id)
        .await
        .map_err(|e| e.to_string())
}

/// Resume session from pause
pub async fn streaming_resume_session(
    session_id: String,
    state: &StreamingState,
) -> Result<Arc<StreamSession>, String> {
    state
        .session_manager
        .resume_session(&session_id)
        .await
        .map_err(|e| e.to_string())
}

/// End session
pub async fn streaming_end_session(
    session_id: String,
    auto: bool,
    state: &StreamingState,
) -> Result<Arc<StreamSession>, String> {
    state
        .session_manager
        .end_session(&session_id, auto)
        .await
        .map_err(|e| e.to_string())
}

/// Check for session conflicts
pub async fn streaming_check_conflicts(
    platforms: Vec<super::PlatformStream>,
    state: &StreamingState,
) -> Result<Vec<Arc<StreamSession>>, String> {
    state
        .session_manager
        .check_session_conflicts(&platforms)
        .await
        .map_err(|e| e.to_string())
}

/// Check auto-resume for a platform+account
pub async fn streaming_check_auto_resume(
    platform: String,
    account_id: String,
    state: &StreamingState,
) -> Result<Option<Arc<StreamSession>>, String> {
    state
        .session_manager
        .check_auto_resume(&platform, &account_id)
        .await
        .map_err(|e| e.to_string())
}

/// Find session by account_id
pub async fn streaming_find_session_by_account(
    platform: String,
    account_id: String,
    state: &StreamingState,
) -> Result<Option<Arc<StreamSession>>, String> {
    state
        .session_manager
        .find_session_by_account_id(&platform, &account_id)
        .await
        .map_err(|e| e.to_string())
}

/// Check if a session has any live streams
pub async fn streaming_is_session_live(
    session_id: String,
    state: &StreamingState,
) -> Result<bool, String> {
    state
        .session_manager
        .is_session_live(&session_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get all account IDs from a session
pub async fn streaming_get_session_account_ids(
    session_id: String,
    state: &StreamingState,
) -> Result<Vec<String>, String> {
    state
        .session_manager
        .get_session_account_ids(&session_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get active sessions
pub async fn streaming_get_active_sessions(
    state: &StreamingState,
) -> Result<Vec<Arc<StreamSession>>, String> {
    state
        .session_manager
        .get_active_sessions()
        .await
        .map_err(|e| e.to_string())
}

/// Get draft sessions
pub async fn streaming_get_draft_sessions(
    state: &StreamingState,
) -> Result<Vec<Arc<StreamSession>>, String> {
    state
        .session_manager
        .get_draft_sessions()
        .await
        .map_err(|e| e.to_string())
}

/// Get recent ended sessions
pub async fn streaming_get_recent_sessions(
    limit: usize,
    state: &StreamingState,
) -> Result<Vec<Arc<StreamSession>>, String> {
    state
        .session_manager
        .get_recent_sessions(limit)
        .await
        .map_err(|e| e.to_string())
}
