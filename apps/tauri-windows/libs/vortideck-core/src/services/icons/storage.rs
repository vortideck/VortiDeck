use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use tokio::fs::remove_dir_all;

/// Handles file system operations for icon storage
pub struct IconStorage {
    base_path: PathBuf,
}

impl IconStorage {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    /// Save processed icon data to file system
    pub async fn save_icon(&self, button_id: &str, webp_data: &[u8]) -> Result<(), String> {
        let button_id = button_id.to_string();
        let base_path = self.base_path.clone();
        let webp_data = webp_data.to_vec();

        // Offload file I/O to background thread
        tokio::task::spawn_blocking(move || {
            // Create directory structure: base_path/icons/button_id/
            let save_dir = base_path.join("icons").join(&button_id);
            create_dir_all(&save_dir).map_err(|e| format!("Error creating directory: {}", e))?;

            // Save as icon.webp
            let save_path = save_dir.join("icon.webp");
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&save_path)
                .map_err(|e| format!("Error opening file to save: {}", e))?;

            file.write_all(&webp_data)
                .map_err(|e| format!("Error writing WebP data: {}", e))?;

            Ok(())
        })
        .await
        .map_err(|e| format!("Error executing file save task: {}", e))?
    }

    /// Remove icon directory and all contents
    pub async fn remove_icon(&self, button_id: &str) -> Result<(), String> {
        let icon_dir = self.base_path.join("icons").join(button_id);

        if icon_dir.exists() {
            remove_dir_all(&icon_dir)
                .await
                .map_err(|e| format!("Error removing icon directory: {}", e))?;
        }

        Ok(())
    }

    /// Get the full path to an icon file
    pub fn get_icon_path(&self, button_id: &str) -> PathBuf {
        self.base_path
            .join("icons")
            .join(button_id)
            .join("icon.webp")
    }

    /// Get the directory path for a button's icons
    pub fn get_icon_dir(&self, button_id: &str) -> PathBuf {
        self.base_path.join("icons").join(button_id)
    }

    /// Get the base icons directory
    pub fn get_icons_base_dir(&self) -> PathBuf {
        self.base_path.join("icons")
    }

    /// Get storage statistics
    pub fn get_storage_stats(&self) -> Result<IconStorageStats, String> {
        let icons_dir = self.get_icons_base_dir();

        if !icons_dir.exists() {
            return Ok(IconStorageStats {
                total_icons: 0,
                total_size_bytes: 0,
                button_ids: Vec::new(),
            });
        }

        // Directly collect button IDs by checking directories with icon files
        let mut button_ids = Vec::new();
        for entry in std::fs::read_dir(icons_dir)
            .map_err(|e| format!("Error reading icons directory: {}", e))?
        {
            let entry = entry.map_err(|e| format!("Error reading directory entry: {}", e))?;

            if entry
                .file_type()
                .map_err(|e| format!("Error getting file type: {}", e))?
                .is_dir()
            {
                if let Some(button_id) = entry.file_name().to_str() {
                    // Check if this directory contains an icon file
                    let icon_path = self.get_icon_path(button_id);
                    if icon_path.exists() {
                        button_ids.push(button_id.to_string());
                    }
                }
            }
        }

        let total_icons = button_ids.len();

        // Calculate total size
        let mut total_size_bytes = 0u64;
        for button_id in &button_ids {
            let icon_path = self.get_icon_path(button_id);
            if let Ok(metadata) = std::fs::metadata(icon_path) {
                total_size_bytes += metadata.len();
            }
        }

        Ok(IconStorageStats {
            total_icons,
            total_size_bytes,
            button_ids,
        })
    }
}

/// Statistics about icon storage
#[derive(Debug, Clone)]
pub struct IconStorageStats {
    pub total_icons: usize,
    pub total_size_bytes: u64,
    pub button_ids: Vec<String>,
}
