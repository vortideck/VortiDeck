use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use vortideck_core::db::db::DbPool;
use vortideck_core::db::models::{FocusWindow, NewFocusWindow, UpdateFocusWindow};

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowInfo {
    pub title: String,
    pub process_name: String,
    pub process_id: u32,
}

fn normalize_match_mode(mode: Option<&str>) -> String {
    match mode.unwrap_or("smart").trim().to_lowercase().as_str() {
        "smart" => "smart".to_string(),
        "exact" => "exact".to_string(),
        "contains" => "contains".to_string(),
        "regex" => "regex".to_string(),
        "process_only" => "process_only".to_string(),
        _ => "smart".to_string(),
    }
}

/// Get a list of all visible windows
#[tauri::command]
pub async fn get_visible_windows() -> Result<Vec<WindowInfo>, String> {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use windows::{
            core::BOOL,
            Win32::Foundation::{HWND, LPARAM},
            Win32::System::Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_NATIVE,
                PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
            },
            Win32::UI::WindowsAndMessaging::{
                EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
            },
        };

        let mut windows = Vec::new();

        unsafe {
            unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
                let windows = &mut *(lparam.0 as *mut Vec<WindowInfo>);

                if IsWindowVisible(hwnd).as_bool() {
                    let mut title: [u16; 512] = [0; 512];
                    let len = GetWindowTextW(hwnd, &mut title);

                    if len > 0 {
                        let window_title = OsString::from_wide(&title[..len as usize])
                            .to_string_lossy()
                            .into_owned();

                        if window_title.trim().is_empty() {
                            return BOOL(1);
                        }

                        let mut process_id: u32 = 0;
                        GetWindowThreadProcessId(hwnd, Some(&mut process_id as *mut u32));

                        let mut process_name = String::from("Unknown");

                        if let Ok(handle) = OpenProcess(
                            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
                            false,
                            process_id,
                        ) {
                            let mut process_name_buf: [u16; 512] = [0; 512];
                            let mut size = 512u32;
                            if QueryFullProcessImageNameW(
                                handle,
                                PROCESS_NAME_NATIVE,
                                windows::core::PWSTR(process_name_buf.as_mut_ptr()),
                                &mut size as *mut u32,
                            )
                            .is_ok()
                            {
                                let full_path =
                                    OsString::from_wide(&process_name_buf[..size as usize])
                                        .to_string_lossy()
                                        .into_owned();
                                process_name = std::path::Path::new(&full_path)
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("Unknown")
                                    .to_string();
                            }
                            let _ = windows::Win32::Foundation::CloseHandle(handle);
                        }

                        windows.push(WindowInfo {
                            title: window_title,
                            process_name,
                            process_id,
                        });
                    }
                }

                BOOL(1)
            }

            let lparam = LPARAM(&mut windows as *mut _ as isize);

            if EnumWindows(Some(enum_callback), lparam).is_err() {
                return Err("Failed to enumerate windows".to_string());
            }
        }

        windows.sort_by(|a: &WindowInfo, b: &WindowInfo| a.title.cmp(&b.title));

        Ok(windows)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Window enumeration is only supported on Windows".to_string())
    }
}

/// Get all focus window actions
#[tauri::command]
pub async fn get_focus_windows(db_pool: State<'_, DbPool>) -> Result<Vec<FocusWindow>, String> {
    use diesel::prelude::*;
    use vortideck_core::db::schema::focus_windows;

    let mut conn = db_pool.get().map_err(|e| e.to_string())?;

    focus_windows::table
        .order(focus_windows::name.asc())
        .load::<FocusWindow>(&mut conn)
        .map_err(|e| format!("Failed to load focus windows: {}", e))
}

