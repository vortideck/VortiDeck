use async_trait::async_trait;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use vortideck_core::integrations::interface::EventCallback;

pub struct TauriEventEmitter {
    app_handle: Option<tauri::AppHandle>,
}

impl TauriEventEmitter {
    pub fn new() -> Self {
        Self { app_handle: None }
    }

    pub fn set_app_handle(&mut self, handle: tauri::AppHandle) {
        self.app_handle = Some(handle);
    }
}

#[async_trait]
impl EventCallback for TauriEventEmitter {
    async fn on_event(
        &self,
        event: &str,
        data: serde_json::Value,
    ) -> vortideck_core::integrations::interface::IntegrationResult<()> {
        if let Some(app) = &self.app_handle {
            app.emit(event, data).map_err(|e| {
                vortideck_core::integrations::traits::IntegrationError::Error(e.to_string())
            })?;
            Ok(())
        } else {
            Err(
                vortideck_core::integrations::traits::IntegrationError::Error(
                    "App handle not set".to_string(),
                ),
            )
        }
    }
}
