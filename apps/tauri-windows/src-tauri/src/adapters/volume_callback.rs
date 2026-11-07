// Direct volume callback adapter for Tauri (no factory pattern)
// Simple, direct implementation that connects Windows volume changes to Tauri events

use async_trait::async_trait;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use vortideck_core::core::abstractions::{PlatformError, VolumeChangeCallback, VolumeChangeEvent};
use vortideck_core::core::StateManager;
use vortideck_core::services::system::system_control::SystemControl;

/// Direct volume callback implementation for Tauri
/// No factory pattern - just a simple callback that emits events
pub struct TauriVolumeCallback {
    app_handle: AppHandle,
    state_manager: Arc<StateManager>,
}

impl TauriVolumeCallback {
    pub fn new(app_handle: AppHandle, state_manager: Arc<StateManager>) -> Self {
        Self {
            app_handle,
            state_manager,
        }
    }
}

#[async_trait]
impl VolumeChangeCallback for TauriVolumeCallback {
    async fn on_volume_change(&self, event: VolumeChangeEvent) -> Result<(), PlatformError> {
        log::debug!(
            "🔊 Volume change received: {}%, muted: {}, button_change: {}, button_id: {:?}",
            event.volume,
            event.muted,
            event.is_button_change,
            event.button_id
        );

        // Direct event emission to Tauri frontend
        let volume_data = serde_json::json!({
            "volume": event.volume,
            "muted": event.muted,
            "is_button_change": event.is_button_change,
            "button_id": event.button_id.clone(),
            "timestamp": event.timestamp
        });

        // Emit to both mobile simulator and general listeners
        if let Err(e) =
            self.app_handle
                .emit_to("mobile-simulator", "system_volume_changed", &volume_data)
        {
            log::warn!("Failed to emit volume change to mobile-simulator: {}", e);
        }

        if let Err(e) = self.app_handle.emit("system_volume_changed", &volume_data) {
            log::warn!("Failed to emit volume change globally: {}", e);
        }

        // 🔄 Anti-recursion: Only broadcast to mobile if change DIDN'T originate from button/mobile
        // If is_button_change = true, it means volume was changed via button or mobile command
        // In that case, don't send it back to mobile to avoid infinite loops
        if !event.is_button_change {
            // Broadcast to mobile clients using StateManager
            self.state_manager
                .broadcast_volume_change_to_mobile(
                    event.volume,
                    event.muted,
                    event.is_button_change,
                    event.button_id,
                )
                .await;
            log::debug!("✅ Volume change broadcasted to mobile clients");
        } else {
            log::debug!(
                "🔄 Skipped mobile broadcast - volume change originated from button/mobile (anti-recursion)"
            );
        }

        Ok(())
    }

    fn callback_type(&self) -> &'static str {
        "TauriVolumeCallback"
    }
}

/// Direct volume callback registration function
/// No factory pattern - just register the callback with Windows system
pub async fn register_volume_callback(
    app_handle: AppHandle,
    state_manager: Arc<StateManager>,
) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("🔊 Registering volume callback with Windows system");

    let callback = TauriVolumeCallback::new(app_handle, state_manager);

    match SystemControl::register_volume_callback_static(Arc::new(callback)).await {
        Ok(_) => {
            log::info!("✅ Volume callback registered successfully");
            Ok(())
        }
        Err(e) => {
            log::error!("❌ Failed to register volume callback: {}", e);
            Err(e)
        }
    }
}
