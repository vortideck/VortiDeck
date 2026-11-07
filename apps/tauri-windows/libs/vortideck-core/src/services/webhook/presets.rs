use once_cell::sync::Lazy;
///! Webhook service presets - Load JSON preset files for different webhook providers
///! Presets define how to register, verify, and handle webhooks from various services
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Preset configuration for a webhook service provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPreset {
    pub name: String,
    pub description: String,
    pub events: Vec<EventType>,
    pub registration: Option<RegistrationConfig>,
    pub signature_verification: Option<SignatureConfig>,
    pub documentation: String,
}

/// Event type that a service can emit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventType {
    pub id: String,
    pub name: String,
    pub description: String,
    pub example_payload: Value,
}

/// Configuration for automatic webhook registration with service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationConfig {
    pub api_url: String,
    pub method: String,
    pub auth_type: String,
    pub auth_header: String,
    pub body_template: Value,
    pub unregister_url: String,
}

/// Configuration for signature verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureConfig {
    pub header_name: String,
    pub algorithm: String,
    pub secret_location: String,
    pub message_components: Vec<String>,
}

/// Global lazy-loaded presets
static WEBHOOK_PRESETS: Lazy<HashMap<String, WebhookPreset>> = Lazy::new(|| load_webhook_presets());

/// Load all webhook presets from JSON files
fn load_webhook_presets() -> HashMap<String, WebhookPreset> {
    let mut presets = HashMap::new();

    // Load each preset from embedded JSON
    load_preset(&mut presets, "twitch", include_str!("presets/twitch.json"));
    load_preset(
        &mut presets,
        "discord",
        include_str!("presets/discord.json"),
    );
    load_preset(&mut presets, "github", include_str!("presets/github.json"));
    load_preset(&mut presets, "kick", include_str!("presets/kick.json"));

    // Add custom preset (no JSON file needed)
    presets.insert(
        "custom".to_string(),
        WebhookPreset {
            name: "Custom Webhook".to_string(),
            description: "Generic webhook for any service".to_string(),
            events: vec![EventType {
                id: "custom".to_string(),
                name: "Custom Event".to_string(),
                description: "Generic webhook event".to_string(),
                example_payload: serde_json::json!({"data": "Any JSON payload"}),
            }],
            registration: None,
            signature_verification: None,
            documentation: "".to_string(),
        },
    );

    presets
}

/// Load a single preset from JSON string
fn load_preset(presets: &mut HashMap<String, WebhookPreset>, key: &str, json: &str) {
    match serde_json::from_str::<WebhookPreset>(json) {
        Ok(preset) => {
            presets.insert(key.to_string(), preset);
        }
        Err(e) => {
            log::error!("Failed to load webhook preset '{}': {}", key, e);
        }
    }
}

/// Get all webhook presets
pub fn get_webhook_presets() -> &'static HashMap<String, WebhookPreset> {
    &WEBHOOK_PRESETS
}

/// Get a specific preset by service name
pub fn get_preset(service: &str) -> Option<&'static WebhookPreset> {
    WEBHOOK_PRESETS.get(service)
}

/// Get list of available services
pub fn get_available_services() -> Vec<String> {
    WEBHOOK_PRESETS.keys().cloned().collect()
}

/// Simple template variable replacement
/// Replaces {{variable}} with values from context
pub fn render_template(template: &Value, context: &Value) -> Result<Value, String> {
    let template_str = serde_json::to_string(template)
        .map_err(|e| format!("Template serialization error: {}", e))?;

    let rendered = render_template_string(&template_str, context)?;

    serde_json::from_str(&rendered).map_err(|e| format!("Template parsing error: {}", e))
}

/// Render template string with variable replacement
pub fn render_template_string(template: &str, context: &Value) -> Result<String, String> {
    let mut result = template.to_string();

    // Simple variable replacement: {{variable}} or {{object.property}}
    let re = regex::Regex::new(r"\{\{([a-zA-Z0-9_.]+)\}\}")
        .map_err(|e| format!("Regex error: {}", e))?;

    for cap in re.captures_iter(template) {
        let full_match = &cap[0];
        let var_path = &cap[1];

        // Navigate nested path (e.g., "config.broadcaster_id")
        let value = get_nested_value(context, var_path);

        if let Some(val) = value {
            let replacement = match val {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => val.to_string(),
            };

            result = result.replace(full_match, &replacement);
        }
    }

    Ok(result)
}

/// Get nested value from JSON object using dot notation
fn get_nested_value<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value;

    for part in parts {
        current = current.get(part)?;
    }

    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_loading() {
        let presets = get_webhook_presets();

        assert!(presets.contains_key("twitch"));
        assert!(presets.contains_key("discord"));
        assert!(presets.contains_key("github"));
        assert!(presets.contains_key("kick"));
        assert!(presets.contains_key("custom"));
    }

    #[test]
    fn test_get_preset() {
        let twitch = get_preset("twitch");
        assert!(twitch.is_some());
        assert_eq!(twitch.unwrap().name, "Twitch EventSub");
    }

    #[test]
    fn test_template_rendering() {
        let template = serde_json::json!({
            "type": "{{event_type}}",
            "condition": {
                "broadcaster_user_id": "{{broadcaster_id}}"
            }
        });

        let context = serde_json::json!({
            "event_type": "stream.online",
            "broadcaster_id": "12345"
        });

        let result = render_template(&template, &context).unwrap();

        assert_eq!(result["type"], "stream.online");
        assert_eq!(result["condition"]["broadcaster_user_id"], "12345");
    }

    #[test]
    fn test_nested_path() {
        let context = serde_json::json!({
            "config": {
                "broadcaster_id": "67890"
            }
        });

        let value = get_nested_value(&context, "config.broadcaster_id");
        assert_eq!(value, Some(&serde_json::json!("67890")));
    }
}
