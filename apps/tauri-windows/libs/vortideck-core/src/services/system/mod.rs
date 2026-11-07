// src/commands/mod.rs

pub mod commands;
pub mod flow_adapter;
pub mod flow_traits;
pub mod system_control;
pub mod universal_commands;
pub mod window_management;

pub use commands::*; // This line simplifies access
pub use flow_adapter::SystemActionProvider; // Export the provider for the flow registry
#[cfg(target_os = "windows")]
pub use system_control::SystemControl; // This line simplifies access
pub use universal_commands::{
    create_universal_system_commands, execute_system_action_universal,
    get_available_actions_universal, get_system_info_universal, set_volume_universal,
    UniversalSystemCommands,
};
