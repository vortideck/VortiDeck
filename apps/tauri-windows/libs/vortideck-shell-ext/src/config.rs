use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    /// Empty string or absent means the item appears directly under "VortiDeck".
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
    /// Optional icon path for dark mode. If non-empty, used when Windows is in dark theme.
    #[serde(default)]
    pub icon_dark: String,
    /// If true, show this item when right-clicking the Windows desktop background.
    #[serde(default)]
    pub match_desktop: bool,
    /// If true, show this item when right-clicking a drive root (C:\, D:\, etc.).
    #[serde(default)]
    pub match_drive: bool,
    /// Optional file name patterns. If non-empty, files must match one of these patterns.
    /// Supports exact names ("Makefile") and wildcards ("*.config", "readme*").
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
            items: vec![ContextMenuItem {
                id: "open-assistant".to_string(),
                title: "Open with VortiDeck Assistant".to_string(),
                icon: String::new(),
                file_types: vec!["*".to_string()],
                action: "vortideck://context-menu/open-assistant".to_string(),
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
        }
    }
}

/// Returns the path to the context-menus.json config file.
/// Located at %APPDATA%/VortiDeck/context-menus.json
pub fn config_path() -> Option<PathBuf> {
    std::env::var("APPDATA")
        .ok()
        .map(|appdata| PathBuf::from(appdata).join("VortiDeck").join("context-menus.json"))
}

/// Reads the context menu config from disk.
/// Returns the default config if the file doesn't exist or can't be parsed.
pub fn read_config() -> ContextMenuConfig {
    let path = match config_path() {
        Some(p) => p,
        None => return ContextMenuConfig::default(),
    };

    match std::fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => ContextMenuConfig::default(),
    }
}

/// Checks if a file extension matches the configured file types for an item.
/// Supports "*" for all files, and specific extensions like ".png", ".jpg".
pub fn matches_file_type(item: &ContextMenuItem, file_extension: &str) -> bool {
    if item.file_types.iter().any(|ft| ft == "*") {
        return true;
    }
    let ext_lower = file_extension.to_lowercase();
    item.file_types
        .iter()
        .any(|ft| ft.to_lowercase() == ext_lower)
}

/// Checks if a file name matches the configured name patterns for an item.
/// Supports exact names ("Makefile"), suffix wildcards ("*.config"), and prefix wildcards ("readme*").
pub fn matches_file_name(item: &ContextMenuItem, file_name: &str) -> bool {
    if item.file_names.is_empty() {
        return true; // No name filter — everything matches
    }
    let name_lower = file_name.to_lowercase();
    item.file_names.iter().any(|pattern| {
        let pat_lower = pattern.to_lowercase();
        if let Some(suffix) = pat_lower.strip_prefix('*') {
            name_lower.ends_with(suffix)
        } else if let Some(prefix) = pat_lower.strip_suffix('*') {
            name_lower.starts_with(prefix)
        } else {
            name_lower == pat_lower
        }
    })
}

/// Returns the root-level item assigned to a given slot index (0-7).
/// Root-level items are sorted by `order` and mapped to slots sequentially.
/// Returns None if the slot has no assigned item.
pub fn get_root_slot_item(slot: u8) -> Option<ContextMenuItem> {
    let config = read_config();
    let mut root_items: Vec<ContextMenuItem> = config
        .items
        .into_iter()
        .filter(|i| i.root_level && i.enabled)
        .collect();
    root_items.sort_by_key(|i| i.order);
    root_items.into_iter().nth(slot as usize)
}
