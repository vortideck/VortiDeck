//! Session Manager - Actor-Based Implementation
//!
//! Manages ChatSession actors with lifecycle control.
//! Each session runs in its own tokio task.

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex as TokioMutex;
use tokio::task::JoinHandle;

use crate::services::mcp::{
    chat::ChatSession,
    client_pool::ClientPool,
    provider::AIProvider,
    session_actor::{spawn_session_actor, SessionHandle},
    tool::ToolSet,
};

// ============================================================================
// Session Context
// ============================================================================

/// Session context identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SessionContext {
    /// AI Assistant conversation (persistent)
    Conversation { conversation_id: String },
    /// Flow node execution (ephemeral)
    FlowNode {
        flow_instance_id: String,
        node_id: String,
    },
}

impl SessionContext {
    pub fn is_persistent(&self) -> bool {
        matches!(self, SessionContext::Conversation { .. })
    }

    pub fn key(&self) -> String {
        match self {
            SessionContext::Conversation { conversation_id } => format!("conv:{}", conversation_id),
            SessionContext::FlowNode {
                flow_instance_id,
                node_id,
            } => {
                format!("flow:{}:{}", flow_instance_id, node_id)
            }
        }
    }
}

// ============================================================================
// Managed Session (Actor Wrapper)
// ============================================================================

/// Wrapper around SessionHandle with lifecycle metadata
pub struct ManagedSession {
    pub handle: SessionHandle,
    pub context: SessionContext,
    pub created_at: Instant,
    task: JoinHandle<()>,
}

impl ManagedSession {
    pub fn new(handle: SessionHandle, task: JoinHandle<()>, context: SessionContext) -> Self {
        Self {
            handle,
            context,
            created_at: Instant::now(),
            task,
        }
    }

    /// Check if the actor is still running
    pub fn is_alive(&self) -> bool {
        self.handle.is_alive() && !self.task.is_finished()
    }

    /// Get session age
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Check if session is stale
    pub fn is_stale(&self, timeout: Duration) -> bool {
        // Use handle's age tracking
        self.handle.age() > timeout
    }
}

impl std::fmt::Debug for ManagedSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagedSession")
            .field("context", &self.context)
            .field("alive", &self.is_alive())
            .field("age", &self.age())
            .finish()
    }
}

// ============================================================================
// Session Manager
// ============================================================================

/// Manages lifecycle of conversation and ephemeral session actors
#[derive(Debug)]
pub struct SessionManager {
    sessions: Arc<TokioMutex<HashMap<String, ManagedSession>>>,
    client_pool: ClientPool,
    mcp_tools: Arc<TokioMutex<ToolSet>>,
    conversation_timeout: Duration,
    ephemeral_timeout: Duration,
}

impl SessionManager {
    pub fn new(client_pool: ClientPool, mcp_tools: Arc<TokioMutex<ToolSet>>) -> Self {
        Self {
            sessions: Arc::new(TokioMutex::new(HashMap::new())),
            client_pool,
            mcp_tools,
            conversation_timeout: Duration::from_secs(3600), // 1 hour
            ephemeral_timeout: Duration::from_secs(300),     // 5 minutes
        }
    }

    // ========================================================================
    // Session Creation
    // ========================================================================

    /// Get or create a conversation session actor
    /// Returns the session key for subsequent operations
    pub async fn get_or_create_conversation(
        &self,
        conversation_id: String,
        provider: AIProvider,
        model: Option<String>,
    ) -> Result<String, String> {
        let context = SessionContext::Conversation {
            conversation_id: conversation_id.clone(),
        };
        let key = context.key();

        // Check if session already exists and is alive
        {
            let sessions = self.sessions.lock().await;
            if let Some(managed) = sessions.get(&key) {
                if managed.is_alive() {
                    println!(
                        "♻️ Reusing existing conversation session: {}",
                        conversation_id
                    );
                    return Ok(key);
                }
            }
        }

        // Create new session actor
        println!(
            "🆕 Creating new conversation session actor: {}",
            conversation_id
        );

        let client = self.client_pool.get_or_create(&provider).await?;
        let mut chat_session = ChatSession::new(client, self.mcp_tools.clone(), provider);

        if let Some(model_name) = model {
            chat_session.set_model(model_name);
        }

        // Spawn actor
        let (handle, task) = spawn_session_actor(chat_session);
        let managed = ManagedSession::new(handle, task, context);

        let mut sessions = self.sessions.lock().await;
        sessions.insert(key.clone(), managed);

        println!(
            "✅ Session actor created. Total sessions: {}",
            sessions.len()
        );

        Ok(key)
    }

    /// Create an ephemeral session actor for flow node execution
    pub async fn create_ephemeral(
        &self,
        flow_instance_id: String,
        node_id: String,
        provider: AIProvider,
        model: Option<String>,
    ) -> Result<String, String> {
        let context = SessionContext::FlowNode {
            flow_instance_id: flow_instance_id.clone(),
            node_id: node_id.clone(),
        };
        let key = context.key();

        println!(
            "🆕 Creating ephemeral session actor: flow={}, node={}",
            flow_instance_id, node_id
        );

        let client = self.client_pool.get_or_create(&provider).await?;
        let mut chat_session = ChatSession::new(client, self.mcp_tools.clone(), provider);

        if let Some(model_name) = model {
            chat_session.set_model(model_name);
        }

        // Spawn actor
        let (handle, task) = spawn_session_actor(chat_session);
        let managed = ManagedSession::new(handle, task, context);

        let mut sessions = self.sessions.lock().await;
        sessions.insert(key.clone(), managed);

        println!(
            "✅ Ephemeral session actor created. Total sessions: {}",
            sessions.len()
        );

        Ok(key)
    }

