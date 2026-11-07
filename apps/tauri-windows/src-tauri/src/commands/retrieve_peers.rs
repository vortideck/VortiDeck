use serde::Serialize;
use serde_json;
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Manager;
use tokio::sync::Mutex;
use vortideck_core::core::StateManager;

use crate::db::Database;
use std::collections::HashMap;
use vortideck_core::core::SerializablePeerInfo;
use vortideck_core::db::db::DbPool;

#[tauri::command]
pub async fn retrieve_peers(app_handle: AppHandle) -> Result<Vec<SerializablePeerInfo>, String> {
    // Access the database for stored peers
    let pool = match app_handle.try_state::<DbPool>() {
        Some(pool) => pool,
        None => {
            return Err("DbPool not available".to_string());
        }
    };
    let db = Database::new(&pool);
    let peers = db.get_all_peers().await;

    // Return the list of serialized peers directly
    Ok(peers)
}

#[tauri::command]
pub async fn retrieve_active_peers(
    app_handle: AppHandle,
) -> Result<Vec<SerializablePeerInfo>, String> {
    // Use StateManager directly to get active peers
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            return Err("StateManager not available".to_string());
        }
    };
    let core_peers = state_manager.get_peers().await;

    // Core and return types are now the same - no conversion needed
    let peers = core_peers;

    println!("🔄 Retrieved {} active peers via StateManager", peers.len());
    Ok(peers)
}
