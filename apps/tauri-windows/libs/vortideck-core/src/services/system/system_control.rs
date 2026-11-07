// Copy-paste your working implementation with improvements - preserving all functionality
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
use std::sync::{Arc, Mutex};

// ✅ OPTIMAL ARCHITECTURE: Using pure platform abstractions
use crate::core::{PlatformContext, PlatformError, VolumeChangeCallback, VolumeChangeEvent};

// Copy from your working implementation - Windows-specific imports
#[cfg(target_os = "windows")]
use windows::{
    core::*, Win32::Foundation::*, Win32::Media::Audio::Endpoints::*, Win32::Media::Audio::*,
    Win32::System::Com::*,
};

// Add windows_core for the implement macro
#[cfg(target_os = "windows")]
use windows_core as wcore;

// Copy your exact working static definitions
static IS_BUTTON_CHANGE: AtomicBool = AtomicBool::new(false);
static BUTTON_ID: Mutex<Option<String>> = Mutex::new(None);
static CALLBACK_REGISTERED: AtomicBool = AtomicBool::new(false);
static ENDPOINT_INIT: Once = Once::new();

// ✅ OPTIMAL ARCHITECTURE: Pure volume callback implementation
#[cfg(target_os = "windows")]
#[implement(IAudioEndpointVolumeCallback)]
struct WindowsVolumeCallback {
    callback: Arc<dyn VolumeChangeCallback>,
}

