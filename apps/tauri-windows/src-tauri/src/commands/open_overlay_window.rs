use tauri::AppHandle;
use tauri::Manager;

#[tauri::command]
pub async fn open_overlay_window(app_handle: AppHandle) -> Result<String, String> {
    let page_path = "/streaming/overlays".to_string();
    let window_title = "VortiDeck Overlay Manager".to_string();

    // Generate a unique window name based on the page path (same pattern as open_in_new_window)
    let window_name = format!("window_{}", page_path.replace("/", "_").replace(":", "_"));

    // Check if the window already exists
    if let Some(existing_window) = app_handle.get_webview_window(&window_name) {
        let _ = crate::utils::focus_and_reveal_window(&existing_window);
        return Ok("Overlay window focused".to_string());
    }

    // Create a new window (same pattern as open_in_new_window)
    let base_width = 1200.0;
    let base_height = 800.0;

    let mut builder = tauri::WebviewWindowBuilder::new(
        &app_handle,
        &window_name,
        tauri::WebviewUrl::App(page_path.into()),
    )
    .position(100.0, 100.0)
    .resizable(true)
    .decorations(false)
    .inner_size(base_width, base_height)
    .visible(false)
    .maximizable(true)
    .minimizable(true)
    .title(&window_title)
    .disable_drag_drop_handler();

    // Platform-specific window settings
    #[cfg(target_os = "windows")]
    {
        builder = builder.transparent(false);
    }

    match builder.build() {
        Ok(_) => {
            println!("✅ Overlay window created successfully");
            Ok("Overlay window opened successfully".to_string())
        }
        Err(e) => {
            eprintln!("❌ Error creating overlay window: {}", e);
            Err(format!("Failed to create overlay window: {}", e))
        }
    }
}

#[tauri::command]
pub async fn close_overlay_window(app_handle: AppHandle) -> Result<String, String> {
    let window_name = "vorti_overlay_window";

    if let Some(overlay_window) = app_handle.get_webview_window(window_name) {
        match overlay_window.close() {
            Ok(_) => {
                println!("✅ Overlay window closed successfully");
                Ok("Overlay window closed".to_string())
            }
            Err(e) => {
                eprintln!("❌ Error closing overlay window: {}", e);
                Err(format!("Failed to close overlay window: {}", e))
            }
        }
    } else {
        Ok("Overlay window not found or already closed".to_string())
    }
}

#[tauri::command]
pub async fn toggle_overlay_window(app_handle: AppHandle) -> Result<String, String> {
    let window_name = "vorti_overlay_window";

    if let Some(overlay_window) = app_handle.get_webview_window(window_name) {
        // Window exists, check if it's visible and toggle
        match overlay_window.is_visible() {
            Ok(is_visible) => {
                if is_visible {
                    // Hide the window
                    if let Err(e) = overlay_window.hide() {
                        return Err(format!("Failed to hide overlay window: {}", e));
                    }
                    Ok("Overlay window hidden".to_string())
                } else {
                    // Show and focus the window
                    if let Err(e) = overlay_window.show() {
                        return Err(format!("Failed to show overlay window: {}", e));
                    }
                    if let Err(e) = overlay_window.set_focus() {
                        eprintln!("Warning: Failed to focus overlay window: {}", e);
                    }
                    Ok("Overlay window shown".to_string())
                }
            }
            Err(e) => {
                eprintln!("❌ Error checking overlay window visibility: {}", e);
                Err(format!("Failed to check overlay window state: {}", e))
            }
        }
    } else {
        // Window doesn't exist, create it
        open_overlay_window(app_handle).await
    }
}
