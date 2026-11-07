pub mod engine;
pub mod recording;

pub use engine::MacroEngine;
pub use recording::{ActionExecutable, MacroRecorder, MacroStep, RecordingState};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a recorded macro with steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedMacro {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<MacroStep>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub user_id: String,
}
