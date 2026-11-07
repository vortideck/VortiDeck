#![allow(warnings)]
pub mod adapters;
pub mod analytics;
pub mod broadcast_helper;
pub mod client_registry;
pub mod commands; // Made public so main.rs can use it
pub mod crypto;
pub mod db;
pub mod e2ee_handler;
pub mod marketplace;
pub mod overlay;
pub mod package_lifecycle;
pub mod relay_client;
pub mod relay_constants;
// command_handler moved to main.rs since it depends on execute_action_universal

// StateBridge removed - migrated to pure StateManager
pub mod utils;
pub mod widget_document_cache;
// pub mod auth; // ✅ MOVED to services/auth + commands/auth_commands
// pub mod system_control; // Moved to vortideck-core
