pub mod api;
pub mod auth;
pub mod execution;
pub mod flow;
pub mod icons;
pub mod macros;
pub mod marketplace;
pub mod mcp;
pub mod overlay_authoring;
pub mod overlay_portability;
pub mod streaming;
pub mod system;
pub mod themes;
pub mod ui;
pub mod webhook;
pub mod workspace;

// API module exports
pub use api::{
    delete, get, patch, post, put, send_event, send_webhook, send_webhook_async, ApiError,
    ApiResponse, ApiResult, HttpClient, RequestBuilder,
};

// pub use auth::AuthEngine; // AuthEngine is now internal to auth module
pub use execution::{
    ActionInvocationService, ExecutionContext, ExecutionResult, InvocationResult, InvocationStatus,
    UniversalAction, UniversalExecutionEngine,
};
pub use flow::{FlowAction, FlowState};
pub use icons::{IconProcessor, IconService, IconStorage};
pub use macros::MacroEngine;
pub use marketplace::{MarketplaceError, MarketplacePlugin, PluginMarketplace};
pub use streaming::{MessageStore, SessionManager, StreamingState, StreamingWebSocket};
#[cfg(target_os = "windows")]
pub use system::SystemControl;
pub use themes::{Theme, ThemeError, ThemeManager};
pub use ui::{ButtonConfig, ButtonError, ButtonManager};
pub use workspace::WorkspaceConfig;
// TODO: Uncomment when webhook services are implemented
// pub use webhook::{WebhookService, WebhookManager, WebhookExecutor, WebhookValidator, WebhookServiceConfig};

#[cfg(feature = "icons-server")]
pub use icons::IconServer;
