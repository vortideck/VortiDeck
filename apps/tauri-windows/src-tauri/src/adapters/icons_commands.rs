use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tokio::sync::RwLock;
use vortideck_core::services::{IconService, IconStorage};

#[cfg(feature = "icons-server")]
use vortideck_core::services::IconServer;

/// Shared state for icon service
pub struct IconServiceState {
    pub service: Arc<IconService>,
    #[cfg(feature = "icons-server")]
    pub server: Arc<RwLock<Option<IconServer>>>,
}

impl IconServiceState {
    pub fn new(app_handle: &AppHandle) -> Self {
        let app_data_dir = app_handle
            .path()
            .data_dir()
            .expect("Unable to find data dir");
        let storage_path = app_data_dir.join("VortiDeck");

        let service = Arc::new(IconService::new(storage_path));

        Self {
            service,
            #[cfg(feature = "icons-server")]
            server: Arc::new(RwLock::new(None)),
        }
    }
}

/// Save icon from Base64 data
#[tauri::command]
pub async fn save_icon(
    app_handle: tauri::AppHandle,
    icon_state: State<'_, IconServiceState>,
    button_id: &str,
    base64_image: &str,
) -> Result<(), String> {
    use crate::db::db::DbPool;
    use crate::db::Database;
    use std::sync::Arc;
    use tauri::Manager;
    use vortideck_core::core::StateManager;

    // Strip data URL prefix if present (e.g., "data:image/png;base64,")
    let clean_base64 = if base64_image.starts_with("data:") {
        base64_image.split(',').nth(1).unwrap_or(base64_image)
    } else {
        base64_image
    };

    // Save the icon
    icon_state
        .service
        .save_icon(button_id, clean_base64)
        .await?;

    // Broadcast updated deck to all connected mobile clients (with icons for relay mode)
    if let Ok(button_id_int) = button_id.parse::<i32>() {
        if let Some(db_pool) = app_handle.try_state::<DbPool>() {
            let db = Database::new(&db_pool);

            // Get button's deck_id
            if let Ok(Some(button)) = db.get_button_by_id(button_id_int).await {
                let deck_id = button.deck_id;

                // Get icons base path
                let icons_base_path = app_handle
                    .try_state::<IconServiceState>()
                    .map(|icon_state| icon_state.service.get_icons_base_dir());

                // Get deck with icons
                if let Some(deck) = db
                    .get_deck_with_buttons(deck_id, true, icons_base_path.as_deref())
                    .await
                {
                    // Broadcast to ALL clients (direct + relay)
                    if let Some(state_manager) = app_handle.try_state::<Arc<StateManager>>() {
                        let update_message = serde_json::json!({
                            "action": "deck_updated",
                            "deck_id": deck_id,
                            "deck": deck
                        });
                        crate::broadcast_helper::broadcast_to_all_clients(
                            &app_handle,
                            &state_manager,
                            update_message,
                        )
                        .await;
                        println!(
                            "📡 Icon saved for button {} and deck broadcasted to all clients",
                            button_id
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

/// Remove icon and its directory
#[tauri::command]
pub async fn remove_icon(
    icon_state: State<'_, IconServiceState>,
    button_id: &str,
) -> Result<(), String> {
    icon_state.service.remove_icon(button_id).await
}

/// Get icon path
#[tauri::command]
pub async fn get_icon_path(
    icon_state: State<'_, IconServiceState>,
    button_id: &str,
) -> Result<String, String> {
    let path = icon_state.service.get_icon_path(button_id);
    Ok(path.to_string_lossy().to_string())
}

/// Get storage statistics
#[tauri::command]
pub async fn get_icon_storage_stats(
    icon_state: State<'_, IconServiceState>,
) -> Result<serde_json::Value, String> {
    let stats = icon_state.service.get_storage_stats()?;
    Ok(serde_json::json!({
        "totalIcons": stats.total_icons,
        "totalSizeBytes": stats.total_size_bytes,
        "buttonIds": stats.button_ids
    }))
}

/// Start the icons HTTP server (desktop only)
#[cfg(feature = "icons-server")]
#[tauri::command]
pub async fn start_icons_server(
    icon_state: State<'_, IconServiceState>,
    ip: String,
) -> Result<u16, String> {
    use std::net::IpAddr;

    let ip_addr: IpAddr = ip.parse().map_err(|_| "Invalid IP address".to_string())?;

    let icons_base_dir = icon_state.service.get_icons_base_dir();
    let mut server = IconServer::new(icons_base_dir, ip_addr);

    let port = server.start_auto_port().await?;

    // Store server instance
    let mut server_guard = icon_state.server.write().await;
    *server_guard = Some(server);

    Ok(port)
}

/// Get icons server info
#[cfg(feature = "icons-server")]
#[tauri::command]
pub async fn get_icons_server_info(
    icon_state: State<'_, IconServiceState>,
) -> Result<Option<serde_json::Value>, String> {
    let server_guard = icon_state.server.read().await;

    if let Some(server) = server_guard.as_ref() {
        if let Some(port) = server.get_port() {
            return Ok(Some(serde_json::json!({
                "port": port,
                "url": server.get_server_url(),
                "running": server.is_running()
            })));
        }
    }

    Ok(None)
}

/// Read file and convert to base64
#[tauri::command]
pub async fn read_file_as_base64(file_path: String) -> Result<String, String> {
    use base64::{engine::general_purpose, Engine as _};
    use std::fs;

    tokio::task::spawn_blocking(move || {
        let file_bytes =
            fs::read(&file_path).map_err(|e| format!("Error reading file {}: {}", file_path, e))?;

        let base64_string = general_purpose::STANDARD.encode(&file_bytes);
        Ok(base64_string)
    })
    .await
    .map_err(|e| format!("Error executing task: {}", e))?
}
