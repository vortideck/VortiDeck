mod commands;
use tauri::Manager;
#[cfg_attr(mobile, tauri::mobile_entry_point)]

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_haptics::init())
        .plugin(tauri_plugin_barcode_scanner::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_keep_screen_on::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_biometry::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_http::init())
        .setup(|app| {
            #[cfg(debug_assertions)] // only include this code on debug builds
            {
              let window = app.get_webview_window("main").unwrap();
              window.open_devtools();
            //   window.close_devtools();
            }

           
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::discover_websocket::discover_websocket,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
