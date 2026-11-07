use diesel::prelude::*;
use diesel::SqliteConnection;
/// Context Menu Commands
///
/// Tauri commands for managing the Windows 11 modern context menu items.
/// Users can dynamically add/edit/remove context menu entries that appear
/// in the Explorer right-click menu via the IExplorerCommand shell extension.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::command;
use tauri::Manager;
use vortideck_core::db::db::DbPool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMenuConfig {
    pub version: u32,
    pub vortideck_path: String,
    pub items: Vec<ContextMenuItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMenuItem {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub icon: String,
    pub file_types: Vec<String>,
    pub action: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub order: i32,
    /// Optional group name. Items with the same group appear under a sub-menu.
    #[serde(default)]
    pub group: String,
    /// If true, this item appears at root level in the context menu (not under VortiDeck).
    /// Up to 8 root-level items are supported via pre-reserved CLSID slots.
    #[serde(default)]
    pub root_level: bool,
    /// Working directory template (supports variables). Defaults to {parent} if empty.
    #[serde(default)]
    pub working_dir: String,
    /// Window show mode: "normal", "hidden", "minimized"
    #[serde(default)]
    pub show_window: String,
    /// If true, show this item when right-clicking a folder.
    #[serde(default = "default_true")]
    pub match_folders: bool,
    /// If true, show this item when right-clicking empty space in a directory.
    #[serde(default = "default_true")]
    pub match_background: bool,
    /// If true, wrap the command in cmd.exe /c (for .bat/.cmd scripts).
    #[serde(default)]
    pub use_cmd: bool,
    /// Optional icon path for dark mode.
    #[serde(default)]
    pub icon_dark: String,
    /// If true, show this item when right-clicking the Windows desktop background.
    #[serde(default)]
    pub match_desktop: bool,
    /// If true, show this item when right-clicking a drive root (C:\, D:\, etc.).
    #[serde(default)]
    pub match_drive: bool,
    /// File name patterns (e.g. "Makefile", "*.config"). Empty means no name filter.
    #[serde(default)]
    pub file_names: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl Default for ContextMenuConfig {
    fn default() -> Self {
        Self {
            version: 1,
            vortideck_path: String::new(),
            items: vec![],
        }
    }
}

fn config_path() -> Result<PathBuf, String> {
    std::env::var("APPDATA")
        .map(|appdata| {
            PathBuf::from(appdata)
                .join("VortiDeck")
                .join("context-menus.json")
        })
        .map_err(|_| "APPDATA not set".to_string())
}

pub fn read_config() -> Result<ContextMenuConfig, String> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(ContextMenuConfig::default());
    }
    let contents =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read config: {}", e))?;
    serde_json::from_str(&contents).map_err(|e| format!("Failed to parse config: {}", e))
}

fn write_config(config: &ContextMenuConfig) -> Result<(), String> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    let contents = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    std::fs::write(&path, contents).map_err(|e| format!("Failed to write config: {}", e))
}

/// Ensure the config file exists with VortiDeck path set correctly.
pub fn ensure_config(vortideck_exe_path: &str) {
    if let Ok(mut config) = read_config() {
        if config.vortideck_path != vortideck_exe_path {
            config.vortideck_path = vortideck_exe_path.to_string();
            if let Err(e) = write_config(&config) {
                log::warn!("Failed to update context menu config vortideck_path: {}", e);
            }
        }
    } else {
        let config = ContextMenuConfig {
            version: 1,
            vortideck_path: vortideck_exe_path.to_string(),
            items: vec![ContextMenuItem {
                id: "open-assistant".to_string(),
                title: "Open with VortiDeck Assistant".to_string(),
                icon: String::new(),
                file_types: vec!["*".to_string()],
                action: "open-page-assistant".to_string(),
                enabled: true,
                order: 0,
                group: String::new(),
                root_level: false,
                working_dir: String::new(),
                show_window: String::new(),
                match_folders: true,
                match_background: true,
                use_cmd: false,
                icon_dark: String::new(),
                match_desktop: false,
                match_drive: false,
                file_names: vec![],
            }],
        };
        if let Err(e) = write_config(&config) {
            log::warn!("Failed to create initial context menu config: {}", e);
        }
    }
}

