use async_trait::async_trait;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use vortideck_core::integrations::EventEmitter;

/// Tauri implementation of EventEmitter for universal integration status updates
pub struct TauriEventEmitter {
    app_handle: AppHandle,
}

impl TauriEventEmitter {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

#[async_trait]
impl EventEmitter for TauriEventEmitter {
    async fn emit(
        &self,
        event: &str,
        payload: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Emit once globally so the main window and any auxiliary editor
        // windows receive the same lifecycle transition without duplicates.
        self.app_handle
            .emit(event, payload)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        Ok(())
    }
}

/// Helper function to create a Tauri EventEmitter
pub fn create_tauri_event_emitter(app_handle: AppHandle) -> Arc<dyn EventEmitter> {
    Arc::new(TauriEventEmitter::new(app_handle))
}
