use tauri::AppHandle;
use tauri::Manager;

#[tauri::command]
pub async fn open_in_new_window(app_handle: AppHandle, page_path: String, window_title: String) {
    // Generate a unique window name based on the page path
    let window_name = format!("window_{}", page_path.replace("/", "_").replace(":", "_"));

    // Check if the window already exists
    if let Some(existing_window) = app_handle.get_webview_window(&window_name) {
        let _ = crate::utils::focus_and_reveal_window(&existing_window);
        return;
    }

    // Create a new window
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

    if let Err(e) = builder.build() {
        eprintln!("Error creating window: {}", e);
    }
}
