//! Owner-authored invocation inputs. Bind once before capability/approval/admission.
//! No interpolation, coercion, evaluator, or caller-controlled execution target.

use super::UniversalAction;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputContractSnapshot {
    pub revision: i64,
    pub contract: Option<ActionInputContract>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActionInputContract {
    pub version: u8,
    pub input_schema: Value,
    #[serde(default)]
    pub bindings: Vec<InputBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputBinding {
    pub input: String,
    pub target: InputTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum InputTarget {
    HttpBody { pointer: String },
    HttpQuery { name: String },
    HttpPath { segment: usize },
    IntegrationParameter { pointer: String },
    SystemParameter { name: String },
}

impl ActionInputContract {
    pub fn validate_for_template(&self, template: &UniversalAction) -> Result<(), String> {
        self.validate_definition()?;
        if matches!(template, UniversalAction::Flow { .. }) {
            return if self.bindings.is_empty() {
                Ok(())
            } else {
                Err("Flow inputs use their declared names directly".into())
            };
        }
        if !matches!(
            template,
            UniversalAction::HttpRequest { .. }
                | UniversalAction::Integration { .. }
                | UniversalAction::SystemControl { .. }
        ) {
            return Err("This saved action type does not yet support input bindings".into());
        }
        for binding in &self.bindings {
            let scalar = |value: Option<&Value>| {
                value.is_some_and(|value| !value.is_object() && !value.is_array())
            };
            let valid = match (template, &binding.target) {
                (UniversalAction::HttpRequest { body, .. }, InputTarget::HttpBody { pointer }) => {
                    scalar(body.as_ref().and_then(|body| body.pointer(pointer)))
                }
                (UniversalAction::HttpRequest { .. }, InputTarget::HttpQuery { .. }) => true,
                (UniversalAction::HttpRequest { url, .. }, InputTarget::HttpPath { segment }) => {
                    reqwest::Url::parse(url).ok().is_some_and(|url| {
                        url.path_segments()
                            .is_some_and(|segments| segments.count() > *segment)
                    })
                }
                (
                    UniversalAction::Integration { parameters, .. },
                    InputTarget::IntegrationParameter { pointer },
                ) => scalar(
                    serde_json::to_value(parameters)
                        .ok()
                        .as_ref()
                        .and_then(|parameters| parameters.pointer(pointer)),
                ),
                (UniversalAction::SystemControl { .. }, InputTarget::SystemParameter { .. }) => {
                    true
                }
                _ => false,
            };
            if !valid {
                return Err(
                    "Input binding does not select a supported scalar field in the saved action"
                        .into(),
                );
            }
        }
        Ok(())
    }

    pub fn validate_definition(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("Unsupported action input contract version".into());
        }
        if self.input_schema.to_string().len() > 65_536 || self.bindings.len() > 128 {
            return Err("Action input contract is too large".into());
        }
        if self.input_schema.get("type").and_then(Value::as_str) != Some("object") {
            return Err("Action inputs must declare an object schema".into());
        }
        let properties = self
            .input_schema
            .get("properties")
            .and_then(Value::as_object)
            .ok_or("Action input schema must declare properties")?;
        if properties.len() > 128 {
            return Err("Action inputs exceed 128 declared fields".into());
        }
        if let Some(required) = self.input_schema.get("required").and_then(Value::as_array)
            && required.iter().any(|name| {
                name.as_str()
                    .is_none_or(|name| !properties.contains_key(name))
            })
        {
            return Err("Required action inputs must be declared properties".into());
        }
        for (name, schema) in properties {
            if name.is_empty() || name.len() > 128 {
                return Err("Invalid action input name".into());
            }
            let scalar = |value: &Value| {
                value.as_str().is_some_and(|kind| {
                    matches!(kind, "string" | "number" | "integer" | "boolean" | "null")
                })
            };
            let supported = schema.get("type").is_some_and(|kind| {
                scalar(kind)
                    || kind
                        .as_array()
                        .is_some_and(|types| !types.is_empty() && types.iter().all(scalar))
            });
            if !supported {
                return Err(format!("Input '{name}' must declare scalar types"));
            }
        }
        self.validator()?;
        let mut targets = HashSet::new();
        for binding in &self.bindings {
            if !properties.contains_key(&binding.input) {
                return Err(format!(
                    "Binding refers to undeclared input '{}'",
                    binding.input
                ));
            }
            if !targets
                .insert(serde_json::to_string(&binding.target).map_err(|_| "Invalid input target")?)
            {
                return Err("Two bindings cannot write the same target".into());
            }
            match &binding.target {
                InputTarget::HttpBody { pointer }
                | InputTarget::IntegrationParameter { pointer } => {
                    if !pointer.starts_with('/') {
                        return Err("Bindings require a non-root JSON pointer".into());
                    }
                }
                InputTarget::HttpQuery { name } => {
                    if name.is_empty()
                        || matches!(
                            name.to_ascii_lowercase().as_str(),
                            "token"
                                | "access_token"
                                | "api_key"
                                | "apikey"
                                | "authorization"
                                | "password"
                                | "secret"
                        )
                    {
                        return Err("Query bindings cannot select credential fields".into());
                    }
                }
                InputTarget::SystemParameter { name } if name.is_empty() => {
                    return Err("Parameter bindings require a name".into());
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn validator(&self) -> Result<jsonschema::Validator, String> {
        jsonschema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .should_validate_formats(false)
            .build(&self.input_schema)
            .map_err(|_| "Invalid action input JSON Schema".into())
    }

    pub fn validate_inputs(&self, inputs: &Value) -> Result<Map<String, Value>, String> {
        self.validate_definition()?;
        if inputs.to_string().len() > 65_536 {
            return Err("Action inputs exceed 64 KiB".into());
        }
        let mut values = inputs
            .as_object()
            .cloned()
            .ok_or("Action inputs must be an object")?;
        let properties = self.input_schema["properties"]
            .as_object()
            .ok_or("Missing input properties")?;
        if let Some(unknown) = values.keys().find(|name| !properties.contains_key(*name)) {
            return Err(format!("Undeclared action input '{unknown}'"));
        }
        for (name, schema) in properties {
            if !values.contains_key(name)
                && let Some(default) = schema.get("default")
            {
                values.insert(name.clone(), default.clone());
            }
        }
        for (name, value) in &values {
            if value.is_array() || value.is_object() {
                return Err(format!("Input '{name}' must be scalar"));
            }
            if value.as_str().is_some_and(|text| {
                text.contains(crate::services::api::credentials::CREDENTIAL_PREFIX)
            }) {
                return Err("Invocation inputs cannot supply credential references".into());
            }
        }
        if !self.validator()?.is_valid(&Value::Object(values.clone())) {
            // Do not echo private caller data through schema validation diagnostics.
            return Err("Action inputs do not satisfy the declared schema".into());
        }
        Ok(values)
    }

    pub fn prepare(
        &self,
        template: &UniversalAction,
        inputs: &Value,
    ) -> Result<UniversalAction, String> {
        self.validate_for_template(template)?;
        let values = self.validate_inputs(inputs)?;
        let mut action = template.clone();
        if let UniversalAction::Flow { parameters, .. } = &mut action {
            if !self.bindings.is_empty() {
                return Err("Flow inputs use their declared names directly".into());
            }
            *parameters = Some(values.into_iter().collect());
            return Ok(action);
        }
        for binding in &self.bindings {
            let Some(value) = values.get(&binding.input) else {
                continue;
            }; // absent optional: retain literal
            match (&mut action, &binding.target) {
                (UniversalAction::HttpRequest { body, .. }, InputTarget::HttpBody { pointer }) => {
                    write_pointer(
                        body.as_mut()
                            .ok_or("HTTP body binding requires a configured body")?,
                        pointer,
                        value,
                    )?;
                }
                (UniversalAction::HttpRequest { url, .. }, InputTarget::HttpQuery { name }) => {
                    let mut target =
                        reqwest::Url::parse(url).map_err(|_| "Invalid saved HTTP URL")?;
                    let mut pairs: Vec<(String, String)> = target
                        .query_pairs()
                        .filter(|(key, _)| key != name)
                        .map(|(key, value)| (key.into_owned(), value.into_owned()))
                        .collect();
                    pairs.push((name.clone(), scalar_text(value)?));
                    target.query_pairs_mut().clear().extend_pairs(pairs);
                    *url = target.to_string();
                }
                (UniversalAction::HttpRequest { url, .. }, InputTarget::HttpPath { segment }) => {
                    let mut target =
                        reqwest::Url::parse(url).map_err(|_| "Invalid saved HTTP URL")?;
                    let mut segments: Vec<String> = target
                        .path_segments()
                        .ok_or("HTTP URL has no path")?
                        .map(str::to_owned)
                        .collect();
                    if *segment >= segments.len() {
                        return Err("HTTP path binding is outside the saved path".into());
                    }
                    let text = scalar_text(value)?;
                    if matches!(text.as_str(), "." | "..") {
                        return Err("HTTP path inputs cannot traverse directories".into());
                    }
                    // Existing segments are already encoded; decode before pushing exactly once.
                    for existing in &mut segments {
                        *existing = urlencoding::decode(existing)
                            .map_err(|_| "HTTP path is not UTF-8")?
                            .into_owned();
                    }
                    *segments
                        .get_mut(*segment)
                        .ok_or("Invalid HTTP path segment")? = text;
                    target
                        .path_segments_mut()
                        .map_err(|_| "HTTP URL cannot accept path segments")?
                        .clear()
                        .extend(segments);
                    *url = target.to_string();
                }
                (
                    UniversalAction::Integration { parameters, .. },
                    InputTarget::IntegrationParameter { pointer },
                ) => {
                    let mut data = serde_json::to_value(&*parameters)
                        .map_err(|_| "Invalid saved parameters")?;
                    write_pointer(&mut data, pointer, value)?;
                    *parameters =
                        serde_json::from_value(data).map_err(|_| "Invalid prepared parameters")?;
                }
                (
                    UniversalAction::SystemControl { parameters, .. },
                    InputTarget::SystemParameter { name },
                ) => {
                    parameters
                        .get_or_insert_with(HashMap::new)
                        .insert(name.clone(), value.clone());
                }
                _ => return Err("Input binding target does not match the saved action".into()),
            }
        }
        Ok(action)
    }
}

fn write_pointer(target: &mut Value, pointer: &str, value: &Value) -> Result<(), String> {
    let slot = target
        .pointer_mut(pointer)
        .ok_or("Input binding points to a missing saved field")?;
    if slot.is_array() || slot.is_object() {
        return Err("Input bindings can replace scalar fields only".into());
    }
    *slot = value.clone();
    Ok(())
}

fn scalar_text(value: &Value) -> Result<String, String> {
    match value {
        Value::String(text) => Ok(text.clone()),
        Value::Number(_) | Value::Bool(_) => Ok(value.to_string()),
        _ => Err("URL/program argument bindings require a non-null scalar".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn template() -> UniversalAction {
        UniversalAction::HttpRequest {
            name: "Fixture".into(),
            method: "POST".into(),
            url: "http://localhost:1234/v1/a%20b?fixed=one".into(),
            headers: HashMap::from([("Content-Type".into(), "application/json".into())]),
            body: Some(json!({"text":"literal", "duration": 0, "enabled": true})),
            timeout_ms: 1000,
            accepted_status_codes: None,
        }
    }
    fn contract() -> ActionInputContract {
        serde_json::from_value(json!({"version":1, "inputSchema":{"type":"object", "properties":{
            "text":{"type":"string","maxLength":100},"duration":{"type":"integer","minimum":1,"maximum":30,"default":10},
            "enabled":{"type":["boolean","null"]}},"required":["text"]},"bindings":[
            {"input":"text","target":{"type":"httpBody","pointer":"/text"}},
            {"input":"duration","target":{"type":"httpBody","pointer":"/duration"}},
            {"input":"enabled","target":{"type":"httpBody","pointer":"/enabled"}}]})).unwrap()
    }
    #[test]
    fn typed_values_defaults_and_literal_compatibility_survive_preparation() {
        let original = template();
        let UniversalAction::HttpRequest { body, .. } = contract()
            .prepare(
                &original,
                &json!({"text":"quotes \" and 雪\n", "enabled": false}),
            )
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(
            body.unwrap(),
            json!({"text":"quotes \" and 雪\n", "duration":10,"enabled":false})
        );
        let UniversalAction::HttpRequest { body, .. } = original else {
            panic!()
        };
        assert_eq!(body.unwrap()["text"], "literal");
        let UniversalAction::HttpRequest { body, .. } = contract()
            .prepare(&template(), &json!({"text":"", "enabled":null}))
            .unwrap()
        else {
            panic!()
        };
        assert!(body.unwrap()["enabled"].is_null());
    }
    #[test]
    fn invalid_inputs_fail_without_coercion_or_authority_changes() {
        for inputs in [
            json!({}),
            json!({"text":"ok","duration":"10"}),
            json!({"text":"ok","duration":31}),
            json!({"text":"ok","host":"https://other"}),
            json!({"text":"vortideck-credential://stolen"}),
            json!({"text":null}),
        ] {
            assert!(contract().prepare(&template(), &inputs).is_err());
        }
        let mut contract = contract();
        contract.bindings[0].target = InputTarget::HttpQuery {
            name: "access_token".into(),
        };
        assert!(contract.validate_definition().is_err());
        contract.bindings[0].target = InputTarget::HttpBody {
            pointer: "/missing".into(),
        };
        assert!(contract.validate_for_template(&template()).is_err());
    }
    #[test]
    fn url_bindings_encode_data_once_and_keep_the_owned_origin() {
        let mut contract = contract();
        contract.bindings = vec![
            InputBinding {
                input: "text".into(),
                target: InputTarget::HttpQuery { name: "q".into() },
            },
            InputBinding {
                input: "text".into(),
                target: InputTarget::HttpPath { segment: 1 },
            },
        ];
        let UniversalAction::HttpRequest { url, .. } = contract
            .prepare(
                &template(),
                &json!({"text":"https://evil.example/a?b=1 &雪"}),
            )
            .unwrap()
        else {
            panic!()
        };
        let url = reqwest::Url::parse(&url).unwrap();
        assert_eq!(url.origin().ascii_serialization(), "http://localhost:1234");
        assert_eq!(url.path_segments().unwrap().count(), 2);
        assert!(url
            .query_pairs()
            .any(|(key, value)| key == "q" && value == "https://evil.example/a?b=1 &雪"));
        assert!(url
            .query_pairs()
            .any(|(key, value)| key == "fixed" && value == "one"));
        assert!(contract
            .prepare(&template(), &json!({"text":".."}))
            .is_err());
    }
    #[test]
    fn integration_and_flow_inputs_share_validation_and_fixed_targets() {
        let mut contract = contract();
        contract.bindings = vec![InputBinding {
            input: "text".into(),
            target: InputTarget::IntegrationParameter {
                pointer: "/message".into(),
            },
        }];
        let action = UniversalAction::Integration {
            integration_id: "app".into(),
            action_id: "send".into(),
            parameters: HashMap::from([("message".into(), json!("literal"))]),
        };
        let UniversalAction::Integration {
            integration_id,
            action_id,
            parameters,
        } = contract.prepare(&action, &json!({"text":"hello"})).unwrap()
        else {
            panic!()
        };
        assert_eq!((integration_id, action_id), ("app".into(), "send".into()));
        assert_eq!(parameters["message"], "hello");
        contract.bindings.clear();
        let UniversalAction::Flow {
            flow_id,
            parameters,
        } = contract
            .prepare(
                &UniversalAction::Flow {
                    flow_id: "fixed-flow".into(),
                    parameters: None,
                },
                &json!({"text":"hello"}),
            )
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(flow_id, "fixed-flow");
        assert_eq!(parameters.unwrap()["duration"], 10);
    }
}
