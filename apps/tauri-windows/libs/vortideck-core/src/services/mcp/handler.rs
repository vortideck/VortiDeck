//! VortiDeck MCP Client Handler with progress tracking support
//!
//! Implements RMCP's ClientHandler trait to receive progress notifications
//! from MCP servers during long-running tool executions.

use rmcp::model::{
    CancelledNotificationParam, ClientInfo, CreateMessageRequestParams, CreateMessageResult,
    ErrorData, ListRootsResult, LoggingMessageNotificationParam, ProgressNotificationParam,
    ResourceUpdatedNotificationParam,
};
use rmcp::service::{NotificationContext, RequestContext};
use rmcp::ClientHandler;
use rmcp::RoleClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Progress event sent to frontend via Tauri events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProgressEvent {
    /// Unique identifier for the operation (e.g., tool_call_id)
    pub progress_token: String,
    /// Current progress value (0.0 - 1.0 or actual count)
    pub progress: f64,
    /// Optional total/maximum value
    pub total: Option<f64>,
    /// Optional progress message from server
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum McpClientEvent {
    Progress(ToolProgressEvent),
    ToolListChanged { server_name: String },
}

pub type McpClientEventSender = mpsc::UnboundedSender<McpClientEvent>;
pub type McpClientEventReceiver = mpsc::UnboundedReceiver<McpClientEvent>;

pub fn client_event_channel() -> (McpClientEventSender, McpClientEventReceiver) {
    mpsc::unbounded_channel()
}

/// VortiDeck MCP Client Handler
///
/// Receives notifications from MCP servers including:
/// - Progress updates during long-running tool executions
/// - Logging messages from servers
/// - Resource/tool/prompt list change notifications
#[derive(Debug, Clone)]
pub struct VortiDeckClientHandler {
    server_name: Option<String>,
    event_tx: Option<McpClientEventSender>,
}

impl VortiDeckClientHandler {
    pub fn for_server(server_name: impl Into<String>, event_tx: McpClientEventSender) -> Self {
        Self {
            server_name: Some(server_name.into()),
            event_tx: Some(event_tx),
        }
    }

    /// Create a handler without progress forwarding
    pub fn without_progress() -> Self {
        Self {
            server_name: None,
            event_tx: None,
        }
    }

    /// Helper to send progress events to frontend
    fn send_progress(&self, event: ToolProgressEvent) {
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(McpClientEvent::Progress(event));
        }
    }
}

impl ClientHandler for VortiDeckClientHandler {
    /// Handle progress notifications from MCP servers
    ///
    /// This is called when a server sends progress updates during
    /// long-running operations like network scans, file processing, etc.
    async fn on_progress(
        &self,
        params: ProgressNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
        println!(
            "📊 MCP Progress: token={:?}, progress={}, total={:?}",
            params.progress_token, params.progress, params.total
        );

        // Forward to frontend via channel
        self.send_progress(ToolProgressEvent {
            progress_token: format!("{:?}", params.progress_token),
            progress: params.progress,
            total: params.total,
            message: None, // MCP spec doesn't include message in progress params
        });
    }

    /// Handle logging messages from MCP servers
    async fn on_logging_message(
        &self,
        params: LoggingMessageNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
        // Log server messages (optional: could also forward to frontend)
        println!("[MCP Server {:?}] {:?}", params.level, params.data);
    }

    /// Handle tool list change notifications
    async fn on_tool_list_changed(&self, _context: NotificationContext<RoleClient>) {
        if let (Some(server_name), Some(tx)) = (&self.server_name, &self.event_tx) {
            let _ = tx.send(McpClientEvent::ToolListChanged {
                server_name: server_name.clone(),
            });
        } else {
            tracing::warn!("MCP tool list changed without an owned server connection");
        }
    }

    /// Handle resource list change notifications
    async fn on_resource_list_changed(&self, _context: NotificationContext<RoleClient>) {
        println!("📦 MCP Server resource list changed");
    }

    /// Handle resource updated notifications
    async fn on_resource_updated(
        &self,
        _params: ResourceUpdatedNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
        println!("📦 MCP Resource updated");
    }

    /// Handle prompt list change notifications
    async fn on_prompt_list_changed(&self, _context: NotificationContext<RoleClient>) {
        println!("💬 MCP Server prompt list changed");
    }

    /// Handle cancellation notifications
    async fn on_cancelled(
        &self,
        params: CancelledNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
        println!("❌ MCP Operation cancelled: {:?}", params.reason);
    }

    // Default implementations for request handlers
    async fn ping(&self, _context: RequestContext<RoleClient>) -> Result<(), ErrorData> {
        Ok(())
    }

    async fn create_message(
        &self,
        _params: CreateMessageRequestParams,
        _context: RequestContext<RoleClient>,
    ) -> Result<CreateMessageResult, ErrorData> {
        Err(ErrorData::invalid_params(
            "create_message not supported",
            None,
        ))
    }

    async fn list_roots(
        &self,
        _context: RequestContext<RoleClient>,
    ) -> Result<ListRootsResult, ErrorData> {
        Ok(ListRootsResult::default())
    }
}
