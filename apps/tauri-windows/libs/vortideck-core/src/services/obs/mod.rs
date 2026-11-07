use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// OBS Action types that can be executed
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ObsAction {
    StreamStart,
    StreamStop,
    StreamToggle,
    RecordingStart,
    RecordingStop,
    RecordingToggle,
    BufferStart,
    BufferStop,
    BufferToggle,
    BufferSave,
    DesktopMute,
    DesktopUnmute,
    DesktopMuteToggle,
    MicMute,
    MicUnmute,
    MicMuteToggle,
    CollectionActivate,
    SourceActivate,
    SourceDeactivate,
    SourceToggle,
    MixerMute,
    MixerUnmute,
    MixerMuteToggle,
    SceneActivate,
}

impl ObsAction {
    pub fn to_action_id(&self) -> &'static str {
        match self {
            ObsAction::StreamStart => "obs_stream_start",
            ObsAction::StreamStop => "obs_stream_stop",
            ObsAction::StreamToggle => "obs_stream_toggle",
            ObsAction::RecordingStart => "obs_recording_start",
            ObsAction::RecordingStop => "obs_recording_stop",
            ObsAction::RecordingToggle => "obs_recording_toggle",
            ObsAction::BufferStart => "obs_buffer_start",
            ObsAction::BufferStop => "obs_buffer_stop",
            ObsAction::BufferToggle => "obs_buffer_toggle",
            ObsAction::BufferSave => "obs_buffer_save",
            ObsAction::DesktopMute => "obs_desktop_mute",
            ObsAction::DesktopUnmute => "obs_desktop_unmute",
            ObsAction::DesktopMuteToggle => "obs_desktop_mute_toggle",
            ObsAction::MicMute => "obs_mic_mute",
            ObsAction::MicUnmute => "obs_mic_unmute",
            ObsAction::MicMuteToggle => "obs_mic_mute_toggle",
            ObsAction::CollectionActivate => "obs_collection_activate",
            ObsAction::SceneActivate => "obs_scene_activate",
            ObsAction::SourceActivate => "obs_source_activate",
            ObsAction::SourceDeactivate => "obs_source_deactivate",
            ObsAction::SourceToggle => "obs_source_toggle",
            ObsAction::MixerMute => "obs_mixer_mute",
            ObsAction::MixerUnmute => "obs_mixer_unmute",
            ObsAction::MixerMuteToggle => "obs_mixer_mute_toggle",
        }
    }
}

/// Parameters for OBS actions
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ObsParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] 
    pub source_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection_name: Option<String>,
}

impl ObsParams {
    pub fn new() -> Self {
        Self {
            scene_name: None,
            source_name: None,
            collection_name: None,
        }
    }

    pub fn with_scene(scene_name: String) -> Self {
        Self {
            scene_name: Some(scene_name),
            source_name: None,
            collection_name: None,
        }
    }

    pub fn with_source(source_name: String) -> Self {
        Self {
            scene_name: None,
            source_name: Some(source_name),
            collection_name: None,
        }
    }

    pub fn with_collection(collection_name: String) -> Self {
        Self {
            scene_name: None,
            source_name: None,
            collection_name: Some(collection_name),
        }
    }
}

/// OBS plugin action with parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsPluginAction {
    #[serde(rename = "actionId")]
    pub action_id: String,
    pub name: String,
    pub parameters: Vec<ObsParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsParameter {
    #[serde(rename = "assignmentMessage")]
    pub assignment_message: String,
    #[serde(rename = "errorMessage")]
    pub error_message: String,
    #[serde(rename = "listSelection")]
    pub list_selection: Vec<ObsListItem>,
    #[serde(rename = "parameterId")]
    pub parameter_id: String,
    #[serde(rename = "parameterType")]
    pub parameter_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsListItem {
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<ObsParameter>>,
}

// OBS integration is now handled via the core integration framework, not as a service.