/// Check if an action's referenced entity still exists in the database.
/// Returns true if the entity exists or the action doesn't reference a DB entity.
fn action_entity_exists(conn: &mut SqliteConnection, action: &str) -> bool {
    if action.is_empty() || !action.starts_with('{') {
        return true; // Not a DB reference (e.g. "open-page-assistant")
    }
    let parsed: serde_json::Value = match serde_json::from_str(action) {
        Ok(v) => v,
        Err(_) => return true, // Can't parse — don't flag as orphan
    };
    if let Some(eid) = parsed.get("Program").and_then(|v| v.as_i64()) {
        use vortideck_core::db::schema::programs::dsl::*;
        return programs
            .filter(id.eq(eid as i32))
            .count()
            .get_result::<i64>(conn)
            .unwrap_or(0)
            > 0;
    }
    if let Some(eid) = parsed.get("MacroId").and_then(|v| v.as_i64()) {
        use vortideck_core::db::schema::macros::dsl::*;
        return macros
            .filter(id.eq(eid as i32))
            .count()
            .get_result::<i64>(conn)
            .unwrap_or(0)
            > 0;
    }
    if let Some(fid) = parsed.get("FlowId").and_then(|v| v.as_str()) {
        use vortideck_core::db::schema::flows::dsl::*;
        return flows
            .filter(id.eq(fid))
            .count()
            .get_result::<i64>(conn)
            .unwrap_or(0)
            > 0;
    }
    if let Some(saved_id) = parsed.get("SavedActionId").and_then(|v| v.as_str()) {
        use vortideck_core::db::schema::saved_actions::dsl::*;
        return saved_actions
            .filter(stable_id.eq(saved_id))
            .count()
            .get_result::<i64>(conn)
            .unwrap_or(0)
            > 0;
    }
    if let Some(eid) = parsed.get("HttpRequestId").and_then(|v| v.as_i64()) {
        use vortideck_core::db::schema::http_requests::dsl::*;
        return http_requests
            .filter(id.eq(eid as i32))
            .count()
            .get_result::<i64>(conn)
            .unwrap_or(0)
            > 0;
    }
    if let Some(eid) = parsed.get("FocusWindowId").and_then(|v| v.as_i64()) {
        use vortideck_core::db::schema::focus_windows::dsl::*;
        return focus_windows
            .filter(id.eq(eid as i32))
            .count()
            .get_result::<i64>(conn)
            .unwrap_or(0)
            > 0;
    }
    // SystemControl(String), App(String) — no DB reference, always valid
    true
}

/// Clear action and disable any context menu items referencing a deleted entity.
/// `action_key` is the JSON key (e.g. "Program", "MacroId", "FlowId").
/// `entity_id` is the stringified ID value.
/// Returns true if any items were modified.
pub fn clear_orphaned_items(action_key: &str, entity_id: &str) -> bool {
    let mut config = match read_config() {
        Ok(c) => c,
        Err(_) => return false,
    };

    let mut changed = false;
    for item in config.items.iter_mut() {
        if item.action.is_empty() || !item.action.starts_with('{') {
            continue;
        }
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&item.action) {
            let matches = match parsed.get(action_key) {
                Some(serde_json::Value::Number(n)) => n.to_string() == entity_id,
                Some(serde_json::Value::String(s)) => s == entity_id,
                _ => false,
            };
            if matches {
                log::info!(
                    "Clearing orphaned context menu item '{}' (was: {})",
                    item.id,
                    item.action
                );
                item.action = String::new();
                item.enabled = false;
                changed = true;
            }
        }
    }

    if changed {
        let _ = write_config(&config);
    }
    changed
}

/// Count references without modifying user configuration. Deletion guards use
/// this so a saved action cannot silently break a context-menu item.
pub fn count_action_references(action_key: &str, entity_id: &str) -> usize {
    let Ok(config) = read_config() else { return 0 };
    config
        .items
        .iter()
        .filter(|item| {
            serde_json::from_str::<serde_json::Value>(&item.action)
                .ok()
                .and_then(|parsed| parsed.get(action_key).cloned())
                .is_some_and(|value| match value {
                    serde_json::Value::Number(number) => number.to_string() == entity_id,
                    serde_json::Value::String(value) => value == entity_id,
                    _ => false,
                })
        })
        .count()
}

