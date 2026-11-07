use std::path::Path;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Registry::*;
use windows::Win32::UI::Shell::*;

use crate::config::{self, ContextMenuItem};
use crate::dll_release;
use crate::sub_commands::VortiDeckEnumCommands;

/// The kind of command this represents.
#[derive(Clone)]
pub enum CommandKind {
    /// Root "VortiDeck" menu (enumerates all items flat)
    Root,
    /// A root-level slot (0-7) that appears directly in the context menu
    RootSlot(u8),
    /// A leaf action item (clickable)
    Leaf(ContextMenuItem),
    /// A visual separator line between groups
    Separator,
}

#[implement(IExplorerCommand)]
pub struct VortiDeckCommand {
    kind: CommandKind,
}

impl VortiDeckCommand {
    pub fn new_root() -> Self {
        Self {
            kind: CommandKind::Root,
        }
    }

    pub fn new_root_slot(slot: u8) -> Self {
        Self {
            kind: CommandKind::RootSlot(slot),
        }
    }

    pub fn new_sub(item: ContextMenuItem) -> Self {
        Self {
            kind: CommandKind::Leaf(item),
        }
    }

    pub fn new_separator() -> Self {
        Self {
            kind: CommandKind::Separator,
        }
    }

}

impl Drop for VortiDeckCommand {
    fn drop(&mut self) {
        dll_release();
    }
}

/// Allocate a PWSTR using CoTaskMemAlloc so the caller (Explorer) can free it.
fn alloc_pwstr(s: &str) -> Result<PWSTR> {
    unsafe {
        let wide: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
        let byte_len = wide.len() * std::mem::size_of::<u16>();
        let ptr = CoTaskMemAlloc(byte_len) as *mut u16;
        if ptr.is_null() {
            return Err(Error::new(E_OUTOFMEMORY, "CoTaskMemAlloc failed"));
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
        Ok(PWSTR(ptr))
    }
}

/// Check if a path is a drive root (e.g., "C:\", "D:\").
fn is_drive_root(path: &str) -> bool {
    let bytes = path.as_bytes();
    let len = bytes.len();
    (len == 2 || (len == 3 && bytes[2] == b'\\'))
        && bytes.first().map_or(false, |b| b.is_ascii_alphabetic())
        && bytes.get(1) == Some(&b':')
}

/// Detect if Windows is currently using a dark app theme.
/// Reads HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize\AppsUseLightTheme.
/// Returns true for dark mode (value == 0), false for light mode (value == 1 or missing).
fn is_windows_dark_mode() -> bool {
    unsafe {
        let mut hkey = HKEY::default();
        let subkey = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
        let result = RegOpenKeyExW(HKEY_CURRENT_USER, subkey, Some(0), KEY_READ, &mut hkey);
        if result.is_err() {
            return false;
        }
        let value_name = w!("AppsUseLightTheme");
        let mut data: u32 = 1;
        let mut data_size: u32 = std::mem::size_of::<u32>() as u32;
        let result = RegQueryValueExW(
            hkey,
            value_name,
            None,
            None,
            Some(&mut data as *mut u32 as *mut u8),
            Some(&mut data_size),
        );
        let _ = RegCloseKey(hkey);
        if result.is_err() {
            return false;
        }
        data == 0 // 0 = dark mode, 1 = light mode
    }
}

/// Helper: get the state for a leaf-like item given the shell item array.
fn get_leaf_state(item: &ContextMenuItem, psiitemarray: &Ref<'_, IShellItemArray>) -> u32 {
    if !item.enabled {
        return ECS_HIDDEN.0 as u32;
    }

    if let Some(shell_items) = psiitemarray.as_ref() {
        unsafe {
            let count = shell_items.GetCount().unwrap_or(0);

            // Background click: no items selected
            if count == 0 {
                return if item.match_background || item.match_desktop {
                    ECS_ENABLED.0 as u32
                } else {
                    ECS_HIDDEN.0 as u32
                };
            }

            if let Ok(item_ref) = shell_items.GetItemAt(0) {
                if let Ok(name) = item_ref.GetDisplayName(SIGDN_FILESYSPATH) {
                    let path_str = name.to_string().unwrap_or_default();
                    CoTaskMemFree(Some(name.0 as *const _));

                    let path = Path::new(&path_str);

                    // Drive root target (e.g., "C:\") — check match_drive.
                    if is_drive_root(&path_str) {
                        return if item.match_drive {
                            ECS_ENABLED.0 as u32
                        } else {
                            ECS_HIDDEN.0 as u32
                        };
                    }

                    // Folder target — only visible when match_folders is explicitly set.
                    // file_types wildcard ("*") means "all file types", not "folders too".
                    if path.is_dir() {
                        return if item.match_folders {
                            ECS_ENABLED.0 as u32
                        } else {
                            ECS_HIDDEN.0 as u32
                        };
                    }

                    // File target — check file name patterns first (if configured)
                    let file_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");

                    // If file_names is non-empty and matches, show regardless of file_types
                    if !item.file_names.is_empty() && config::matches_file_name(item, file_name) {
                        return ECS_ENABLED.0 as u32;
                    }

                    // Check extension via file_types
                    if item.file_types.iter().any(|ft| ft == "*") {
                        // Wildcard matches all files, but still must pass file_names filter
                        // (if file_names is empty, matches_file_name returns true)
                        return if config::matches_file_name(item, file_name) {
                            ECS_ENABLED.0 as u32
                        } else {
                            ECS_HIDDEN.0 as u32
                        };
                    }
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| format!(".{}", e))
                        .unwrap_or_default();
                    if config::matches_file_type(item, &ext) {
                        return if config::matches_file_name(item, file_name) {
                            ECS_ENABLED.0 as u32
                        } else {
                            ECS_HIDDEN.0 as u32
                        };
                    }
                    return ECS_HIDDEN.0 as u32;
                }
            }
        }
    } else {
        // No shell item array at all — background context
        if item.match_background || item.match_desktop {
            return ECS_ENABLED.0 as u32;
        }
    }

    ECS_HIDDEN.0 as u32
}

