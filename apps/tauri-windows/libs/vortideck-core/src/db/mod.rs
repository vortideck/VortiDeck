// vortideck-core/src/db/mod.rs
// Platform-agnostic database core (schema, models, migrations)
// Tauri-specific utils are in the tauri app's db module

pub mod database; // New: Platform-agnostic database operations
pub mod db;
pub mod models;
pub mod schema;

pub use database::{Database, RemoteCommandClaim}; // Export canonical database boundary
pub use db::initialize_database;
pub use models::*;
pub use schema::*;
