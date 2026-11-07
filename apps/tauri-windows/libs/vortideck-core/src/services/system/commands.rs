use crate::core::{PlatformContext, PlatformError, SystemAction};
#[cfg(target_os = "windows")]
use crate::services::system::SystemControl;
use serde_json::Value;
use std::sync::Arc;

/// 🚀 UNIVERSAL ARCHITECTURE: Pure business logic system commands
/// All platform interactions go through PlatformContext

/// Set system volume using universal SystemControl
#[cfg(target_os = "windows")]
pub async fn set_volume(
    platform_context: &PlatformContext,
    volume: f32,
    button_id: Option<String>,
) -> Result<String, PlatformError> {
    let system_control = SystemControl::with_platform_context(Arc::new(platform_context.clone()));
    system_control.set_volume(volume, button_id).await
}

#[cfg(not(target_os = "windows"))]
pub async fn set_volume(
    _platform_context: &PlatformContext,
    _volume: f32,
    _button_id: Option<String>,
) -> Result<String, PlatformError> {
    Err(PlatformError::UnsupportedPlatform(
        "System control not available on this platform".to_string(),
    ))
}

/// Get available system actions using universal SystemControl
#[cfg(target_os = "windows")]
pub async fn get_available_actions(
    platform_context: &PlatformContext,
) -> Result<Vec<SystemAction>, PlatformError> {
    let system_control = SystemControl::with_platform_context(Arc::new(platform_context.clone()));

    // Note: Volume callback registration is handled by the adapter layer
    // The core library doesn't need to register callbacks directly

    // Convert SystemControlAction to SystemAction
    let control_actions = system_control.get_available_actions();
    let system_actions = control_actions
        .into_iter()
        .map(|action| SystemAction {
            id: action.id.clone(),
            name: action.name,
            description: action.description,
            category: action.category.clone(),
            parameters: action
                .parameters
                .into_iter()
                .map(|param| crate::core::ActionParameter {
                    name: param.name,
                    parameter_type: param.parameter_type,
                    description: param.description,
                    required: param.required,
                    default_value: param.default_value,
                    min_value: param.min_value,
                    max_value: param.max_value,
                })
                .collect(),
            requires_elevation: action.requires_elevation,
            is_system: action.is_system,
            action_type: action.action_type,
        })
        .collect();

    Ok(system_actions)
}

#[cfg(not(target_os = "windows"))]
pub async fn get_available_actions(
    _platform_context: &PlatformContext,
) -> Result<Vec<SystemAction>, PlatformError> {
    Ok(Vec::new()) // Return empty actions on non-Windows platforms
}

/// Execute system action using universal SystemControl
#[cfg(target_os = "windows")]
pub async fn execute_system_action(
    platform_context: &PlatformContext,
    action_id: &str,
    params: Value,
) -> Result<Value, PlatformError> {
    let system_control = SystemControl::with_platform_context(Arc::new(platform_context.clone()));
    system_control.execute_action(action_id, params).await
}

#[cfg(not(target_os = "windows"))]
pub async fn execute_system_action(
    _platform_context: &PlatformContext,
    _action_id: &str,
    _params: Value,
) -> Result<Value, PlatformError> {
    Err(PlatformError::UnsupportedPlatform(
        "System action execution not available on this platform".to_string(),
    ))
}

/// Universal system service that encapsulates all system operations
#[cfg(target_os = "windows")]
pub struct UniversalSystemService {
    platform_context: Arc<PlatformContext>,
    system_control: SystemControl,
}

#[cfg(not(target_os = "windows"))]
pub struct UniversalSystemService {
    platform_context: Arc<PlatformContext>,
}

#[cfg(target_os = "windows")]
impl UniversalSystemService {
    pub fn new(platform_context: Arc<PlatformContext>) -> Self {
        let system_control = SystemControl::with_platform_context(platform_context.clone());
        Self {
            platform_context,
            system_control,
        }
    }

    /// Set volume with full error handling and event emission
    pub async fn set_volume(
        &self,
        volume: f32,
        button_id: Option<String>,
    ) -> Result<String, PlatformError> {
        self.system_control.set_volume(volume, button_id).await
    }

    /// Get all available system actions
    pub async fn get_available_actions(&self) -> Result<Vec<SystemAction>, PlatformError> {
        get_available_actions(&self.platform_context).await
    }

    /// Execute any system action by ID
    pub async fn execute_action(
        &self,
        action_id: &str,
        params: Value,
    ) -> Result<Value, PlatformError> {
        self.system_control.execute_action(action_id, params).await
    }

    /// Get system capabilities
    pub fn get_capabilities(&self) -> std::collections::HashMap<String, bool> {
        self.system_control.get_capabilities()
    }

    /// Validate action parameters before execution
    pub fn validate_action(
        &self,
        action_id: &str,
        params: &std::collections::HashMap<String, String>,
    ) -> Result<(), PlatformError> {
        self.system_control.validate_action(action_id, params)
    }
}

#[cfg(not(target_os = "windows"))]
impl UniversalSystemService {
    pub fn new(platform_context: Arc<PlatformContext>) -> Self {
        Self { platform_context }
    }

    /// Set volume with full error handling and event emission
    pub async fn set_volume(
        &self,
        _volume: f32,
        _button_id: Option<String>,
    ) -> Result<String, PlatformError> {
        Err(PlatformError::UnsupportedPlatform(
            "System control not available on this platform".to_string(),
        ))
    }

    /// Get all available system actions
    pub async fn get_available_actions(&self) -> Result<Vec<SystemAction>, PlatformError> {
        get_available_actions(&self.platform_context).await
    }

    /// Execute any system action by ID
    pub async fn execute_action(
        &self,
        _action_id: &str,
        _params: Value,
    ) -> Result<Value, PlatformError> {
        Err(PlatformError::UnsupportedPlatform(
            "System action execution not available on this platform".to_string(),
        ))
    }

    /// Get system capabilities
    pub fn get_capabilities(&self) -> std::collections::HashMap<String, bool> {
        std::collections::HashMap::new()
    }

    /// Validate action parameters before execution
    pub fn validate_action(
        &self,
        _action_id: &str,
        _params: &std::collections::HashMap<String, String>,
    ) -> Result<(), PlatformError> {
        Err(PlatformError::UnsupportedPlatform(
            "Action validation not available on this platform".to_string(),
        ))
    }
}
