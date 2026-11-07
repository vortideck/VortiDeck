pub mod processor;
pub mod storage;

#[cfg(feature = "icons-server")]
pub mod server;

pub use processor::IconProcessor;
pub use storage::IconStorage;

#[cfg(feature = "icons-server")]
pub use server::IconServer;

/// Icons service combining processing and storage
/// This is the main service interface for icon management
pub struct IconService {
    processor: IconProcessor,
    storage: IconStorage,
}

impl IconService {
    pub fn new(storage_path: std::path::PathBuf) -> Self {
        Self {
            processor: IconProcessor::new(),
            storage: IconStorage::new(storage_path),
        }
    }

    /// Process and save an icon from base64 data
    pub async fn save_icon(&self, button_id: &str, base64_image: &str) -> Result<(), String> {
        // Process the image
        let processed_data = self.processor.process_base64_to_webp(base64_image).await?;

        // Store the processed image
        self.storage.save_icon(button_id, &processed_data).await
    }

    /// Remove an icon and its directory
    pub async fn remove_icon(&self, button_id: &str) -> Result<(), String> {
        self.storage.remove_icon(button_id).await
    }

    /// Get the path to an icon file
    pub fn get_icon_path(&self, button_id: &str) -> std::path::PathBuf {
        self.storage.get_icon_path(button_id)
    }

    /// Get storage statistics
    pub fn get_storage_stats(
        &self,
    ) -> Result<crate::services::icons::storage::IconStorageStats, String> {
        self.storage.get_storage_stats()
    }

    /// Get the base icons directory for server usage
    pub fn get_icons_base_dir(&self) -> std::path::PathBuf {
        self.storage.get_icons_base_dir()
    }
}
