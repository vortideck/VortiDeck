use std::net::IpAddr;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;
use tauri::WebviewWindow;

/// Return the one writable MCP configuration path. Older builds used AppData;
/// migrate that file once on platforms where AppData and AppConfig differ.
pub fn ensure_mcp_config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let canonical = app
        .path()
        .resolve(
            "resources/mcp/config.json",
            tauri::path::BaseDirectory::AppConfig,
        )
        .map_err(|error| format!("Failed to resolve MCP config path: {error}"))?;
    if canonical.exists() {
        return Ok(canonical);
    }

    let legacy = app
        .path()
        .resolve(
            "resources/mcp/config.json",
            tauri::path::BaseDirectory::AppData,
        )
        .map_err(|error| format!("Failed to resolve legacy MCP config path: {error}"))?;
    if legacy != canonical && legacy.is_file() {
        let parent = canonical
            .parent()
            .ok_or_else(|| "MCP config path has no parent directory".to_string())?;
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create MCP config directory: {error}"))?;
        std::fs::copy(&legacy, &canonical)
            .map_err(|error| format!("Failed to migrate legacy MCP config: {error}"))?;
        log::info!(
            "Migrated MCP configuration from {} to {}",
            legacy.display(),
            canonical.display()
        );
    }

    Ok(canonical)
}

pub fn generate_token() -> String {
    // Implement token generation logic (e.g., random string or JWT)
    uuid::Uuid::new_v4().to_string()
}

/// Helper to reveal, unminimize, and focus an existing window
pub fn focus_and_reveal_window(window: &WebviewWindow) -> Result<(), String> {
    let _ = window.show();
    let _ = window.unminimize();
    window.set_focus().map_err(|e| e.to_string())
}

pub fn ensure_window(app: &AppHandle, window_label: &str, url: String) {
    // Try to get the window by label
    match app.get_webview_window(window_label) {
        Some(w) => {
            println!("Window is existing, revealing and focusing it now...");
            let _ = focus_and_reveal_window(&w);
        }
        None => {
            println!("Window not found, creating a new one...");
            // If the window doesn't exist, create it
            let _ = build_main_window(app, url);
        }
    }
}

pub fn build_main_window(
    app_handle: &AppHandle,
    url: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut builder =
        tauri::WebviewWindowBuilder::new(app_handle, "main", tauri::WebviewUrl::App(url.into()))
            //.position(300.0, 500.0)
            .visible(false) // Make it initially hidden
            .title("VortiDeck")
            .inner_size(1016.0, 769.0)
            .min_inner_size(1016.0, 769.0)
            .focus()
            .resizable(true)
            .decorations(false)
            .maximizable(false)
            .disable_drag_drop_handler();

    // Platform-specific window settings
    #[cfg(target_os = "windows")]
    {
        builder = builder.transparent(true);
    }

    #[cfg(target_os = "macos")]
    {
        builder = builder
    }

    builder.build().unwrap();

    Ok(())
}

//? Premium features

use crate::commands::auth_commands::get_access_token;
use std::error::Error;

#[tauri::command]
pub fn check_premium_status() {
    let access_token = get_access_token();

    log::debug!("Premium access token available: {}", access_token.is_ok());
}

// === CLIENT FILTERING UTILITIES ===

/// Represents the different types of clients that can connect
#[derive(Debug, Clone, PartialEq)]
pub enum ClientType {
    /// Mobile device connecting from a different IP
    MobileDevice,
    /// Plugin (OBS, etc.)
    Plugin,
    /// Host/same-device client (different app windows, web browser on same machine)
    HostClient,
    /// Other external device (non-mobile)
    ExternalDevice,
}

/// Determines the client type based on connection parameters
pub fn classify_client(is_plugin: bool, peer_ip: &str, server_ip: &IpAddr) -> ClientType {
    if is_plugin {
        ClientType::Plugin
    } else if peer_ip == server_ip.to_string() {
        ClientType::HostClient
    } else {
        // For now, we'll classify all external non-plugin clients as mobile devices
        // This could be enhanced with device type detection in the future
        ClientType::MobileDevice
    }
}

/// Checks if a client should be registered for mobile volume updates
pub fn should_register_for_volume_updates(
    is_plugin: bool,
    peer_ip: &str,
    server_ip: &IpAddr,
) -> bool {
    let client_type = classify_client(is_plugin, peer_ip, server_ip);
    matches!(client_type, ClientType::MobileDevice)
}

/// Checks if a client should be excluded from peer listings
pub fn should_exclude_from_peer_list(device_type: &str, device_model: &str) -> bool {
    device_type == "host" || device_model == "obs_plugin" || device_model.contains("_plugin")
}

/// Checks if a client should receive broadcast messages
pub fn should_receive_broadcasts(is_plugin: bool, peer_ip: &str, server_ip: &IpAddr) -> bool {
    let client_type = classify_client(is_plugin, peer_ip, server_ip);
    matches!(
        client_type,
        ClientType::MobileDevice | ClientType::ExternalDevice
    )
}

/// Gets a human-readable description of why a client was excluded/included
pub fn get_client_exclusion_reason(is_plugin: bool, peer_ip: &str, server_ip: &IpAddr) -> String {
    let client_type = classify_client(is_plugin, peer_ip, server_ip);
    match client_type {
        ClientType::Plugin => "Plugin client (excluded from mobile features)".to_string(),
        ClientType::HostClient => {
            "Host/same-device client (excluded from mobile features)".to_string()
        }
        ClientType::MobileDevice => "Mobile device (included in mobile features)".to_string(),
        ClientType::ExternalDevice => "External device (included in most features)".to_string(),
    }
}

/// Checks if a client is considered "internal" (should be excluded from most user-facing lists)
pub fn is_internal_client(is_plugin: bool, peer_ip: &str, server_ip: &IpAddr) -> bool {
    let client_type = classify_client(is_plugin, peer_ip, server_ip);
    matches!(client_type, ClientType::Plugin | ClientType::HostClient)
}

// pub fn build_credentials(username: &str, password: &str) -> Result<(), Box<dyn Error>> {
//     let entry = Entry::new(SERVICE_NAME, username)?;
//     entry.set_password(password)?;
//     Ok(())
// }

// pub fn get_credentials(username: &str) -> Result<String, Box<dyn Error>> {
//     let entry = Entry::new(SERVICE_NAME, username)?;
//     let password = entry.get_password()?;
//     Ok(password)
// }

// pub fn delete_credentials(username: &str) -> Result<(), Box<dyn Error>> {
//     let entry = Entry::new(SERVICE_NAME, username)?;
//     entry.delete_password()?;
//     Ok(())
// }

// pub fn build_tray_window(app_handle: &AppHandle, url: String) -> Result<(), Box<dyn std::error::Error>> {
//     let mut builder = tauri::WebviewWindowBuilder::new(
//         app_handle,
//         "tray_menu",
//         tauri::WebviewUrl::App(url.into()),
//     )

//     Ok(())
// }
