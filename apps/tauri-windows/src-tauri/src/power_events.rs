/// Windows Power Event Detection
///
/// Detects system sleep/wake events using Windows Power Management APIs
/// and triggers WebSocket reconnection when the system resumes from sleep.
use tauri::AppHandle;

#[cfg(target_os = "windows")]
pub fn start_power_event_monitor(app_handle: AppHandle) {
    use windows::core::{w, HSTRING, PCWSTR};
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetWindowLongPtrW,
        RegisterClassW, SetWindowLongPtrW, TranslateMessage, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT,
        GWLP_USERDATA, MSG, WINDOW_EX_STYLE, WINDOW_STYLE, WM_POWERBROADCAST, WNDCLASSW,
    };

    // Power broadcast event constants
    const PBT_APMSUSPEND: u32 = 0x0004;
    const PBT_APMRESUMEAUTOMATIC: u32 = 0x0012;

    println!("⚡ Starting Windows power event monitor...");

    // Spawn background thread for message loop
    std::thread::spawn(move || {
        unsafe {
            let class_name = HSTRING::from("VortiDeckPowerMonitor");

            let hinstance = HINSTANCE(GetModuleHandleW(None).unwrap().0);

            // Register window class
            let wnd_class = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                lpszClassName: PCWSTR(class_name.as_ptr()),
                hInstance: hinstance,
                style: CS_HREDRAW | CS_VREDRAW,
                ..Default::default()
            };

            RegisterClassW(&wnd_class);

            // Create hidden window for receiving power messages
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                PCWSTR(class_name.as_ptr()),
                PCWSTR(class_name.as_ptr()),
                WINDOW_STYLE(0),
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                None,
                None,
                Some(hinstance),
                None,
            )
            .expect("Failed to create window");

            // Store app_handle in window user data so we can access it in window_proc
            let app_ptr = Box::into_raw(Box::new(app_handle)) as isize;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, app_ptr);

            println!("✅ Power event monitor window created");

            // Message loop
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).into() {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            println!("⚡ Power event monitor stopped");
        }
    });
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn window_proc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::LRESULT;
    use windows::Win32::UI::WindowsAndMessaging::{
        DefWindowProcW, GetWindowLongPtrW, GWLP_USERDATA, WM_POWERBROADCAST,
    };

    // Power broadcast event constants
    const PBT_APMSUSPEND: u32 = 0x0004;
    const PBT_APMRESUMEAUTOMATIC: u32 = 0x0012;

    match msg {
        x if x == WM_POWERBROADCAST => {
            let event = wparam.0 as u32;

            match event {
                PBT_APMSUSPEND => {
                    println!("💤 System entering sleep...");
                }
                PBT_APMRESUMEAUTOMATIC => {
                    println!("💤 System resuming from sleep - reconnecting WebSockets...");

                    // Get app_handle from window user data
                    let app_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppHandle;
                    if !app_ptr.is_null() {
                        let app_handle = &*app_ptr;

                        // Trigger reconnection - connection functions will abort old tasks if needed
                        let app_relay = app_handle.clone();
                        let app_streaming = app_handle.clone();

                        tauri::async_runtime::spawn(async move {
                            println!("🔄 Reconnecting relay WebSocket...");
                            if let Err(e) =
                                crate::relay_connection::attempt_relay_connection(app_relay).await
                            {
                                eprintln!("⚠️ Relay reconnection failed: {}", e);
                            }
                        });

                        tauri::async_runtime::spawn(async move {
                            println!("🔄 Reconnecting streaming WebSocket...");
                            if let Err(e) =
                                crate::streaming_connection::attempt_streaming_connection(
                                    app_streaming,
                                )
                                .await
                            {
                                eprintln!("⚠️ Streaming reconnection failed: {}", e);
                            }
                        });
                    }
                }
                _ => {}
            }

            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn start_power_event_monitor(app_handle: AppHandle) {
    // No-op on non-Windows platforms
    println!("⚡ Power event monitoring not supported on this platform");
}
