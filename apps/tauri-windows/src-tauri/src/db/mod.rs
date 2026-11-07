// Tauri adapter for database
// Re-exports platform-agnostic database components from vortideck-core
// Plus Tauri-specific utils (commands, Database wrapper)

pub mod utils;

// Re-export from vortideck-core (platform-agnostic) - EXCEPT Database (we have our own wrapper)
pub use vortideck_core::db::database::DbPool;
pub use vortideck_core::db::models::MacroType; // Explicit re-export for easier access
pub use vortideck_core::db::{db, models, schema}; // Re-export DbPool for easier access

// Re-export Tauri-specific utils (includes Database wrapper)
pub use utils::*;