/// Replace an exact entity reference in context-menu actions. Returns the
/// number of items changed so repair tooling can report its full impact.
pub fn replace_action_references(
    action_key: &str,
    old_entity_id: &str,
    new_entity_id: &str,
) -> Result<usize, String> {
    let mut config = read_config()?;
    let mut changed = 0;
    for item in &mut config.items {
        let Ok(mut parsed) = serde_json::from_str::<serde_json::Value>(&item.action) else {
            continue;
        };
        let Some(object) = parsed.as_object_mut() else {
            continue;
        };
        let matches = object.get(action_key).is_some_and(|value| match value {
            serde_json::Value::Number(number) => number.to_string() == old_entity_id,
            serde_json::Value::String(value) => value == old_entity_id,
            _ => false,
        });
        if matches {
            object.insert(
                action_key.to_string(),
                serde_json::Value::String(new_entity_id.to_string()),
            );
            item.action = serde_json::to_string(&parsed).map_err(|error| error.to_string())?;
            changed += 1;
        }
    }
    if changed > 0 {
        write_config(&config)?;
    }
    Ok(changed)
}

/// Get all context menu items, auto-disabling any with orphaned entity references.
#[command]
pub async fn get_context_menu_items(
    db_pool: tauri::State<'_, DbPool>,
) -> Result<Vec<ContextMenuItem>, String> {
    let mut config = read_config()?;

    let conn = &mut db_pool
        .get()
        .map_err(|e| format!("Database connection error: {}", e))?;

    let mut changed = false;
    for item in config.items.iter_mut() {
        if item.enabled && !item.action.is_empty() && item.action.starts_with('{') {
            if !action_entity_exists(conn, &item.action) {
                log::info!(
                    "Context menu item '{}' references deleted entity ({}), disabling",
                    item.id,
                    item.action
                );
                item.enabled = false;
                changed = true;
            }
        }
    }

    if changed {
        let _ = write_config(&config);
    }

    Ok(config.items)
}

/// Get the full context menu config.
#[command]
pub async fn get_context_menu_config() -> Result<ContextMenuConfig, String> {
    read_config()
}

/// Validate that an item ID is safe for URLs and deep links.
/// Only allows [A-Za-z0-9_-] characters.
fn validate_item_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("Item ID cannot be empty".to_string());
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!(
            "Item ID '{}' contains invalid characters. Only A-Z, a-z, 0-9, - and _ are allowed.",
            id
        ));
    }
    Ok(())
}

/// Add a new context menu item.
#[command]
pub async fn add_context_menu_item(item: ContextMenuItem) -> Result<ContextMenuItem, String> {
    validate_item_id(&item.id)?;
    let mut config = read_config()?;

    // Ensure unique ID
    if config.items.iter().any(|i| i.id == item.id) {
        return Err(format!("Item with id '{}' already exists", item.id));
    }

    let added = item.clone();
    config.items.push(item);
    write_config(&config)?;
    Ok(added)
}

/// Update an existing context menu item.
#[command]
pub async fn update_context_menu_item(
    id: String,
    item: ContextMenuItem,
) -> Result<ContextMenuItem, String> {
    validate_item_id(&id)?;
    let mut config = read_config()?;

    let existing = config.items.iter_mut().find(|i| i.id == id);
    match existing {
        Some(existing) => {
            existing.title = item.title;
            existing.icon = item.icon;
            existing.file_types = item.file_types;
            existing.action = item.action;
            existing.enabled = item.enabled;
            existing.order = item.order;
            existing.group = item.group;
            existing.root_level = item.root_level;
            existing.working_dir = item.working_dir;
            existing.show_window = item.show_window;
            existing.match_folders = item.match_folders;
            existing.match_background = item.match_background;
            existing.use_cmd = item.use_cmd;
            existing.icon_dark = item.icon_dark;
            existing.match_desktop = item.match_desktop;
            existing.match_drive = item.match_drive;
            existing.file_names = item.file_names;
            let updated = existing.clone();
            write_config(&config)?;
            Ok(updated)
        }
        None => Err(format!("Item with id '{}' not found", id)),
    }
}

