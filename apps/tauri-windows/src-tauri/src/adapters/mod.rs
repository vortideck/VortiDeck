// 🚀 TAURI ADAPTERS: Direct platform implementations (no factory pattern)
// Simple, direct implementations without unnecessary abstractions

pub mod commands;
pub mod flow_commands;
pub mod flow_registry_adapter;
pub mod icons_commands;
pub mod macro_commands;
pub mod mcp_commands;
pub mod obs_commands;
pub mod tauri;
pub mod webhook_routes;

// Direct platform adapters (no factory pattern)
pub mod platform_adapters;
pub mod state_manager;
pub mod tauri_event_emitter;
pub mod volume_callback;

// Keep the old TauriEventEmitter for integrations (legacy)
pub use tauri::TauriEventEmitter as IntegrationEventEmitter;

pub use commands::register_commands;

// Export the direct platform adapter functions
pub use platform_adapters::{create_tauri_platform_context, spawn_event_bridge};