/// Get a single focus window by ID
#[tauri::command]
pub async fn get_focus_window_by_id(
    db_pool: State<'_, DbPool>,
    id: i32,
) -> Result<FocusWindow, String> {
    use diesel::prelude::*;
    use vortideck_core::db::schema::focus_windows;

    let mut conn = db_pool.get().map_err(|e| e.to_string())?;

    focus_windows::table
        .find(id)
        .first::<FocusWindow>(&mut conn)
        .map_err(|e| format!("Failed to load focus window: {}", e))
}

/// Create a new focus window action
#[tauri::command]
pub async fn create_focus_window(
    db_pool: State<'_, DbPool>,
    app_handle: tauri::AppHandle,
    name: String,
    window_title: String,
    process_name: String,
    match_mode: Option<String>,
) -> Result<serde_json::Value, serde_json::Value> {
    use diesel::prelude::*;
    use vortideck_core::db::schema::focus_windows;

    let mut conn = db_pool
        .get()
        .map_err(|e| serde_json::json!({"status": 500, "message": format!("DB error: {}", e)}))?;

    let normalized_mode = normalize_match_mode(match_mode.as_deref());
    let new_window = NewFocusWindow {
        name: &name,
        window_title: &window_title,
        process_name: &process_name,
        match_mode: &normalized_mode,
    };

    diesel::insert_into(focus_windows::table)
        .values(&new_window)
        .execute(&mut conn)
        .map_err(
            |e| serde_json::json!({"status": 500, "message": format!("Insert failed: {}", e)}),
        )?;

    let created = focus_windows::table
        .order(focus_windows::id.desc())
        .first::<FocusWindow>(&mut conn)
        .map_err(
            |e| serde_json::json!({"status": 500, "message": format!("Fetch failed: {}", e)}),
        )?;

    let _ = app_handle.emit("focus_window_created", ());

    Ok(serde_json::json!({"data": created, "status": 200, "message": "Created"}))
}

/// Update a focus window action
#[tauri::command]
pub async fn update_focus_window(
    db_pool: State<'_, DbPool>,
    app_handle: tauri::AppHandle,
    id: i32,
    name: Option<String>,
    window_title: Option<String>,
    process_name: Option<String>,
    match_mode: Option<String>,
) -> Result<serde_json::Value, serde_json::Value> {
    use diesel::prelude::*;
    use vortideck_core::db::schema::focus_windows;

    let mut conn = db_pool
        .get()
        .map_err(|e| serde_json::json!({"status": 500, "message": format!("{}", e)}))?;

    let normalized_mode = match_mode
        .as_deref()
        .map(|value| normalize_match_mode(Some(value)));
    let update = UpdateFocusWindow {
        name: name.as_deref(),
        window_title: window_title.as_deref(),
        process_name: process_name.as_deref(),
        match_mode: normalized_mode.as_deref(),
    };

    diesel::update(focus_windows::table.find(id))
        .set(&update)
        .execute(&mut conn)
        .map_err(|e| serde_json::json!({"status": 500, "message": format!("{}", e)}))?;

    let updated = focus_windows::table
        .find(id)
        .first::<FocusWindow>(&mut conn)
        .map_err(|e| serde_json::json!({"status": 500, "message": format!("{}", e)}))?;

    let _ = app_handle.emit("focus_window_updated", ());

    Ok(serde_json::json!({"data": updated, "status": 200, "message": "Updated"}))
}

/// Delete a focus window action
#[tauri::command]
pub async fn delete_focus_window(
    db_pool: State<'_, DbPool>,
    id: i32,
) -> Result<serde_json::Value, serde_json::Value> {
    use diesel::prelude::*;
    use vortideck_core::db::schema::focus_windows;

    let mut conn = db_pool
        .get()
        .map_err(|e| serde_json::json!({"status": 500, "message": format!("{}", e)}))?;

    diesel::delete(focus_windows::table.find(id))
        .execute(&mut conn)
        .map_err(|e| serde_json::json!({"status": 500, "message": format!("{}", e)}))?;

    Ok(serde_json::json!({"status": 200, "message": "Deleted"}))
}