/// Remove a context menu item.
#[command]
pub async fn remove_context_menu_item(id: String) -> Result<(), String> {
    let mut config = read_config()?;
    let original_len = config.items.len();
    config.items.retain(|i| i.id != id);

    if config.items.len() == original_len {
        return Err(format!("Item with id '{}' not found", id));
    }

    write_config(&config)
}

/// Toggle a context menu item's enabled state.
/// Refuses to enable items whose referenced entity no longer exists.
#[command]
pub async fn toggle_context_menu_item(
    id: String,
    enabled: bool,
    db_pool: tauri::State<'_, DbPool>,
) -> Result<(), String> {
    let mut config = read_config()?;

    let existing = config.items.iter_mut().find(|i| i.id == id);
    match existing {
        Some(item) => {
            if enabled && !item.action.is_empty() && item.action.starts_with('{') {
                let conn = &mut db_pool
                    .get()
                    .map_err(|e| format!("Database connection error: {}", e))?;
                if !action_entity_exists(conn, &item.action) {
                    return Err("Cannot enable: the assigned action references a deleted entity. Please assign a new action first.".to_string());
                }
            }
            item.enabled = enabled;
            write_config(&config)
        }
        None => Err(format!("Item with id '{}' not found", id)),
    }
}

/// Reorder context menu items.
#[command]
pub async fn reorder_context_menu_items(ids: Vec<String>) -> Result<(), String> {
    let mut config = read_config()?;

    // Assign order based on provided IDs
    for (order, id) in ids.iter().enumerate() {
        if let Some(item) = config.items.iter_mut().find(|i| &i.id == id) {
            item.order = order as i32;
        }
    }

    // Items not in the provided list get appended after the last provided ID
    let next_order = ids.len() as i32;
    for (i, item) in config.items.iter_mut().enumerate() {
        if !ids.contains(&item.id) {
            item.order = next_order + i as i32;
        }
    }

    config.items.sort_by_key(|i| i.order);
    write_config(&config)
}

