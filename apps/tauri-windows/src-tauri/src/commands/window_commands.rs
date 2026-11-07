use tauri::{AppHandle, Manager, PhysicalSize, Position, WebviewUrl, WebviewWindowBuilder};

#[tauri::command]
pub async fn open_streaming_window(app: AppHandle) -> Result<(), String> {
    println!("📺 Opening streaming window...");

    // Check if streaming window already exists
    if let Some(window) = app.get_webview_window("streaming") {
        println!("✅ Streaming window already exists, focusing...");
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    // Create new streaming window with custom titlebar
    let streaming_window =
        WebviewWindowBuilder::new(&app, "streaming", WebviewUrl::App("/streaming".into()))
            .title("VortiDeck Streaming Center")
            .inner_size(1050.0, 680.0)
            .min_inner_size(760.0, 460.0)
            .resizable(true)
            .decorations(false) // Custom titlebar like mobile simulator
            .visible(false)
            .center()
            .disable_drag_drop_handler()
            .build()
            .map_err(|e| format!("Failed to create streaming window: {}", e))?;

    println!("✅ Streaming window created");

    Ok(())
}

#[tauri::command]
pub async fn close_streaming_window(app: AppHandle) -> Result<(), String> {
    println!("📺 Closing streaming window...");

    if let Some(window) = app.get_webview_window("streaming") {
        window.close().map_err(|e| e.to_string())?;
        println!("✅ Streaming window closed");
    }

    Ok(())
}

#[tauri::command]
pub async fn toggle_streaming_window_always_on_top(app: AppHandle) -> Result<bool, String> {
    println!("📺 Toggling streaming window always on top");

    if let Some(window) = app.get_webview_window("streaming") {
        // Get current state and toggle
        let current_state = window.is_always_on_top().map_err(|e| e.to_string())?;
        let new_state = !current_state;

        window
            .set_always_on_top(new_state)
            .map_err(|e| e.to_string())?;
        println!("✅ Streaming window always on top: {}", new_state);

        Ok(new_state)
    } else {
        Err("Streaming window not found".to_string())
    }
}

#[tauri::command]
pub async fn get_streaming_window_always_on_top(app: AppHandle) -> Result<bool, String> {
    if let Some(window) = app.get_webview_window("streaming") {
        window.is_always_on_top().map_err(|e| e.to_string())
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub async fn move_streaming_window_to_monitor(
    app: AppHandle,
    monitor_index: usize,
) -> Result<(), String> {
    println!("📺 Moving streaming window to monitor {}", monitor_index);

    if let Some(window) = app.get_webview_window("streaming") {
        let monitors = window.available_monitors().map_err(|e| e.to_string())?;

        if let Some(monitor) = monitors.get(monitor_index) {
            let monitor_pos = monitor.position();
            let monitor_size = monitor.size();

            // Center window on target monitor
            let window_size = window.outer_size().map_err(|e| e.to_string())?;
            let x = monitor_pos.x + (monitor_size.width as i32 - window_size.width as i32) / 2;
            let y = monitor_pos.y + (monitor_size.height as i32 - window_size.height as i32) / 2;

            window
                .set_position(Position::Physical((x, y).into()))
                .map_err(|e| e.to_string())?;

            println!("✅ Moved streaming window to monitor {}", monitor_index);
        } else {
            return Err(format!("Monitor {} not found", monitor_index));
        }
    } else {
        return Err("Streaming window not found".to_string());
    }

    Ok(())
}

#[tauri::command]
pub async fn get_monitors_info(app: AppHandle) -> Result<Vec<MonitorInfo>, String> {
    let main_window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    let monitors = main_window
        .available_monitors()
        .map_err(|e| e.to_string())?;

    let monitor_info: Vec<MonitorInfo> = monitors
        .iter()
        .enumerate()
        .map(|(index, monitor)| {
            let size = monitor.size();
            let position = monitor.position();

            MonitorInfo {
                index,
                name: monitor
                    .name()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("Monitor {}", index + 1)),
                width: size.width,
                height: size.height,
                x: position.x,
                y: position.y,
                scale_factor: monitor.scale_factor(),
            }
        })
        .collect();

    Ok(monitor_info)
}

#[derive(serde::Serialize)]
pub struct MonitorInfo {
    pub index: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub scale_factor: f64,
}

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct GlobalModifierState {
    pub is_alt: bool,
    pub is_shift: bool,
    pub is_ctrl: bool,
}

#[tauri::command]
pub fn get_global_modifiers() -> GlobalModifierState {
    #[cfg(target_os = "windows")]
    {
        unsafe {
            unsafe extern "system" {
                fn GetAsyncKeyState(v_key: i32) -> i16;
            }
            let alt_down = (GetAsyncKeyState(0x12) as u16 & 0x8000) != 0;
            let shift_down = (GetAsyncKeyState(0x10) as u16 & 0x8000) != 0;
            let ctrl_down = (GetAsyncKeyState(0x11) as u16 & 0x8000) != 0;
            GlobalModifierState {
                is_alt: alt_down,
                is_shift: shift_down,
                is_ctrl: ctrl_down,
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        use device_query::{DeviceQuery, DeviceState, Keycode};
        let device_state = DeviceState::new();
        let keys = device_state.get_keys();
        GlobalModifierState {
            is_alt: keys.contains(&Keycode::LAlt) || keys.contains(&Keycode::RAlt),
            is_shift: keys.contains(&Keycode::LShift) || keys.contains(&Keycode::RShift),
            is_ctrl: keys.contains(&Keycode::LControl) || keys.contains(&Keycode::RControl),
        }
    }
}