/// Helper: invoke a leaf-like item (collect paths, launch VortiDeck with deep link).
fn invoke_leaf(item: &ContextMenuItem, psiitemarray: &Ref<'_, IShellItemArray>) -> Result<()> {
    let config = config::read_config();
    let vortideck_path = config.vortideck_path;

    if vortideck_path.is_empty() {
        return Err(Error::new(E_FAIL, "VortiDeck path not configured"));
    }

    let mut paths = Vec::new();
    if let Some(shell_items) = psiitemarray.as_ref() {
        unsafe {
            let count = shell_items.GetCount().unwrap_or(0);
            for i in 0..count {
                if let Ok(shell_item) = shell_items.GetItemAt(i) {
                    if let Ok(name) = shell_item.GetDisplayName(SIGDN_FILESYSPATH) {
                        let path_str = name.to_string().unwrap_or_default();
                        CoTaskMemFree(Some(name.0 as *const _));
                        if !path_str.is_empty() {
                            paths.push(path_str);
                        }
                    }
                }
            }
        }
    }

    let paths_param = paths.join("|");
    let encoded_paths = url_encode(&paths_param);
    // Deep link uses item ID only — Rust looks up the stored action and executes it
    let deep_link = format!("vortideck://context-menu/item-{}?paths={}", item.id, encoded_paths);

    launch_vortideck(&vortideck_path, &deep_link)?;
    Ok(())
}

impl IExplorerCommand_Impl for VortiDeckCommand_Impl {
    fn GetTitle(&self, _psiitemarray: Ref<'_, IShellItemArray>) -> Result<PWSTR> {
        match &self.kind {
            CommandKind::Root => alloc_pwstr("VortiDeck"),
            CommandKind::RootSlot(slot) => {
                match config::get_root_slot_item(*slot) {
                    Some(item) => alloc_pwstr(&item.title),
                    None => Err(Error::new(E_NOTIMPL, "")),
                }
            }
            CommandKind::Leaf(item) => alloc_pwstr(&item.title),
            CommandKind::Separator => Err(Error::new(E_NOTIMPL, "")),
        }
    }

    fn GetIcon(&self, _psiitemarray: Ref<'_, IShellItemArray>) -> Result<PWSTR> {
        // Return Err(E_NOTIMPL) for "no icon" - returning Ok(PWSTR::null())
        // causes Explorer to crash trying to dereference the null pointer.
        match &self.kind {
            CommandKind::Root => {
                // Prefer icon.ico next to the DLL (always available),
                // fall back to extracting icon from VortiDeck.exe.
                if let Some(dir) = crate::dll_dir() {
                    if !dir.is_empty() {
                        let ico = format!("{}\\icon.ico", dir);
                        if Path::new(&ico).exists() {
                            alloc_pwstr(&ico)
                        } else {
                            let config = config::read_config();
                            if !config.vortideck_path.is_empty() && Path::new(&config.vortideck_path).exists() {
                                let icon = format!("{},0", config.vortideck_path);
                                alloc_pwstr(&icon)
                            } else {
                                Err(Error::new(E_NOTIMPL, ""))
                            }
                        }
                    } else {
                        Err(Error::new(E_NOTIMPL, ""))
                    }
                } else {
                    Err(Error::new(E_NOTIMPL, ""))
                }
            }
            CommandKind::RootSlot(slot) => {
                match config::get_root_slot_item(*slot) {
                    Some(item) => {
                        let dark = is_windows_dark_mode();
                        if dark && !item.icon_dark.is_empty() {
                            alloc_pwstr(&item.icon_dark)
                        } else if !item.icon.is_empty() {
                            alloc_pwstr(&item.icon)
                        } else {
                            Err(Error::new(E_NOTIMPL, ""))
                        }
                    }
                    _ => Err(Error::new(E_NOTIMPL, "")),
                }
            }
            CommandKind::Leaf(item) => {
                let dark = is_windows_dark_mode();
                if dark && !item.icon_dark.is_empty() {
                    alloc_pwstr(&item.icon_dark)
                } else if !item.icon.is_empty() {
                    alloc_pwstr(&item.icon)
                } else {
                    Err(Error::new(E_NOTIMPL, ""))
                }
            }
            CommandKind::Separator => Err(Error::new(E_NOTIMPL, "")),
        }
    }

