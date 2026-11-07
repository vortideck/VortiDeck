use std::sync::Arc;
use tauri::Emitter;
use tauri::{AppHandle, Manager};
use vortideck_core::core::StateManager;

// Helper function to calculate simulator position relative to main window
fn calculate_simulator_position(
    main_position: tauri::PhysicalPosition<i32>,
    main_size: tauri::PhysicalSize<u32>,
    base_width: f64,
    app_handle: &AppHandle,
) -> (f64, f64, i32, i32) {
    // Get available screens
    let screens = app_handle.available_monitors().unwrap_or_default();

    if screens.is_empty() {
        eprintln!("No monitors available, using default positioning");
        let x_pos = (main_position.x + main_size.width as i32) as f64 + 20.0;
        let y_pos = main_position.y as f64;
        return (x_pos, y_pos, 1000, 0); // Default to right side
    }

    // Find the screen containing the main window
    let current_screen = screens.iter().find(|screen| {
        let screen_position = screen.position();
        let screen_size = screen.size();
        main_position.x >= screen_position.x
            && main_position.x < screen_position.x + screen_size.width as i32
            && main_position.y >= screen_position.y
            && main_position.y < screen_position.y + screen_size.height as i32
    });

    // Use the found screen or fall back to the first screen (primary)
    let screen_to_use = current_screen.unwrap_or_else(|| &screens[0]);

    // Calculate available space on both sides
    let screen_position = screen_to_use.position();
    let screen_size = screen_to_use.size();
    let space_on_left = main_position.x - screen_position.x;
    let space_on_right =
        (screen_position.x + screen_size.width as i32) - (main_position.x + main_size.width as i32);

    // Determine position based on available space
    let x_pos = if space_on_right >= space_on_left {
        // Position on right side
        (main_position.x + main_size.width as i32) as f64 + 20.0 // Added 20px gap
    } else {
        // Position on left side
        main_position.x as f64 - base_width - 20.0 // Added 20px gap
    };

    let y_pos = main_position.y as f64;

    (x_pos, y_pos, space_on_right, space_on_left)
}

#[tauri::command]
pub async fn open_mobile_simulator(app_handle: AppHandle) {
    // Check if the "mobile_simulator" window exists
    if let Some(mobile_simulator_window) = app_handle.get_webview_window("mobile_simulator") {
        eprintln!("Mobile Simulator Window already exists");
        let _ = mobile_simulator_window.show();
        let _ = mobile_simulator_window.unminimize();
        // Emit event to notify frontend that simulator is open
        let _ = app_handle.emit("toggle_is_simulator", true);
        return;
    } else {
        let main_window = app_handle.get_webview_window("main").unwrap();
        let base_width = 550.0;
        let base_height = 700.0;

        // Get main window position and size
        let main_position = main_window.outer_position().unwrap();
        let main_size = main_window.inner_size().unwrap();

        // Calculate initial position
        let (x_pos, y_pos, space_on_right, space_on_left) =
            calculate_simulator_position(main_position, main_size, base_width, &app_handle);

        let mut builder = tauri::WebviewWindowBuilder::new(
            &app_handle,
            "mobile_simulator",
            tauri::WebviewUrl::App("deck/simulator".into()),
        )
        .title("VortiDeck - Mobile Simulator")
        .parent(&main_window)
        .expect("Failed to set parent window")
        .position(x_pos, y_pos)
        .resizable(false)
        .decorations(false)
        .inner_size(base_width, base_height)
        .visible(false)
        .maximizable(false)
        .disable_drag_drop_handler()
        .focusable(false);

        // Platform-specific window settings
        #[cfg(target_os = "windows")]
        {
            builder = builder.transparent(true);
        }

        #[cfg(target_os = "macos")]
        {
            builder = builder
        }

        let simulator_window = builder.build().unwrap();

        // Emit event to notify frontend that simulator is open
        let _ = app_handle.emit("toggle_is_simulator", true);

        // Listen for simulator window close event
        let app_handle_for_close = app_handle.clone();
        simulator_window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // Emit event to notify frontend that simulator is closed
                let _ = app_handle_for_close.emit("toggle_is_simulator", false);
            }
        });

        // Listen for main window move events to update simulator position
        let simulator_window_clone = simulator_window.clone();
        let base_width_clone = base_width;

        // Clone the app_handle to move into the closure
        let app_handle_clone = app_handle.clone();

        main_window.on_window_event(move |event| {
            if let tauri::WindowEvent::Moved(position) = event {
                // Use StateManager directly (with error handling)
                let state_manager = match app_handle_clone.try_state::<Arc<StateManager>>() {
                    Some(state) => state,
                    None => {
                        eprintln!("❌ StateManager not available for simulator positioning");
                        return;
                    }
                };
                if state_manager.is_mobile_simulator_pinned() {
                    // Get current main window size
                    if let Ok(current_main_size) = app_handle_clone
                        .get_webview_window("main")
                        .unwrap()
                        .inner_size()
                    {
                        // Recalculate optimal positioning based on current position
                        let (_, _, space_on_right, space_on_left) = calculate_simulator_position(
                            *position,
                            current_main_size,
                            base_width_clone,
                            &app_handle_clone,
                        );

                        let new_x = if space_on_right >= space_on_left {
                            position.x + current_main_size.width as i32 + 20
                        } else {
                            position.x - base_width_clone as i32 - 20
                        };
                        let new_y = position.y;

                        let _ = simulator_window_clone.set_position(tauri::Position::Physical(
                            tauri::PhysicalPosition { x: new_x, y: new_y },
                        ));
                    }
                }
            }
        });
    }
}

