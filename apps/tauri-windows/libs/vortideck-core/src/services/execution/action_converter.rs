/// Action Converter - Converts database Action enum to UniversalAction
/// This module handles loading action data from the database and converting it
/// to the platform-agnostic UniversalAction format for execution.
use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::UniversalAction;
use crate::db::models::Action;
use crate::db::Database;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionReferenceKind {
    SavedAction,
    Macro,
    Program,
    Flow,
    SystemControl,
    HttpRequest,
    FocusWindow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActionReference {
    pub kind: ActionReferenceKind,
    pub id: String,
}

impl ActionReference {
    pub fn storage_key(&self) -> Result<String, String> {
        let action = self.to_database_action()?;
        let canonical =
            Self::from_database_action(&action).ok_or("Unsupported saved action reference")?;
        serde_json::to_string(&canonical).map_err(|_| "Invalid action reference".into())
    }
    pub fn from_database_action(action: &Action) -> Option<Self> {
        let (kind, id) = match action {
            Action::SavedActionId(id) => (ActionReferenceKind::SavedAction, id.clone()),
            Action::MacroId(id) => (ActionReferenceKind::Macro, id.to_string()),
            Action::Program(id) => (ActionReferenceKind::Program, id.to_string()),
            Action::FlowId(id) => (ActionReferenceKind::Flow, id.clone()),
            Action::SystemControl(id) => (ActionReferenceKind::SystemControl, id.clone()),
            Action::HttpRequestId(id) => (ActionReferenceKind::HttpRequest, id.to_string()),
            Action::FocusWindowId(id) => (ActionReferenceKind::FocusWindow, id.to_string()),
            Action::App(_) => return None,
        };
        Some(Self { kind, id })
    }
    fn numeric_id(&self, label: &str) -> Result<i32, String> {
        self.id.parse::<i32>().map_err(|_| {
            format!(
                "{} reference '{}' is not a valid numeric ID",
                label, self.id
            )
        })
    }

    pub fn to_database_action(&self) -> Result<Action, String> {
        match self.kind {
            ActionReferenceKind::SavedAction => Ok(Action::SavedActionId(self.id.clone())),
            ActionReferenceKind::Macro => Ok(Action::MacroId(self.numeric_id("Macro")?)),
            ActionReferenceKind::Program => Ok(Action::Program(self.numeric_id("Program")?)),
            ActionReferenceKind::Flow => Ok(Action::FlowId(self.id.clone())),
            ActionReferenceKind::SystemControl => Ok(Action::SystemControl(self.id.clone())),
            ActionReferenceKind::HttpRequest => {
                Ok(Action::HttpRequestId(self.numeric_id("HTTP request")?))
            }
            ActionReferenceKind::FocusWindow => {
                Ok(Action::FocusWindowId(self.numeric_id("Focus window")?))
            }
        }
    }
}

pub async fn convert_action_reference_to_universal_action(
    reference: &ActionReference,
    value: Option<f64>,
    button_id: Option<String>,
    db: &Arc<Database>,
) -> Result<UniversalAction, String> {
    let action = reference.to_database_action()?;
    convert_action_to_universal_action(&action, value, button_id, db).await
}

/// Convert a database Action to a UniversalAction
///
/// # Arguments
/// * `action` - The Action enum from the database
/// * `value` - Optional value parameter (e.g., for volume control)
/// * `button_id` - Optional button ID for tracking purposes
/// * `db` - Database instance for loading action data
///
/// # Returns
/// * `Ok(UniversalAction)` - The converted action ready for execution
/// * `Err(String)` - Error message if conversion fails
pub async fn convert_action_to_universal_action(
    action: &Action,
    value: Option<f64>,
    button_id: Option<String>,
    db: &Arc<Database>,
) -> Result<UniversalAction, String> {
    match action {
        Action::SystemControl(command) => {
            let mut params = HashMap::new();

            // Parse embedded value (e.g., "system.volume.set:47.7")
            if command.contains(':') {
                let parts: Vec<&str> = command.split(':').collect();
                if parts.len() == 2 {
                    if let Ok(val) = parts[1].parse::<f64>() {
                        params.insert("volume".to_string(), serde_json::json!(val));
                        params.insert("button_id".to_string(), serde_json::json!(button_id));
                        return Ok(UniversalAction::SystemControl {
                            command: parts[0].to_string(),
                            parameters: Some(params),
                        });
                    }
                }
            }

            // Use value parameter if provided
            if let Some(val) = value {
                params.insert("volume".to_string(), serde_json::json!(val));
                params.insert("button_id".to_string(), serde_json::json!(button_id));
            }

            Ok(UniversalAction::SystemControl {
                command: command.clone(),
                parameters: Some(params),
            })
        }

        Action::SavedActionId(saved_action_id) => {
            let saved_action = db
                .get_saved_action_by_stable_id(saved_action_id)
                .await
                .map_err(|e| format!("Failed to resolve saved action: {}", e))?;

            let parameters: HashMap<String, serde_json::Value> =
                serde_json::from_str(&saved_action.params).map_err(|e| {
                    format!(
                        "Saved action '{}' has invalid parameters: {}",
                        saved_action_id, e
                    )
                })?;

            Ok(UniversalAction::Integration {
                integration_id: saved_action.integration_id,
                action_id: saved_action.action,
                parameters,
            })
        }

        Action::HttpRequestId(request_id) => {
            // Load HTTP request from database
            let http_request = db
                .get_http_request_by_id(*request_id)
                .await
                .map_err(|e| format!("Failed to load HTTP request: {}", e))?;

            saved_http_request_template(&http_request)
        }

        Action::FocusWindowId(window_id) => {
            // Load focus window from database
            let focus_window = db
                .get_focus_window_by_id(*window_id)
                .await
                .map_err(|e| format!("Failed to load focus window: {}", e))?;

            Ok(UniversalAction::FocusWindow {
                name: focus_window.name,
                window_title: focus_window.window_title,
                process_name: focus_window.process_name,
                match_mode: focus_window.match_mode,
            })
        }

        Action::Program(program_id) => {
            // Load program from database
            let program = db
                .get_program_by_id(*program_id)
                .await
                .map_err(|e| format!("Failed to load program: {}", e))?
                .ok_or_else(|| format!("Program not found with ID: {}", program_id))?;

            let args: Vec<String> = if let Some(args_str) = &program.args {
                if !args_str.is_empty() {
                    serde_json::from_str(args_str).unwrap_or_else(|_| vec![])
                } else {
                    vec![]
                }
            } else {
                vec![]
            };

            let env_vars: Option<HashMap<String, String>> =
                if let Some(env_vars_str) = &program.env_vars {
                    if !env_vars_str.is_empty() {
                        serde_json::from_str(env_vars_str).ok()
                    } else {
                        None
                    }
                } else {
                    None
                };

            let program_type = program
                .program_type
                .parse()
                .unwrap_or(super::ProgramType::Command);

            Ok(UniversalAction::Program {
                executable: program.executable,
                args,
                working_dir: program.working_dir.filter(|w| !w.is_empty()),
                env_vars,
                timeout_ms: program.timeout_ms.filter(|&t| t > 0).map(|t| t as u64),
                program_type,
                show_terminal: program.show_terminal,
            })
        }

        Action::MacroId(macro_id) => {
            // Convert macro ID to UniversalAction::Macro
            Ok(UniversalAction::Macro {
                macro_id: macro_id.to_string(),
                parameters: None,
            })
        }

        Action::FlowId(flow_id) => {
            // Convert flow ID to UniversalAction::Flow
            Ok(UniversalAction::Flow {
                flow_id: flow_id.clone(),
                parameters: None,
            })
        }

        Action::App(app_identifier) => {
            // Convert app to UniversalAction::AppControl
            // Default to Launch action
            Ok(UniversalAction::AppControl {
                app_identifier: app_identifier.clone(),
                action: super::AppAction::Launch,
                parameters: None,
            })
        }
    }
}

pub fn saved_http_request_template(
    request: &crate::db::models::HttpRequest,
) -> Result<UniversalAction, String> {
    let config = crate::services::api::saved_request::parse_saved_request(
        &request.method,
        &request.url,
        request.headers.as_deref(),
        request.body.as_deref(),
        request.timeout_ms,
    )?;
    let accepted_status_codes: Option<Vec<u16>> = request
        .accepted_status_codes
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|_| "Invalid saved HTTP status policy")?;
    crate::services::api::saved_request::validate_status_codes(accepted_status_codes.as_deref())?;
    Ok(UniversalAction::HttpRequest {
        name: request.name.clone(),
        method: request.method.clone(),
        url: request.url.clone(),
        headers: config.headers,
        body: config.body,
        timeout_ms: config.timeout_ms,
        accepted_status_codes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_action_reference_uses_stable_wire_shape() {
        let reference: ActionReference = serde_json::from_value(serde_json::json!({
            "kind": "saved_action",
            "id": "f625a62e-a141-4e1e-b01a-d43ec12bcc26"
        }))
        .expect("valid action reference");

        assert_eq!(reference.kind, ActionReferenceKind::SavedAction);
        assert!(matches!(
            reference.to_database_action().expect("resolved reference"),
            Action::SavedActionId(id) if id == "f625a62e-a141-4e1e-b01a-d43ec12bcc26"
        ));
    }

    #[test]
    fn numeric_action_reference_rejects_non_numeric_id() {
        let reference = ActionReference {
            kind: ActionReferenceKind::Macro,
            id: "not-an-id".to_string(),
        };

        assert!(reference.to_database_action().is_err());
    }

    #[test]
    fn numeric_action_reference_converts_transport_string_at_boundary() {
        let reference = ActionReference {
            kind: ActionReferenceKind::Macro,
            id: "10".to_string(),
        };

        assert!(matches!(
            reference
                .to_database_action()
                .expect("valid macro reference"),
            Action::MacroId(10)
        ));
    }

    #[test]
    fn test_system_control_with_embedded_value() {
        // Test that embedded values in system control commands are parsed correctly
        let action = Action::SystemControl("system.volume.set:75.5".to_string());
        // This would need a mock database to test properly
        // For now, this is a placeholder for future testing
    }
}