    fn GetToolTip(&self, _psiitemarray: Ref<'_, IShellItemArray>) -> Result<PWSTR> {
        Err(Error::new(E_NOTIMPL, ""))
    }

    fn GetCanonicalName(&self) -> Result<GUID> {
        Ok(GUID::zeroed())
    }

    fn GetState(
        &self,
        psiitemarray: Ref<'_, IShellItemArray>,
        _fokaliased: BOOL,
    ) -> Result<u32> {
        match &self.kind {
            CommandKind::Root => {
                // Hide root menu if no sub-items would be visible for this selection
                let config = config::read_config();
                let has_visible = config.items.iter().any(|item| {
                    item.enabled && get_leaf_state(item, &psiitemarray) == ECS_ENABLED.0 as u32
                });
                Ok(if has_visible { ECS_ENABLED.0 as u32 } else { ECS_HIDDEN.0 as u32 })
            }
            CommandKind::Separator => Ok(ECS_ENABLED.0 as u32),
            CommandKind::RootSlot(slot) => {
                match config::get_root_slot_item(*slot) {
                    Some(item) => Ok(get_leaf_state(&item, &psiitemarray)),
                    None => Ok(ECS_HIDDEN.0 as u32),
                }
            }
            CommandKind::Leaf(item) => Ok(get_leaf_state(item, &psiitemarray)),
        }
    }

    fn GetFlags(&self) -> Result<u32> {
        match &self.kind {
            CommandKind::Root => Ok(ECF_HASSUBCOMMANDS.0 as u32),
            CommandKind::Separator => Ok(ECF_ISSEPARATOR.0 as u32),
            CommandKind::RootSlot(_) | CommandKind::Leaf(_) => Ok(ECF_DEFAULT.0 as u32),
        }
    }

    fn EnumSubCommands(&self) -> Result<IEnumExplorerCommand> {
        match &self.kind {
            CommandKind::Root => {
                let config = config::read_config();
                let enabled_items: Vec<ContextMenuItem> = config
                    .items
                    .into_iter()
                    .filter(|i| i.enabled)
                    .collect();

                // Collect unique groups (preserving order of first appearance)
                let mut seen_groups: Vec<String> = Vec::new();
                for item in &enabled_items {
                    if !item.group.is_empty() && !seen_groups.contains(&item.group) {
                        seen_groups.push(item.group.clone());
                    }
                }

                // Flatten: groups become sections separated by ECF_ISSEPARATOR items
                let mut flat: Vec<TopLevelEntry> = Vec::new();

                for group_name in &seen_groups {
                    let mut group_items: Vec<ContextMenuItem> = enabled_items
                        .iter()
                        .filter(|i| &i.group == group_name)
                        .cloned()
                        .collect();
                    group_items.sort_by_key(|i| i.order);

                    // Add separator before each group (if not the first entry)
                    if !flat.is_empty() {
                        flat.push(TopLevelEntry::Separator);
                    }

                    for item in group_items {
                        flat.push(TopLevelEntry::Item(item));
                    }
                }

                // Add ungrouped items
                let mut ungrouped: Vec<ContextMenuItem> = enabled_items
                    .into_iter()
                    .filter(|i| i.group.is_empty())
                    .collect();
                ungrouped.sort_by_key(|i| i.order);

                if !ungrouped.is_empty() && !flat.is_empty() {
                    flat.push(TopLevelEntry::Separator);
                }

                for item in ungrouped {
                    flat.push(TopLevelEntry::Item(item));
                }

                let enum_commands = VortiDeckEnumCommands::new_mixed(flat);
                Ok(enum_commands.into())
            }
            _ => {
                Err(Error::new(E_NOTIMPL, "only root has sub-commands"))
            }
        }
    }

    fn Invoke(
        &self,
        psiitemarray: Ref<'_, IShellItemArray>,
        _pbc: Ref<'_, IBindCtx>,
    ) -> Result<()> {
        match &self.kind {
            CommandKind::Leaf(item) => invoke_leaf(item, &psiitemarray),
            CommandKind::RootSlot(slot) => {
                match config::get_root_slot_item(*slot) {
                    Some(item) => invoke_leaf(&item, &psiitemarray),
                    None => Ok(()),
                }
            }
            _ => Ok(()),
        }
    }
}

/// Represents a top-level entry under the root "VortiDeck" menu.
#[derive(Clone)]
pub enum TopLevelEntry {
    Item(ContextMenuItem),
    Separator,
}

/// Simple URL encoding for file paths.
fn url_encode(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}

fn launch_vortideck(exe_path: &str, deep_link: &str) -> Result<()> {
    use std::process::Command;

    Command::new(exe_path)
        .arg(deep_link)
        .spawn()
        .map_err(|e| Error::new(E_FAIL, format!("Failed to launch VortiDeck: {}", e)))?;

    Ok(())
}