//     // let position = main_window.inner_position();

//     // // Explicitly define the type for `to_logical` as `f64`
//     // let logical = position
//     //     .expect("REASON")
//     //     .to_logical::<f64>(factor);  // Specify `f64` as the pixel type

//     // // Define a base size ratio (width: 300, height: 900) - this is your ideal size.
//     let base_width = 550.0;
//     let base_height = 700.0;

//     // // Adjust the base width and height based on the scale factor
//     // let adjusted_width = base_width * factor; // Adjust the width based on the scale factor
//     // let adjusted_height = base_height * factor; // Adjust the height based on the scale factor

//     // Create the webview window for the mobile simulator with the calculated relative size

//     let webview_window = tauri::WebviewWindowBuilder::new(&app_handle, "mobile_simulator", tauri::WebviewUrl::App("deck/simulator".into()))
//         .parent(&main_window).expect("REASON")
//         .resizable(false)
//         .transparent(true)
//         .decorations(false)
//         .inner_size(base_width, base_height) // Correctly pass as f64
//         .visible(false)
//         .build()
//         .unwrap();
// }

// Add new command
#[tauri::command]
pub async fn toggle_simulator_pin(app_handle: AppHandle) {
    if let Some(simulator) = app_handle.get_webview_window("mobile_simulator") {
        // Use StateManager directly (with error handling)
        let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
            Some(state) => state,
            None => {
                eprintln!("❌ StateManager not available for simulator toggle");
                return;
            }
        };

        // Toggle dock (companion snapping) state
        if state_manager.is_mobile_simulator_pinned() {
            state_manager.unpin_mobile_simulator();
            println!("📱 Mobile simulator unpinned from main window");
        } else {
            state_manager.pin_mobile_simulator();
            println!("📱 Mobile simulator docked to main window");

            // Immediately reposition the simulator when pinning to main window
            if let Some(main_window) = app_handle.get_webview_window("main") {
                if let (Ok(main_position), Ok(main_size)) =
                    (main_window.outer_position(), main_window.inner_size())
                {
                    let base_width = 550.0;
                    let (_x_pos, _y_pos, space_on_right, space_on_left) =
                        calculate_simulator_position(
                            main_position,
                            main_size,
                            base_width,
                            &app_handle,
                        );

                    let new_x = if space_on_right >= space_on_left {
                        main_position.x + main_size.width as i32 + 20
                    } else {
                        main_position.x - base_width as i32 - 20
                    };
                    let new_y = main_position.y;

                    let _ = simulator.set_position(tauri::Position::Physical(
                        tauri::PhysicalPosition { x: new_x, y: new_y },
                    ));
                }
            }
        }
    }
}

/// Toggle OS-level Always on Top for Mobile Simulator (floats above all desktop windows)
#[tauri::command]
pub async fn toggle_simulator_always_on_top(app_handle: AppHandle) -> Result<bool, String> {
    if let Some(simulator) = app_handle.get_webview_window("mobile_simulator") {
        let is_on_top = simulator
            .is_always_on_top()
            .map_err(|e| format!("Failed to get always on top state: {}", e))?;
        let new_state = !is_on_top;

        #[cfg(target_os = "windows")]
        {
            use windows::Win32::Foundation::HWND;
            use windows::Win32::UI::WindowsAndMessaging::{SetWindowLongPtrW, GWLP_HWNDPARENT};

            if let Ok(sim_hwnd) = simulator.hwnd() {
                let sim_hwnd = HWND(sim_hwnd.0 as _);
                if new_state {
                    // Detach from main window parent so simulator can float freely above all OS windows
                    unsafe {
                        let _ = SetWindowLongPtrW(sim_hwnd, GWLP_HWNDPARENT, 0);
                    }
                } else if let Some(main_win) = app_handle.get_webview_window("main") {
                    // Reattach main window as parent
                    if let Ok(main_hwnd) = main_win.hwnd() {
                        unsafe {
                            let _ =
                                SetWindowLongPtrW(sim_hwnd, GWLP_HWNDPARENT, main_hwnd.0 as isize);
                        }
                    }
                }
            }
        }

        simulator
            .set_always_on_top(new_state)
            .map_err(|e| format!("Failed to set always on top state: {}", e))?;
        println!("📱 Mobile simulator always on top: {}", new_state);
        Ok(new_state)
    } else {
        Err("Mobile simulator window not found".to_string())
    }
}

#[tauri::command]
pub async fn get_simulator_always_on_top(app_handle: AppHandle) -> bool {
    if let Some(simulator) = app_handle.get_webview_window("mobile_simulator") {
        simulator.is_always_on_top().unwrap_or(false)
    } else {
        false
    }
}

#[tauri::command]
pub async fn get_mobile_simulator_pinned(app_handle: AppHandle) -> bool {
    // Use StateManager directly (with error handling)
    let state_manager = match app_handle.try_state::<Arc<StateManager>>() {
        Some(state) => state,
        None => {
            eprintln!("❌ StateManager not available for simulator pin status");
            return false;
        }
    };
    let is_pinned = state_manager.is_mobile_simulator_pinned();
    println!(
        "📱 Mobile simulator pin status: {} (via StateManager)",
        is_pinned
    );
    is_pinned
}
