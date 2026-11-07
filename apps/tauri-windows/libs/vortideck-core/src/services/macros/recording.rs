use crate::services::execution::UniversalAction;
use device_query::{DeviceQuery, DeviceState, Keycode};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::time::Duration;

/// Represents a single step in a macro recording
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MacroStep {
    pub action: ActionExecutable,
    pub delay: u64, // Delay in milliseconds before this step
}

/// Represents different types of actions that can be recorded in a macro
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ActionExecutable {
    KeyDown(String),
    KeyUp(String),
    MouseMove { x: i32, y: i32 },
    MouseClick(String),
    ObsAction(String),
    SystemControl(String),
}

impl ActionExecutable {
    /// Convert legacy ActionExecutable to UniversalAction for execution
    pub fn to_universal_action(&self) -> UniversalAction {
        use crate::services::execution::{KeyAction, MouseActionType, MouseButton, TimingConfig};
        use std::collections::HashMap;

        match self {
            ActionExecutable::KeyDown(key) => UniversalAction::KeySequence {
                keys: vec![KeyAction::Press(key.clone())],
                timing: TimingConfig {
                    delay_before_ms: 0,
                    delay_after_ms: 0,
                    key_press_duration_ms: 10,
                },
            },
            ActionExecutable::KeyUp(key) => UniversalAction::KeySequence {
                keys: vec![KeyAction::Release(key.clone())],
                timing: TimingConfig {
                    delay_before_ms: 0,
                    delay_after_ms: 0,
                    key_press_duration_ms: 10,
                },
            },
            ActionExecutable::MouseMove { x, y } => UniversalAction::MouseAction {
                action_type: MouseActionType::Move,
                coordinates: Some((*x, *y)),
                button: None,
            },
            ActionExecutable::MouseClick(button) => UniversalAction::MouseAction {
                action_type: MouseActionType::Click,
                coordinates: None,
                button: Some(match button.as_str() {
                    "left" => MouseButton::Left,
                    "right" => MouseButton::Right,
                    "middle" => MouseButton::Middle,
                    _ => MouseButton::Left,
                }),
            },
            ActionExecutable::ObsAction(action) => UniversalAction::Integration {
                integration_id: "obs".to_string(),
                action_id: action.clone(),
                parameters: HashMap::new(),
            },
            ActionExecutable::SystemControl(control) => UniversalAction::SystemControl {
                command: control.clone(),
                parameters: None,
            },
        }
    }
}

/// Simple recording state with Tauri focus integration
pub struct RecordingState {
    is_recording: RwLock<Option<bool>>,
    is_paused: Arc<AtomicBool>, // Updated directly by Tauri window events
    macros: RwLock<Vec<MacroStep>>,
    session_id: RwLock<Option<String>>,
}

impl Default for RecordingState {
    fn default() -> Self {
        RecordingState {
            is_recording: RwLock::new(Some(false)),
            is_paused: Arc::new(AtomicBool::new(false)), // Start focused
            macros: RwLock::new(Vec::new()),
            session_id: RwLock::new(None),
        }
    }
}

impl RecordingState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_recording_state(&self, value: bool) {
        if let Ok(mut is_recording) = self.is_recording.write() {
            *is_recording = Some(value);
        }
    }

    pub fn get_recording_state(&self) -> Option<bool> {
        match self.is_recording.read() {
            Ok(is_recording) => *is_recording,
            _ => None,
        }
    }

    /// Check if recording is currently paused due to window focus
    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::Relaxed)
    }

    /// Update pause state directly from Tauri window events
    pub fn set_window_focused(&self, focused: bool) {
        let should_pause = !focused;
        let was_paused = self.is_paused.swap(should_pause, Ordering::Relaxed);

        // Only show pause/resume messages if recording is actually active
        if was_paused != should_pause && self.get_recording_state().unwrap_or(false) {
            println!(
                "🎬 Recording {}",
                if should_pause {
                    "paused (window unfocused)"
                } else {
                    "resumed (window focused)"
                }
            );
        }
    }

    /// Get the pause state reference for Tauri event setup
    pub fn get_pause_state(&self) -> Arc<AtomicBool> {
        self.is_paused.clone()
    }

    pub fn reset_macros(&self) {
        if let Ok(mut macros) = self.macros.write() {
            macros.clear();
        }
    }

    pub fn get_macros(&self) -> Vec<MacroStep> {
        match self.macros.read() {
            Ok(macros) => macros.clone(),
            _ => Vec::new(),
        }
    }

    pub fn add_macro_step(&self, step: &MacroStep) {
        // Only add steps if recording is active and not paused
        if let (Some(true), false) = (self.get_recording_state(), self.is_paused()) {
            if let Ok(mut macros) = self.macros.write() {
                macros.push(step.clone());
            }
        }
    }

    pub fn set_session_id(&self, session_id: Option<String>) {
        if let Ok(mut session) = self.session_id.write() {
            *session = session_id;
        }
    }

    pub fn get_session_id(&self) -> Option<String> {
        match self.session_id.read() {
            Ok(session) => session.clone(),
            _ => None,
        }
    }

    pub fn display_macros(&self) -> String {
        let macros = self.get_macros();
        if macros.is_empty() {
            "No macros recorded".to_string()
        } else {
            let mut result = String::new();
            for (i, macro_step) in macros.iter().enumerate() {
                result.push_str(&format!(
                    "Step {}: {:?} (delay: {}ms)\n",
                    i + 1,
                    macro_step.action,
                    macro_step.delay
                ));
            }
            result
        }
    }
}