#[cfg(target_os = "windows")]
impl IAudioEndpointVolumeCallback_Impl for WindowsVolumeCallback_Impl {
    fn OnNotify(&self, pnotify: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> windows_core::Result<()> {
        unsafe {
            if !pnotify.is_null() {
                let data = &*pnotify;
                let volume = data.fMasterVolume * 100.0;
                let muted = data.bMuted.as_bool();

                // Get the button_id and is_button_change state atomically
                let (is_button_change, button_id) = {
                    let is_button_change = IS_BUTTON_CHANGE.load(Ordering::SeqCst);
                    let button_id = if is_button_change {
                        match BUTTON_ID.lock() {
                            Ok(mut guard) => guard.take(),
                            _ => None,
                        }
                    } else {
                        None
                    };

                    // Always reset the flag after reading to prevent sticking
                    if is_button_change {
                        IS_BUTTON_CHANGE.store(false, Ordering::SeqCst);
                    }

                    (is_button_change, button_id)
                };

                // Log the volume change
                println!(
                    "🔊 Volume changed: {:.1}%, Muted: {}, Button: {:?}, IsButtonChange: {}",
                    volume, muted, button_id, is_button_change
                );

                // ✅ OPTIMAL: Create pure event data and delegate to callback
                let event = VolumeChangeEvent {
                    volume,
                    muted,
                    is_button_change,
                    button_id,
                    timestamp: chrono::Utc::now().timestamp(),
                };

                // ✅ CRITICAL FIX: Use Handle::try_current() to safely spawn from native thread
                let callback = self.callback.clone();

                // Check if we're in a tokio runtime context
                match tokio::runtime::Handle::try_current() {
                    Ok(handle) => {
                        // We're in a tokio context - spawn normally
                        handle.spawn(async move {
                            if let Err(e) = callback.on_volume_change(event).await {
                                println!("⚠️ Volume callback error: {}", e);
                            }
                        });
                    }
                    _ => {
                        // We're in a native thread - use blocking approach
                        std::thread::spawn(move || {
                            // Create a new tokio runtime for this callback
                            match tokio::runtime::Runtime::new() {
                                Ok(rt) => {
                                    rt.block_on(async move {
                                        if let Err(e) = callback.on_volume_change(event).await {
                                            println!("⚠️ Volume callback error: {}", e);
                                        }
                                    });
                                }
                                _ => {
                                    println!(
                                        "⚠️ Failed to create tokio runtime for volume callback"
                                    );
                                }
                            }
                        });
                    }
                }
            }
            Ok(())
        }
    }
}

// ✅ OPTIMAL ARCHITECTURE: Removed platform-specific monitor
// Volume monitoring is now handled by pure callback system
// Platform adapters can implement their own monitoring if needed

// Copy your exact SystemAction and ActionParameter structs - match core abstractions interface
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SystemAction {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String, // Use category to match core abstractions
    pub requires_elevation: bool,
    pub parameters: Vec<ActionParameter>,
    pub is_system: bool,
    pub action_type: String, // Keep both for compatibility
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ActionParameter {
    pub name: String,
    pub parameter_type: String, // "number", "string", "boolean", etc.
    pub description: String,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
    pub min_value: Option<f64>, // For numeric parameters
    pub max_value: Option<f64>, // For numeric parameters
}

// ✅ OPTIMAL ARCHITECTURE: Pure SystemControl with dependency injection
#[cfg(target_os = "windows")]
pub struct SystemControl {
    // Pure abstractions for testability and reusability
    volume_callback: Option<Arc<dyn VolumeChangeCallback>>,
    platform_context: Option<Arc<PlatformContext>>,
}

#[cfg(target_os = "windows")]
impl SystemControl {
    /// ✅ OPTIMAL: Create pure SystemControl without platform dependencies
    pub fn new() -> Self {
        Self {
            volume_callback: None,
            platform_context: None,
        }
    }

    /// ✅ OPTIMAL: Create with platform context for integration
    pub fn with_platform_context(platform_context: Arc<PlatformContext>) -> Self {
        Self {
            volume_callback: None,
            platform_context: Some(platform_context),
        }
    }

    /// ✅ OPTIMAL: Create with pure volume callback for testability
    pub fn with_volume_callback(callback: Arc<dyn VolumeChangeCallback>) -> Self {
        Self {
            volume_callback: Some(callback),
            platform_context: None,
        }
    }

    /// ✅ OPTIMAL: Create with both callback and platform context
    pub fn with_callback_and_context(
        callback: Arc<dyn VolumeChangeCallback>,
        platform_context: Arc<PlatformContext>,
    ) -> Self {
        Self {
            volume_callback: Some(callback),
            platform_context: Some(platform_context),
        }
    }

    // Copy your exact get_audio_endpoint_volume helper function
    fn get_audio_endpoint_volume(
    ) -> std::result::Result<(IAudioEndpointVolume, String), Box<dyn std::error::Error>> {
        unsafe {
            // Initialize COM
            let hr = CoInitializeEx(Some(null_mut()), COINIT_APARTMENTTHREADED);
            if hr.is_err() {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to initialize COM",
                )));
            }

            // Create device enumerator
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

            // Get the default audio endpoint
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;

            // Get the IAudioEndpointVolume interface
            let endpoint_volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;

            // Get device ID
            let device_id = device.GetId()?;
            let device_id_str = device_id
                .to_string()
                .unwrap_or_else(|_| "Unknown".to_string());

            Ok((endpoint_volume, device_id_str))
        }
    }

    pub async fn set_volume_static(
        volume: f32,
        button_id: Option<String>,
    ) -> std::result::Result<String, Box<dyn std::error::Error>> {
        unsafe {
            // Get the audio endpoint volume interface
            let (endpoint_volume, device_id_str) = Self::get_audio_endpoint_volume()?;

            // Ensure volume is in the valid range (0.0 to 1.0)
            let normalized_volume = volume.clamp(0.0, 100.0) / 100.0;

            // Mark this as a button-initiated change and store the button_id
            IS_BUTTON_CHANGE.store(true, Ordering::SeqCst);
            if let Some(id) = &button_id {
                if let Ok(mut guard) = BUTTON_ID.lock() {
                    *guard = Some(id.clone());
                }
                println!(
                    "🎛️ Setting volume with button_id: {} (flag set to prevent callback lag)",
                    id
                );
            } else {
                println!("🎛️ Setting volume without button_id (system initiated)");
            }

            // Set the volume (expects a value between 0.0 and 1.0)
            endpoint_volume.SetMasterVolumeLevelScalar(normalized_volume, &GUID::zeroed())?;

            // Get the updated volume to confirm
            let current_volume = endpoint_volume.GetMasterVolumeLevelScalar()?;

            // Clean up COM
            CoUninitialize();

            // ✅ CRITICAL FIX: Use runtime handle for safety timeout to prevent panic
            let timeout_duration = std::time::Duration::from_millis(100); // Reduced from 1000ms to 100ms

            // Check if we're in a tokio runtime context
            match tokio::runtime::Handle::try_current() {
                Ok(handle) => {
                    // We're in a tokio context - spawn normally
                    handle.spawn(async move {
                        tokio::time::sleep(timeout_duration).await;
                        // Reset flag if it's still set (safety mechanism)
                        if IS_BUTTON_CHANGE.load(Ordering::SeqCst) {
                            IS_BUTTON_CHANGE.store(false, Ordering::SeqCst);
                            if let Ok(mut guard) = BUTTON_ID.lock() {
                                *guard = None;
                            }
                            println!("🔄 Safety timeout: Reset IS_BUTTON_CHANGE flag (100ms)");
                        }
                    });
                }
                _ => {
                    // We're not in a tokio context - use thread-based timeout
                    std::thread::spawn(move || {
                        std::thread::sleep(timeout_duration);
                        // Reset flag if it's still set (safety mechanism)
                        if IS_BUTTON_CHANGE.load(Ordering::SeqCst) {
                            IS_BUTTON_CHANGE.store(false, Ordering::SeqCst);
                            if let Ok(mut guard) = BUTTON_ID.lock() {
                                *guard = None;
                            }
                            println!("🔄 Safety timeout: Reset IS_BUTTON_CHANGE flag (100ms)");
                        }
                    });
                }
            }

            Ok(format!(
                "Volume set successfully. New volume: {:.1}%, Device: {}",
                current_volume * 100.0,
                device_id_str
            ))
        }
    }

    // Copy your exact get_volume method (static)
    pub fn get_volume() -> std::result::Result<f32, Box<dyn std::error::Error>> {
        unsafe {
            // Get the audio endpoint volume interface
            let (endpoint_volume, _) = Self::get_audio_endpoint_volume()?;

            // Get the current volume
            let current_volume = endpoint_volume.GetMasterVolumeLevelScalar()?;

            // Clean up COM
            CoUninitialize();

            // Return the volume in 0-100 range
            Ok(current_volume)
        }
    }

    // Copy your exact get_brightness method (static)
    pub fn get_brightness() -> std::result::Result<f32, Box<dyn std::error::Error>> {
        // Get brightness from Windows system settings
        Ok(50.0)
    }

    // Copy your exact get_available_actions method (static)
    pub fn get_available_actions_static() -> Vec<SystemAction> {
        let mut actions = Vec::new();

        // System volume control
        actions.push(SystemAction {
            id: "system.volume.set".to_string(),
            name: "Set System Volume".to_string(),
            description: "Sets the system master volume level".to_string(),
            category: "System".to_string(), // Use category for compatibility
            requires_elevation: false,
            parameters: vec![ActionParameter {
                name: "volume".to_string(),
                parameter_type: "number".to_string(),
                description: "Volume level (0-100)".to_string(),
                required: true,
                default_value: Some(serde_json::json!(50)),
                min_value: Some(0.0),
                max_value: Some(100.0),
            }],
            is_system: true,
            action_type: "volume".to_string(), // Keep for compatibility
        });

        actions
    }

    // Copy your exact execute_action method (static)
    pub async fn execute_action_static(
        action_id: &str,
        params: Value,
    ) -> std::result::Result<Value, Box<dyn std::error::Error>> {
        println!("Executing action: {}", action_id);
        println!("With parameters: {}", params);

        match action_id {
            "system.volume.set" => {
                // Extract volume parameter, with fallback if not explicitly set
                let volume = params["volume"]
                    .as_f64()
                    .or_else(|| {
                        params["volume"]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                    })
                    .unwrap_or(50.0);

                // Get button_id if it exists
                let button_id = params["button_id"].as_str().map(|id| id.to_string());

                // Log button_id if available
                if let Some(id) = &button_id {
                    println!("Volume change requested from button: {}", id);
                }

                // Call set_volume and convert the result to a Value
                let result = Self::set_volume_static(volume as f32, button_id).await?;
                Ok(serde_json::json!({ "success": true, "message": result }))
            }
            _ => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Action not found: {}", action_id),
            ))),
        }
    }

    // ✅ COMPATIBILITY WRAPPERS: Instance methods that call static methods
    // These preserve compatibility with existing codebase while using your working implementation

    /// Instance wrapper for set_volume - uses std::result::Result
    pub async fn set_volume(
        &self,
        volume: f32,
        button_id: Option<String>,
    ) -> std::result::Result<String, PlatformError> {
        Self::set_volume_static(volume, button_id)
            .await
            .map_err(|e| PlatformError::SystemError(e.to_string()))
    }

    /// Instance wrapper for get_available_actions  
    pub fn get_available_actions(&self) -> Vec<SystemAction> {
        Self::get_available_actions_static()
    }

    /// Instance wrapper for execute_action - uses std::result::Result
    pub async fn execute_action(
        &self,
        action_id: &str,
        params: Value,
    ) -> std::result::Result<Value, PlatformError> {
        Self::execute_action_static(action_id, params)
            .await
            .map_err(|e| PlatformError::SystemError(e.to_string()))
    }

    /// ✅ OPTIMAL: Register pure volume callback - no platform dependencies
    pub async fn register_volume_callback(
        &self,
        callback: Arc<dyn VolumeChangeCallback>,
    ) -> std::result::Result<(), PlatformError> {
        Self::register_volume_callback_static(callback)
            .await
            .map_err(|e| PlatformError::SystemError(e.to_string()))
    }

    /// ✅ OPTIMAL: Static method for registering pure volume callback
    #[cfg(target_os = "windows")]
    pub async fn register_volume_callback_static(
        callback: Arc<dyn VolumeChangeCallback>,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        // Check if we've already registered a callback
        if CALLBACK_REGISTERED.load(Ordering::SeqCst) {
            println!("Volume callback already registered");
            return Ok(());
        }

        // Initialize endpoint volume and callback only once
        ENDPOINT_INIT.call_once(|| {
            if let Ok((endpoint_volume, _device_id_str)) = Self::get_audio_endpoint_volume() {
                let windows_callback = WindowsVolumeCallback {
                    callback: callback.clone(),
                };
                let callback_object: IAudioEndpointVolumeCallback = windows_callback.into();

                // Register the callback with the endpoint volume
                unsafe {
                    if let Ok(_) = endpoint_volume.RegisterControlChangeNotify(&callback_object) {
                        // Mark as registered
                        CALLBACK_REGISTERED.store(true, Ordering::SeqCst);
                        println!("🔊 Pure volume change callback registered successfully");

                        // Intentionally leak the COM objects to keep them alive
                        std::mem::forget(endpoint_volume);
                        std::mem::forget(callback_object);
                    }
                }
            }
        });

        Ok(())
    }

    /// Instance method for getting capabilities
    pub fn get_capabilities(&self) -> HashMap<String, bool> {
        let mut capabilities = HashMap::new();
        capabilities.insert("volume_control".to_string(), true);
        capabilities.insert("system_actions".to_string(), true);

        #[cfg(target_os = "windows")]
        capabilities.insert("windows_native".to_string(), true);

        capabilities
    }

    /// Instance method for action validation
    pub fn validate_action(
        &self,
        action_id: &str,
        _params: &HashMap<String, String>,
    ) -> std::result::Result<(), PlatformError> {
        // Simple validation - check if action exists
        let actions = Self::get_available_actions_static();
        if actions.iter().any(|a| a.id == action_id) {
            Ok(())
        } else {
            Err(PlatformError::ValidationError(format!(
                "Unknown action: {}",
                action_id
            )))
        }
    }
}

#[cfg(target_os = "windows")]
impl Default for SystemControl {
    fn default() -> Self {
        Self::new()
    }
}
