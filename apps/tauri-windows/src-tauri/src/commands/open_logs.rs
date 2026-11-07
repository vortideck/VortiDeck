use tauri::AppHandle;
use tauri::Listener;
use tauri::Manager;

#[tauri::command]
pub async fn open_logs(app_handle: AppHandle) {
    // Check if the "logs" window exists
    if let Some(logs_window) = app_handle.get_webview_window("logs") {
        eprintln!("Logs Window already exists");
        let _ = crate::utils::focus_and_reveal_window(&logs_window);
        return;
    } else {
        // let logs_config = tauri::WindowConfig
        let main_window = app_handle.get_webview_window("main").unwrap(); // Make sure "main" is the name of your main window
                                                                          // Define the base size for the mobile simulator
        let base_width = 1016.0;
        let base_height = 569.0;

        //Try to create and show the mobile simulator window
        let mut builder = tauri::WebviewWindowBuilder::new(
            &app_handle,
            "logs",
            tauri::WebviewUrl::App("/logs".into()),
        )
        // .parent(&main_window)
        // .expect("REASON") // Attach it to the main window
        .position(300.0, 500.0)
        .resizable(true)
        .decorations(false)
        .inner_size(base_width, base_height)
        .visible(false) // Make it initially hidden
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

        // let webview_window = tauri::window::WindowBuilder::from_config(&app_handle, &app_handle.config().app.windows.get(1).unwrap().clone())
        //     .unwrap()
        //     .build()
        //     .unwrap();

        // Now that the window is created, show it (if needed)
        // webview_window.show().unwrap();
    }
}