/// Simple macro recorder with Tauri focus integration
pub struct MacroRecorder {
    state: Arc<RecordingState>,
}

impl MacroRecorder {
    pub fn new(state: Arc<RecordingState>) -> Self {
        Self { state }
    }

    /// Start recording with automatic pause on window unfocus
    pub async fn start_recording<F>(&self, mut event_callback: F) -> Result<Vec<MacroStep>, String>
    where
        F: FnMut(MacroStep) + Send + 'static,
    {
        println!("🎬 Starting macro recording with Tauri focus detection");

        // Set recording state
        self.state.set_recording_state(true);
        self.state.reset_macros();

        let state = self.state.clone();
        let device_state = DeviceState::new();
        let mut previous_keys: HashSet<Keycode> = HashSet::new();
        let mut last_action_time = Instant::now();
        let mut step_count = 0;
        const MAX_STEPS: usize = 10000;

        // Simple recording loop with focus detection
        loop {
            // Check if recording should stop
            if let Some(false) = state.get_recording_state() {
                break;
            }

            // Skip recording if paused (window unfocused)
            if state.is_paused() {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }

            // Prevent infinite recording
            if step_count >= MAX_STEPS {
                println!("⚠️ Maximum steps reached, stopping recording");
                break;
            }

            // Get current keyboard state
            let keys: HashSet<Keycode> = device_state.get_keys().into_iter().collect();

            // Calculate delay since last action
            let current_time = Instant::now();
            let delay = current_time.duration_since(last_action_time).as_millis() as u64;

            // Detect key presses (keys that are now pressed but weren't before)
            for key in keys.difference(&previous_keys) {
                let key_name = format!("{:?}", key);
                let step = MacroStep {
                    action: ActionExecutable::KeyDown(key_name),
                    delay: if step_count == 0 { 0 } else { delay },
                };

                state.add_macro_step(&step);
                event_callback(step);
                step_count += 1;
                last_action_time = current_time;
            }

            // Detect key releases (keys that were pressed but aren't now)
            for key in previous_keys.difference(&keys) {
                let key_name = format!("{:?}", key);
                let step = MacroStep {
                    action: ActionExecutable::KeyUp(key_name),
                    delay: if step_count == 0 { 0 } else { delay },
                };

                state.add_macro_step(&step);
                event_callback(step);
                step_count += 1;
                last_action_time = current_time;
            }

            previous_keys = keys;

            // Small delay to prevent excessive CPU usage
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        println!("✅ Macro recording stopped with {} steps", step_count);
        Ok(state.get_macros())
    }

    pub fn stop_recording(&self) {
        println!("🛑 Stopping macro recording");
        self.state.set_recording_state(false);
    }

    pub fn reset_macros(&self) {
        println!("🔄 Resetting recorded macros");
        self.state.reset_macros();
    }

    pub fn is_recording(&self) -> bool {
        self.state.get_recording_state().unwrap_or(false)
    }

    pub fn is_paused(&self) -> bool {
        self.state.is_paused()
    }

    pub fn get_recorded_steps(&self) -> Vec<MacroStep> {
        self.state.get_macros()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SerializedMacroStep {
    pub action: ActionExecutable,
    pub delay: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MacroCommand {
    pub name: String,
    pub macro_chain: Vec<SerializedMacroStep>,
    pub macro_type: String, // "RunOnce", "RepeatWhilePressing", "Switch", "Queue"
}

impl MacroCommand {
    pub fn to_universal_actions(&self) -> Vec<UniversalAction> {
        self.macro_chain
            .iter()
            .map(|step| step.action.to_universal_action())
            .collect()
    }
}