/// Check if the shell extension is registered.
#[command]
pub async fn get_context_menu_status() -> Result<ContextMenuStatus, String> {
    let config_exists = config_path().map(|p| p.exists()).unwrap_or(false);

    // Single PowerShell call — dll_registered == sparse_package_registered
    let sparse_package_registered = check_sparse_package();

    Ok(ContextMenuStatus {
        config_exists,
        dll_registered: sparse_package_registered,
        sparse_package_registered,
        item_count: if config_exists {
            read_config().map(|c| c.items.len()).unwrap_or(0)
        } else {
            0
        },
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMenuStatus {
    pub config_exists: bool,
    pub dll_registered: bool,
    pub sparse_package_registered: bool,
    pub item_count: usize,
}

/// Strip the `\\?\` extended-length path prefix that Tauri's resource_dir() adds.
/// Add-AppxPackage rejects this prefix as an invalid URI.
fn clean_path(p: &std::path::Path) -> String {
    let s = p.to_string_lossy().to_string();
    s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
}

fn sparse_package_version(app: &tauri::AppHandle) -> String {
    let mut fields = app
        .package_info()
        .version
        .to_string()
        .split(|character: char| !character.is_ascii_digit())
        .filter(|field| !field.is_empty())
        .take(4)
        .map(|field| field.parse::<u32>().unwrap_or(0))
        .collect::<Vec<_>>();
    fields.resize(4, 0);
    format!("{}.{}.{}.{}", fields[0], fields[1], fields[2], fields[3])
}

fn powershell_single_quoted(value: &str) -> String {
    value.replace('\'', "''")
}

/// Register the shell extension sparse package.
/// The manifest's com:SurrogateServer handles COM registration automatically
/// (no regsvr32 needed). ExternalLocation points to the MSI-owned application
/// directory, while the generated manifest is staged in writable LocalAppData.
#[command]
pub async fn register_context_menu_extension(app: tauri::AppHandle) -> Result<(), String> {
    do_register_extension(&app)
}

fn do_register_extension(app: &tauri::AppHandle) -> Result<(), String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;

    let shell_ext_dir = resource_dir.join("resources").join("shell-ext");
    let dll_path = shell_ext_dir.join("vortideck-shell-ext.dll");
    if !dll_path.exists() {
        return Err("Shell extension DLL not found".to_string());
    }

    let icon_path = shell_ext_dir.join("icon.png");
    if !icon_path.exists() {
        return Err("Shell extension icon not found".to_string());
    }

    // The installed resource tree is immutable under Program Files. Only the
    // generated manifest belongs in per-user LocalAppData; ExternalLocation
    // remains the MSI-owned application directory containing the executable,
    // DLL, and icon referenced by the manifest.
    let package_version = sparse_package_version(app);
    let manifest_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get local application data dir: {}", e))?
        .join("shell-extension")
        .join(&package_version);
    std::fs::create_dir_all(&manifest_dir)
        .map_err(|e| format!("Failed to create shell extension staging directory: {}", e))?;
    let manifest_path = manifest_dir.join("AppxManifest.xml");

    // Count root-level items for slot verb generation in manifest
    let root_level_count = read_config()
        .map(|c| c.items.iter().filter(|i| i.root_level && i.enabled).count())
        .unwrap_or(0);
    let manifest_content = generate_manifest(root_level_count, &package_version);
    std::fs::write(&manifest_path, &manifest_content)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;

    let manifest_clean = clean_path(&manifest_path);
    let dir_clean = clean_path(&resource_dir);

    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;
    #[cfg(target_os = "windows")]
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    // Remove any existing package first to avoid "different external location" error
    // when the app was moved or rebuilt to a different directory.
    let mut clean_cmd = std::process::Command::new("powershell");
    #[cfg(target_os = "windows")]
    clean_cmd.creation_flags(CREATE_NO_WINDOW);
    let clean_output = clean_cmd
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$ErrorActionPreference='Stop'; Get-AppxPackage -Name 'VortiDeck.ShellExtension' | Remove-AppxPackage -ErrorAction Stop",
        ])
        .output()
        .map_err(|e| format!("Failed to remove the previous sparse package: {}", e))?;
    if !clean_output.status.success() {
        return Err(format!(
            "Failed to remove the previous sparse package: {}",
            String::from_utf8_lossy(&clean_output.stderr)
        ));
    }

    let ps_script = format!(
        "Add-AppxPackage -Register '{}' -ExternalLocation '{}'",
        powershell_single_quoted(&manifest_clean),
        powershell_single_quoted(&dir_clean)
    );

    let mut reg_cmd = std::process::Command::new("powershell");
    #[cfg(target_os = "windows")]
    reg_cmd.creation_flags(CREATE_NO_WINDOW);
    let output = reg_cmd
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
        .output()
        .map_err(|e| format!("Failed to run PowerShell: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to register sparse package: {}", stderr));
    }

    Ok(())
}

/// Auto-register the sparse package on startup if DLL exists but package isn't registered.
pub fn try_auto_register(app: &tauri::AppHandle) {
    if check_sparse_package_version(&sparse_package_version(app)) {
        return; // Already registered
    }
    if let Err(e) = do_register_extension(app) {
        log::warn!("Auto-registration of shell extension failed: {}", e);
    }
}

/// Unregister the shell extension sparse package.
/// This removes both the COM registration and context menu declarations.
#[command]
pub async fn unregister_context_menu_extension(app: tauri::AppHandle) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;
    #[cfg(target_os = "windows")]
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut cmd = std::process::Command::new("powershell");
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$ErrorActionPreference='Stop'; Get-AppxPackage -Name 'VortiDeck.ShellExtension' | Remove-AppxPackage -ErrorAction Stop",
        ])
        .output()
        .map_err(|e| format!("Failed to run PowerShell: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to unregister: {}", stderr));
    }

    if let Ok(local_data) = app.path().app_local_data_dir() {
        let _ = std::fs::remove_dir_all(local_data.join("shell-extension"));
    }

    Ok(())
}

