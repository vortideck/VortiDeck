//! Installed widget command declarations. No executor or delivery queue lives here.
use serde_json::Value;
use std::collections::BTreeMap;

pub fn validate_command_catalog(commands: &BTreeMap<String, Value>) -> Result<(), String> {
    if commands.len() > 64
        || serde_json::to_vec(commands)
            .map_err(|_| "Invalid widget commands")?
            .len()
            > 65_536
    {
        return Err("Widget command catalog exceeds its limit".into());
    }
    for (name, schema) in commands {
        validate_name(name)?;
        super::widget_settings::compile_config_schema(schema)?;
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.len() > 128
        || !name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
    {
        return Err(
            "Widget command names must be stable ASCII identifiers (1–128 characters)".into(),
        );
    }
    Ok(())
}

/// False means unsupported, rather than an invalid payload. Broadcast adapters
/// can skip unsupported widgets while rejecting malformed input before delivery.
/// Missing declarations preserve existing packages; new packages should declare
/// an explicit map. Defaults never rewrite a delivery's durable payload.
pub fn accepts_command(
    commands: Option<&BTreeMap<String, Value>>,
    name: &str,
    payload: &Value,
) -> Result<bool, String> {
    validate_name(name)?;
    if payload.to_string().len() > 65_536 {
        return Err("Widget command payload exceeds 64 KiB".into());
    }
    let Some(commands) = commands else {
        return Ok(true);
    };
    let Some(schema) = commands.get(name) else {
        return Ok(false);
    };
    let validator = super::widget_settings::compile_config_schema(schema)?;
    if !validator.is_valid(payload) {
        // Do not echo user payloads (which can contain private application data).
        return Err(format!(
            "Payload does not match widget command '{name}' schema"
        ));
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn declarations_validate_without_coercing_or_rewriting_delivery() {
        let catalog = BTreeMap::from([(
            "timer.start".into(),
            json!({"type":"object","additionalProperties":false,"required":["seconds"],"properties":{"seconds":{"type":"integer","minimum":1,"maximum":3600,"default":60}}}),
        )]);
        validate_command_catalog(&catalog).unwrap();
        assert!(accepts_command(Some(&catalog), "timer.start", &json!({"seconds":30})).unwrap());
        assert!(!accepts_command(Some(&catalog), "timer.skip", &json!({})).unwrap());
        for invalid in [
            json!({}),
            json!({"seconds":"30"}),
            json!({"seconds":0}),
            json!({"seconds":30,"secret":"do-not-echo"}),
        ] {
            let error = accepts_command(Some(&catalog), "timer.start", &invalid).unwrap_err();
            assert!(!error.contains("do-not-echo"));
        }
        assert!(accepts_command(None, "legacy.show", &json!(null)).unwrap());
        assert!(!accepts_command(Some(&BTreeMap::new()), "legacy.show", &json!(null)).unwrap());
        assert!(validate_command_catalog(&BTreeMap::from([(
            "bad name".into(),
            json!({"type":"object"})
        )]))
        .is_err());
        assert!(validate_command_catalog(&BTreeMap::from([(
            "x".into(),
            json!({"type":"object","$ref":"https://example.com/commands"})
        )]))
        .is_err());
    }
}
