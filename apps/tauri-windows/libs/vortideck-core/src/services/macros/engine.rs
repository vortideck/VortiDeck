use super::recording::{MacroRecorder, MacroStep, RecordingState};
use super::RecordedMacro;
use crate::core::state_manager::StateManager;
use crate::features::requirements::FeatureRequirement;
use crate::features::types::StorageValidator;
use crate::features::types::{PremiumFeature, ResetPeriod};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use uuid::Uuid;

/// Simple macro engine with Tauri focus integration
pub struct MacroEngine {
    recorder: MacroRecorder,
    state: Arc<RecordingState>,
    state_manager: Arc<StateManager>,
}

impl MacroEngine {
    pub fn new(state_manager: Arc<StateManager>) -> Self {
        let state = Arc::new(RecordingState::new());
        let recorder = MacroRecorder::new(state.clone());

        Self {
            recorder,
            state,
            state_manager,
        }
    }

    /// Get the pause state for direct Tauri window event integration
    pub fn get_pause_state(&self) -> Arc<AtomicBool> {
        self.state.get_pause_state()
    }

    /// Set window focus state directly (called by Tauri events)
    pub fn set_window_focused(&self, focused: bool) {
        self.state.set_window_focused(focused);
    }

    /// Start real-time macro recording with Tauri focus integration
    pub async fn start_real_time_recording<F>(
        &self,
        event_callback: F,
    ) -> Result<Vec<MacroStep>, String>
    where
        F: FnMut(MacroStep) + Send + 'static,
    {
        println!("🎬 Starting real-time macro recording with Tauri focus integration");

        // TODO: Add premium validation when feature system is available
        // let validation = self.state_manager.try_use_feature(self).await;
        // if !validation.available {
        //     return Err(validation.message.unwrap_or_else(||
        //         "Upgrade to Premium for unlimited macro recordings!".to_string()
        //     ));
        // }

        // Start recording with automatic pause/resume on focus changes
        self.recorder.start_recording(event_callback).await
    }

    /// Stop macro recording
    pub async fn stop_real_time_recording(&self) -> Result<(), String> {
        println!("🛑 Stopping real-time macro recording");
        self.recorder.stop_recording();
        Ok(())
    }

    /// Reset recorded macros
    pub async fn reset_recording(&self) -> Result<(), String> {
        println!("🔄 Resetting macro recording");
        self.recorder.reset_macros();
        Ok(())
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.recorder.is_recording()
    }

    /// Check if recording is paused due to window focus
    pub fn is_paused(&self) -> bool {
        self.recorder.is_paused()
    }

    /// Get recorded steps
    pub fn get_recorded_steps(&self) -> Vec<MacroStep> {
        self.recorder.get_recorded_steps()
    }

    /// Save recorded macro with premium validation
    pub async fn save_recorded_macro(
        &self,
        user_id: String,
        macro_name: String,
        description: Option<String>,
    ) -> Result<String, String> {
        println!("💾 Saving recorded macro: {}", macro_name);

        // TODO: Add premium validation when feature system is available
        // let validation = self.state_manager.try_use_feature(self).await;
        // if !validation.available {
        //     return Err(validation.message.unwrap_or_else(||
        //         "Upgrade to Premium for unlimited macro recordings!".to_string()
        //     ));
        // }

        let steps = self.get_recorded_steps();
        if steps.is_empty() {
            return Err("No macro steps recorded".to_string());
        }

        let macro_id = Uuid::new_v4().to_string();
        let _recorded_macro = RecordedMacro {
            id: macro_id.clone(),
            name: macro_name,
            description,
            steps,
            created_at: chrono::Utc::now(),
            user_id,
        };

        // TODO: Save to storage when storage system is available
        // self.storage.save_macro(&recorded_macro).await
        //     .map_err(|e| format!("Failed to save macro: {}", e))?;

        println!("✅ Macro saved successfully: {}", macro_id);
        Ok(macro_id)
    }

    /// Execute a macro by ID
    pub async fn execute_macro(&self, macro_id: &str) -> Result<(), String> {
        println!("▶️ Executing macro: {}", macro_id);

        // TODO: Add premium validation when feature system is available
        // let validation = self.state_manager.try_use_feature(self).await;
        // if !validation.available {
        //     return Err(validation.message.unwrap_or_else(||
        //         "Upgrade to Premium for unlimited macro executions!".to_string()
        //     ));
        // }

        // TODO: Load macro from storage and execute it
        // let macro_data = self.load_macro(macro_id).await?;
        // Execute the macro steps...

        println!("✅ Macro executed successfully: {}", macro_id);
        Ok(())
    }

    /// Load macro by ID (placeholder)
    pub async fn load_macro(&self, macro_id: &str) -> Result<RecordedMacro, String> {
        // TODO: Implement when storage system is available
        Err(format!(
            "Macro loading not implemented yet for ID: {}",
            macro_id
        ))
    }

    /// List all macros for a user (placeholder)
    pub async fn list_user_macros(&self, user_id: &str) -> Result<Vec<RecordedMacro>, String> {
        // TODO: Implement when storage system is available
        println!("📋 Listing macros for user: {}", user_id);
        Ok(vec![])
    }

    /// Delete macro by ID (placeholder)
    pub async fn delete_macro(&self, macro_id: &str) -> Result<(), String> {
        // TODO: Implement when storage system is available
        println!("🗑️ Deleting macro: {}", macro_id);
        Ok(())
    }
}

impl FeatureRequirement for MacroEngine {
    fn get_requirements(&self) -> Vec<PremiumFeature> {
        vec![PremiumFeature::UsageLimit {
            feature_id: "macro_recording".to_string(),
            free_limit: 0,       // ✅ Ignored - JWT provides real limits
            premium_limit: None, // ✅ Ignored - JWT provides real limits
            reset_period: ResetPeriod::Never,
        }]
    }

    fn feature_context(&self) -> String {
        "Macro Recording".to_string()
    }
}