/// Generate AppxManifest.xml dynamically based on how many root-level items are configured.
/// When root-level items exist, their slot CLSIDs are added as Verb entries so Windows
/// creates separate top-level context menu entries for them.
fn generate_manifest(root_level_count: usize, package_version: &str) -> String {
    let slot_clsids = [
        "D5EF25D0-8556-4838-97D9-53E265083794",
        "D5EF25D1-8556-4838-97D9-53E265083794",
        "D5EF25D2-8556-4838-97D9-53E265083794",
        "D5EF25D3-8556-4838-97D9-53E265083794",
        "D5EF25D4-8556-4838-97D9-53E265083794",
        "D5EF25D5-8556-4838-97D9-53E265083794",
        "D5EF25D6-8556-4838-97D9-53E265083794",
        "D5EF25D7-8556-4838-97D9-53E265083794",
    ];

    // Build slot verb entries for each valid ItemType section
    // Valid types per AppxManifest schema: *, .ext, Directory, Directory\Background
    // Desktop background is covered by Directory\Background, drive roots by Directory
    let mut slot_verbs_star = String::new();
    let mut slot_verbs_dir = String::new();
    let mut slot_verbs_bg = String::new();

    for i in 0..root_level_count.min(8) {
        slot_verbs_star.push_str(&format!(
            "\n              <desktop5:Verb Id=\"VDSlot{}\" Clsid=\"{}\" />",
            i, slot_clsids[i]
        ));
        slot_verbs_dir.push_str(&format!(
            "\n              <desktop5:Verb Id=\"VDSlot{}Dir\" Clsid=\"{}\" />",
            i, slot_clsids[i]
        ));
        slot_verbs_bg.push_str(&format!(
            "\n              <desktop5:Verb Id=\"VDSlot{}Bg\" Clsid=\"{}\" />",
            i, slot_clsids[i]
        ));
    }

    // Build com:Class entries for all 8 slots (always present so COM server is ready)
    let mut com_classes = String::new();
    for clsid in &slot_clsids {
        com_classes.push_str(&format!(
            "\n              <com:Class Id=\"{}\" Path=\"resources\\shell-ext\\vortideck-shell-ext.dll\" ThreadingModel=\"STA\"/>",
            clsid
        ));
    }

    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<Package
  xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
  xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
  xmlns:uap3="http://schemas.microsoft.com/appx/manifest/uap/windows10/3"
  xmlns:uap10="http://schemas.microsoft.com/appx/manifest/uap/windows10/10"
  xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
  xmlns:desktop4="http://schemas.microsoft.com/appx/manifest/desktop/windows10/4"
  xmlns:desktop5="http://schemas.microsoft.com/appx/manifest/desktop/windows10/5"
  xmlns:com="http://schemas.microsoft.com/appx/manifest/com/windows10"
  IgnorableNamespaces="uap uap3 uap10 rescap desktop4 desktop5 com">

  <Identity
    Name="VortiDeck.ShellExtension"
    Publisher="CN=VortiDeck"
    Version="{package_version}"
    ProcessorArchitecture="x64" />

  <Properties>
    <DisplayName>VortiDeck Shell Extension</DisplayName>
    <PublisherDisplayName>VortiDeck</PublisherDisplayName>
    <Description>VortiDeck Windows Explorer context menu integration</Description>
    <Logo>resources\shell-ext\icon.png</Logo>
    <uap10:AllowExternalContent>true</uap10:AllowExternalContent>
  </Properties>

  <Resources>
    <Resource Language="en-us" />
  </Resources>

  <Dependencies>
    <TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.19041.0" MaxVersionTested="10.0.26200.0" />
  </Dependencies>

  <Capabilities>
    <rescap:Capability Name="runFullTrust" />
  </Capabilities>

  <Applications>
    <Application Id="VortiDeck" Executable="VortiDeck.exe"
      uap10:TrustLevel="mediumIL"
      uap10:RuntimeBehavior="win32App">
      <uap:VisualElements
        DisplayName="VortiDeck"
        Description="VortiDeck - Your Ultimate Deck Controller"
        BackgroundColor="transparent"
        Square150x150Logo="resources\shell-ext\icon.png"
        Square44x44Logo="resources\shell-ext\icon.png"
        AppListEntry="none" />
      <Extensions>
        <com:Extension Category="windows.comServer">
          <com:ComServer>
            <com:SurrogateServer DisplayName="VortiDeck Context Menu Handler">
              <com:Class Id="D5EF25C7-8556-4838-97D9-53E265083794" Path="resources\shell-ext\vortideck-shell-ext.dll" ThreadingModel="STA"/>{com_classes}
            </com:SurrogateServer>
          </com:ComServer>
        </com:Extension>
        <desktop4:Extension Category="windows.fileExplorerContextMenus">
          <desktop4:FileExplorerContextMenus>
            <desktop5:ItemType Type="*">
              <desktop5:Verb Id="VortiDeckMenu" Clsid="D5EF25C7-8556-4838-97D9-53E265083794" />{slot_verbs_star}
            </desktop5:ItemType>
            <desktop5:ItemType Type="Directory">
              <desktop5:Verb Id="VortiDeckMenuDir" Clsid="D5EF25C7-8556-4838-97D9-53E265083794" />{slot_verbs_dir}
            </desktop5:ItemType>
            <desktop5:ItemType Type="Directory\Background">
              <desktop5:Verb Id="VortiDeckMenuBg" Clsid="D5EF25C7-8556-4838-97D9-53E265083794" />{slot_verbs_bg}
            </desktop5:ItemType>
          </desktop4:FileExplorerContextMenus>
        </desktop4:Extension>
      </Extensions>
    </Application>
  </Applications>
</Package>
"#,
        com_classes = com_classes,
        package_version = package_version,
        slot_verbs_star = slot_verbs_star,
        slot_verbs_dir = slot_verbs_dir,
        slot_verbs_bg = slot_verbs_bg,
    )
}

