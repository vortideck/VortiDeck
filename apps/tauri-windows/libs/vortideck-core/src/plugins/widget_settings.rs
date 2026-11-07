//! Widget authoring settings. Schemas come from installed manifests, never callers.
use serde_json::Value;

const MAX_SCHEMA_BYTES: usize = 65_536;
const MAX_CONFIG_BYTES: usize = 65_536;

/// Validate locally without resolving files or making network requests.
/// Use draft 7 consistently; formats are annotations, not hidden host-dependent checks.
pub fn compile_config_schema(schema: &Value) -> Result<jsonschema::Validator, String> {
    if !schema.is_object() || schema.get("type").and_then(Value::as_str) != Some("object") {
        return Err("Widget configSchema must declare type object".into());
    }
    if schema.to_string().len() > MAX_SCHEMA_BYTES {
        return Err("Widget configSchema exceeds 64 KiB".into());
    }
    if let Some(draft) = schema.get("$schema").and_then(Value::as_str) {
        if draft != "http://json-schema.org/draft-07/schema#"
            && draft != "https://json-schema.org/draft-07/schema#"
        {
            return Err("Widget configSchema must use JSON Schema draft 7".into());
        }
    }
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .should_validate_formats(false)
        .build(schema)
        .map_err(|error| format!("Invalid widget configSchema: {error}"))
}

/// Apply unambiguous defaults from object properties only. Never choose a branch
/// in oneOf/anyOf, coerce types, delete unknown keys, or overwrite explicit nulls.
fn apply_defaults(schema: &Value, config: &mut Value, depth: usize) {
    if depth > 32 {
        return;
    }
    if let (Some(properties), Some(object)) = (
        schema.get("properties").and_then(Value::as_object),
        config.as_object_mut(),
    ) {
        for (key, property) in properties {
            if !object.contains_key(key) {
                if let Some(default) = property.get("default") {
                    object.insert(key.clone(), default.clone());
                }
            }
            if let Some(value) = object.get_mut(key) {
                apply_defaults(property, value, depth + 1);
            }
        }
    } else if let (Some(items), Some(array)) = (schema.get("items"), config.as_array_mut()) {
        if items.is_object() {
            for value in array {
                apply_defaults(items, value, depth + 1);
            }
        }
    }
}

pub fn validate_widget_config(schema: Option<&Value>, config: &Value) -> Result<Value, String> {
    if !config.is_object() {
        return Err("Widget config must be an object".into());
    }
    if config.to_string().len() > MAX_CONFIG_BYTES {
        return Err("Widget config exceeds 64 KiB".into());
    }
    let Some(schema) = schema else {
        return Ok(config.clone());
    };
    let validator = compile_config_schema(schema)?;
    let mut value = config.clone();
    apply_defaults(schema, &mut value, 0);
    if value.to_string().len() > MAX_CONFIG_BYTES {
        return Err("Widget config with defaults exceeds 64 KiB".into());
    }
    let errors: Vec<String> = validator
        .iter_errors(&value)
        .take(8)
        .map(|error| error.to_string())
        .collect();
    if !errors.is_empty() {
        return Err(format!("Invalid widget settings: {}", errors.join("; ")));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_preserve_explicit_values_and_validate_nested_constraints() {
        let schema = json!({"type":"object","additionalProperties":false,"required":["goal"],"properties":{
            "goal":{"type":"integer","minimum":1,"default":1000},
            "style":{"type":"object","default":{},"properties":{"color":{"enum":["red","blue"],"default":"blue"}}},
            "label":{"type":["string","null"],"default":"Followers"}
        }});
        let config = json!({"goal":50,"label":null});
        assert_eq!(
            validate_widget_config(Some(&schema), &config).unwrap(),
            json!({"goal":50,"label":null,"style":{"color":"blue"}})
        );
        for invalid in [
            json!({"goal":0}),
            json!({"goal":"50"}),
            json!({"style":{"color":"yellow"}}),
            json!({"unknown":true}),
        ] {
            assert!(validate_widget_config(Some(&schema), &invalid).is_err());
        }
        assert_eq!(
            config,
            json!({"goal":50,"label":null}),
            "never mutate the caller's input"
        );
    }

    #[test]
    fn schemas_do_not_resolve_external_resources() {
        for reference in [
            "https://example.com/schema.json",
            "file:///C:/Users/private.json",
        ] {
            assert!(compile_config_schema(
                &json!({"type":"object","properties":{"value":{"$ref":reference}}})
            )
            .is_err());
        }
        let local = json!({"type":"object","definitions":{"label":{"type":"string","minLength":2}},"properties":{"title":{"$ref":"#/definitions/label"}}});
        assert!(validate_widget_config(Some(&local), &json!({"title":"ok"})).is_ok());
        assert!(validate_widget_config(Some(&local), &json!({"title":"x"})).is_err());
    }

    #[test]
    fn structured_values_use_real_schema_validation() {
        let schema = json!({"type":"object","properties":{"items":{"type":"array","minItems":1,"items":{"type":"object","required":["url"],"properties":{"url":{"type":"string","pattern":"^https://"}}}}}});
        assert!(validate_widget_config(
            Some(&schema),
            &json!({"items":[{"url":"https://example.com"}]})
        )
        .is_ok());
        assert!(validate_widget_config(
            Some(&schema),
            &json!({"items":[{"url":"javascript:bad"}]})
        )
        .is_err());
    }

    #[test]
    fn legacy_and_invalid_schema_behavior_is_explicit() {
        assert_eq!(
            validate_widget_config(None, &json!({"anything":[1,2]})).unwrap(),
            json!({"anything":[1,2]})
        );
        assert!(validate_widget_config(None, &json!([])).is_err());
        assert!(compile_config_schema(
            &json!({"type":"object","properties":{"x":{"type":"made-up"}}})
        )
        .is_err());
        assert!(compile_config_schema(&json!({"type":"array"})).is_err());
        assert!(
            validate_widget_config(None, &json!({"large":"x".repeat(MAX_CONFIG_BYTES)})).is_err()
        );
    }
}