    // ========================================================================
    // Session Access
    // ========================================================================

    /// Get a session handle by key
    /// Returns None if session doesn't exist or actor is dead
    pub async fn get(&self, key: &str) -> Option<SessionHandle> {
        let sessions = self.sessions.lock().await;
        sessions.get(key).and_then(|managed| {
            if managed.is_alive() {
                Some(managed.handle.clone())
            } else {
                println!("⚠️ Session {} exists but actor is dead", key);
                None
            }
        })
    }

    /// Get cancellation flag for a session
    pub async fn get_cancellation_flag(&self, key: &str) -> Option<Arc<AtomicBool>> {
        let sessions = self.sessions.lock().await;
        sessions
            .get(key)
            .map(|managed| managed.handle.cancellation_flag())
    }

    /// Cancel a session's current operation
    pub async fn cancel(&self, key: &str) -> Result<(), String> {
        let sessions = self.sessions.lock().await;
        if let Some(managed) = sessions.get(key) {
            managed.handle.cancel();
            println!("🛑 Cancelled session: {}", key);
            Ok(())
        } else {
            Err(format!("Session not found: {}", key))
        }
    }

    /// Reset cancellation flag for a session
    pub async fn reset_cancellation(&self, key: &str) {
        let sessions = self.sessions.lock().await;
        if let Some(managed) = sessions.get(key) {
            managed.handle.reset_cancellation();
        }
    }

    // ========================================================================
    // Session Removal
    // ========================================================================

    /// Remove an ephemeral session
    pub async fn remove_ephemeral(&self, flow_instance_id: &str, node_id: &str) {
        let context = SessionContext::FlowNode {
            flow_instance_id: flow_instance_id.to_string(),
            node_id: node_id.to_string(),
        };
        let key = context.key();

        let mut sessions = self.sessions.lock().await;
        if let Some(managed) = sessions.remove(&key) {
            // Shutdown the actor gracefully
            managed.handle.shutdown().await;
            println!(
                "🗑️ Removed ephemeral session: flow={}, node={}",
                flow_instance_id, node_id
            );
            println!("   Remaining sessions: {}", sessions.len());
        }
    }

    /// Remove a conversation session
    pub async fn remove_conversation(&self, conversation_id: &str) {
        let context = SessionContext::Conversation {
            conversation_id: conversation_id.to_string(),
        };
        let key = context.key();

        let mut sessions = self.sessions.lock().await;
        if let Some(managed) = sessions.remove(&key) {
            // Shutdown the actor gracefully
            managed.handle.shutdown().await;
            println!("🗑️ Removed conversation session: {}", conversation_id);
            println!("   Remaining sessions: {}", sessions.len());
        }
    }

    // ========================================================================
    // Lifecycle Management
    // ========================================================================

    /// Cleanup stale/dead sessions
    pub async fn cleanup_stale(&self) {
        let mut sessions = self.sessions.lock().await;
        let before_count = sessions.len();

        // Collect keys to remove
        let to_remove: Vec<String> = sessions
            .iter()
            .filter(|(_, managed)| {
                let timeout = if managed.context.is_persistent() {
                    self.conversation_timeout
                } else {
                    self.ephemeral_timeout
                };
                !managed.is_alive() || managed.is_stale(timeout)
            })
            .map(|(key, _)| key.clone())
            .collect();

        // Remove and shutdown
        for key in &to_remove {
            if let Some(managed) = sessions.remove(key) {
                println!(
                    "🧹 Removing stale session: {} (age: {:?})",
                    key,
                    managed.age()
                );
                managed.handle.shutdown().await;
            }
        }

        let removed = before_count - sessions.len();
        if removed > 0 {
            println!(
                "🧹 Cleanup complete: removed {} sessions, {} remaining",
                removed,
                sessions.len()
            );
        }
    }

    /// Get statistics
    pub async fn stats(&self) -> (usize, usize, usize) {
        let sessions = self.sessions.lock().await;
        let total = sessions.len();
        let conversations = sessions
            .values()
            .filter(|m| m.context.is_persistent())
            .count();
        let ephemeral = total - conversations;
        (total, conversations, ephemeral)
    }

    // ========================================================================
    // Compatibility Layer (for gradual migration)
    // ========================================================================

    /// Execute a sync operation with access to a session
    /// NOTE: This locks the session handle, sends a command, and waits for response.
    /// For new code, prefer using get() and calling handle methods directly.
    pub async fn with_session<F, R>(&self, key: &str, operation: F) -> Result<R, String>
    where
        F: FnOnce(&SessionHandle) -> Result<R, String>,
    {
        let handle = self
            .get(key)
            .await
            .ok_or_else(|| format!("Session not found: {}", key))?;
        operation(&handle)
    }
}

impl Clone for SessionManager {
    fn clone(&self) -> Self {
        Self {
            sessions: Arc::clone(&self.sessions),
            client_pool: self.client_pool.clone(),
            mcp_tools: Arc::clone(&self.mcp_tools),
            conversation_timeout: self.conversation_timeout,
            ephemeral_timeout: self.ephemeral_timeout,
        }
    }
}
