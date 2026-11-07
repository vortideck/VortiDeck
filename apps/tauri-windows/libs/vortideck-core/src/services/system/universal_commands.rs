use crate::core::{PlatformContext, PlatformError, SystemAction, SystemInfo};
use chrono;
use serde_json::Value;
use std::sync::Arc;

/// Universal system commands that work across all platforms
pub struct UniversalSystemCommands {
    platform_context: Arc<PlatformContext>,
}

impl UniversalSystemCommands {
    pub fn new(platform_context: Arc<PlatformContext>) -> Self {
        Self { platform_context }
    }

    /// Set system volume (platform-agnostic)
    pub async fn set_volume(
        &self,
        volume: f32,
        button_id: Option<String>,
    ) -> Result<String, PlatformError> {
        // Validate volume range
        if !(0.0..=100.0).contains(&volume) {
            return Err(PlatformError::ValidationError(
                "Volume must be between 0 and 100".to_string(),
            ));
        }

        // Execute platform-specific volume control
        let result = self
            .execute_volume_control(volume, button_id.clone())
            .await?;

        // Emit event to notify UI via EventBus
        let event_payload =
            crate::events::EventPayload::System(crate::events::SystemEventPayload::VolumeChanged {
                volume,
                button_id,
                timestamp: chrono::Utc::now().timestamp(),
            });

        self.platform_context.event_bus.emit(event_payload).ok();

        Ok(result)
    }

    /// Get system information
    pub async fn get_system_info(&self) -> Result<SystemInfo, PlatformError> {
        let system_info = self
            .platform_context
            .system_interface
            .get_system_info()
            .await?;

        // Emit event with system info via EventBus
        let event_payload = crate::events::EventPayload::System(
            crate::events::SystemEventPayload::SystemInfoRetrieved {
                system_info: system_info.clone(),
            },
        );

        self.platform_context.event_bus.emit(event_payload).ok();

        Ok(system_info)
    }

    /// Get available system actions
    pub async fn get_available_actions(&self) -> Result<Vec<SystemAction>, PlatformError> {
        let actions = self
            .platform_context
            .system_interface
            .get_available_actions()
            .await?;

        // Note: Volume callback registration is handled by the adapter layer
        // The core library doesn't need to register callbacks directly

        Ok(actions)
    }

    /// Execute a system action by ID
    pub async fn execute_system_action(
        &self,
        action_id: &str,
        params: Value,
    ) -> Result<Value, PlatformError> {
        // Execute the action based on ID
        match action_id {
            "system.volume.set" => {
                let volume = params
                    .get("volume")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| {
                        PlatformError::ValidationError("Missing volume parameter".to_string())
                    })?;

                let button_id = params
                    .get("button_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let result = self.set_volume(volume as f32, button_id).await?;
                Ok(Value::String(result))
            }
            "system.info.get" => {
                let info = self.get_system_info().await?;
                serde_json::to_value(info).map_err(|e| {
                    PlatformError::ValidationError(format!(
                        "Failed to serialize system info: {}",
                        e
                    ))
                })
            }
            "system.command.execute" => {
                let command = params
                    .get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        PlatformError::ValidationError("Missing command parameter".to_string())
                    })?;

                let args = params
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect::<Vec<String>>()
                    })
                    .unwrap_or_default();

                let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                let result = self
                    .platform_context
                    .system_interface
                    .execute_command(command, &args_refs)
                    .await?;

                serde_json::to_value(result).map_err(|e| {
                    PlatformError::ValidationError(format!(
                        "Failed to serialize command result: {}",
                        e
                    ))
                })
            }
            _ => Err(PlatformError::ValidationError(format!(
                "Unknown action: {}",
                action_id
            ))),
        }
    }

    /// Platform-specific volume control implementation
    async fn execute_volume_control(
        &self,
        volume: f32,
        button_id: Option<String>,
    ) -> Result<String, PlatformError> {
        #[cfg(target_os = "windows")]
        {
            // 🚀 PERFORMANCE FIX: Use direct Windows API instead of PowerShell for real-time volume control
            use crate::services::system::SystemControl;

            match SystemControl::set_volume_static(volume, button_id).await {
                Ok(result) => Ok(result),
                Err(e) => Err(PlatformError::SystemError(format!(
                    "Failed to set volume: {}",
                    e
                ))),
            }
        }

        #[cfg(target_os = "macos")]
        {
            // macOS-specific volume control
            let volume_level = (volume / 100.0 * 7.0).round() as i32;
            let result = self
                .platform_context
                .system_interface
                .execute_command(
                    "osascript",
                    &["-e", &format!("set volume output volume {}", volume_level)],
                )
                .await?;

            if result.exit_code == 0 {
                Ok(format!("Volume set to {}%", volume))
            } else {
                Err(PlatformError::SystemError(format!(
                    "Failed to set volume: {}",
                    result.stderr
                )))
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Linux-specific volume control (using amixer)
            let result = self
                .platform_context
                .system_interface
                .execute_command("amixer", &["sset", "Master", &format!("{}%", volume)])
                .await?;

            if result.exit_code == 0 {
                Ok(format!("Volume set to {}%", volume))
            } else {
                Err(PlatformError::SystemError(format!(
                    "Failed to set volume: {}",
                    result.stderr
                )))
            }
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            // Fallback for other platforms
            Ok(format!("Volume set to {}% (simulated)", volume))
        }
    }

    /// Get platform information for debugging
    pub fn get_platform_info(&self) -> std::collections::HashMap<String, String> {
        self.platform_context.get_platform_info()
    }
}

/// Factory function to create universal system commands with platform context
pub fn create_universal_system_commands(
    platform_context: Arc<PlatformContext>,
) -> UniversalSystemCommands {
    UniversalSystemCommands::new(platform_context)
}

/// Convenience functions for common operations
pub async fn set_volume_universal(
    platform_context: &PlatformContext,
    volume: f32,
    button_id: Option<String>,
) -> Result<String, PlatformError> {
    let commands = UniversalSystemCommands::new(Arc::new(platform_context.clone()));
    commands.set_volume(volume, button_id).await
}

pub async fn get_system_info_universal(
    platform_context: &PlatformContext,
) -> Result<SystemInfo, PlatformError> {
    let commands = UniversalSystemCommands::new(Arc::new(platform_context.clone()));
    commands.get_system_info().await
}

pub async fn get_available_actions_universal(
    platform_context: &PlatformContext,
) -> Result<Vec<SystemAction>, PlatformError> {
    let commands = UniversalSystemCommands::new(Arc::new(platform_context.clone()));
    commands.get_available_actions().await
}

pub async fn execute_system_action_universal(
    platform_context: &PlatformContext,
    action_id: &str,
    params: Value,
) -> Result<Value, PlatformError> {
    let commands = UniversalSystemCommands::new(Arc::new(platform_context.clone()));
    commands.execute_system_action(action_id, params).await
}