fn check_dll_registered() -> bool {
    // With manifest-based COM, the sparse package registration handles everything.
    // If the sparse package is registered, the DLL is registered too.
    check_sparse_package()
}

fn check_sparse_package() -> bool {
    #[cfg(target_os = "windows")]
    {
        // Direct registry check — instant vs ~2s PowerShell spawn.
        // When a sparse package is registered, Windows creates an entry under
        // PackagedCom\Package\{PackageFullName}. We look for any key starting
        // with "VortiDeck.ShellExtension_".
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey(r"Software\Classes\PackagedCom\Package") {
            for name in key.enum_keys().filter_map(|k| k.ok()) {
                if name.starts_with("VortiDeck.ShellExtension_") {
                    return true;
                }
            }
        }
        false
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

fn check_sparse_package_version(version: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;
        let expected_prefix = format!("VortiDeck.ShellExtension_{}_", version);
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey(r"Software\Classes\PackagedCom\Package") {
            return key
                .enum_keys()
                .filter_map(|entry| entry.ok())
                .any(|name| name.starts_with(&expected_prefix));
        }
        false
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = version;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::{generate_manifest, powershell_single_quoted};

    #[test]
    fn generated_manifest_uses_installed_resource_paths_and_requested_version() {
        let manifest = generate_manifest(1, "9.8.7.6");

        assert!(manifest.contains("Version=\"9.8.7.6\""));
        assert!(manifest.contains("Path=\"resources\\shell-ext\\vortideck-shell-ext.dll\""));
        assert!(manifest.contains("Square44x44Logo=\"resources\\shell-ext\\icon.png\""));
        assert!(manifest.contains("Id=\"VDSlot0\""));
        assert!(!manifest.contains("Id=\"VDSlot1\""));
    }

    #[test]
    fn generated_manifest_caps_root_level_slots_at_eight() {
        let manifest = generate_manifest(99, "1.2.3.4");

        assert!(manifest.contains("Id=\"VDSlot7\""));
        assert!(!manifest.contains("Id=\"VDSlot8\""));
    }

    #[test]
    fn powershell_single_quotes_are_escaped() {
        assert_eq!(powershell_single_quoted("C:\\It's here"), "C:\\It''s here");
    }
}
