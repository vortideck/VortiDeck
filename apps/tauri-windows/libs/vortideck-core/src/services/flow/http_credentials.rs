//! Protect literal HTTP-node authentication before graph/checkpoint persistence.
//! Recovery retains immutable references, not redacted values that cannot execute.
use super::executor::FlowDefinition;
use crate::services::api::credentials::{
    is_credential_reference, protect_headers, requires_protection, ProtectedHeaders,
};
use serde_json::Value;
use std::collections::HashMap;

pub struct ProtectedFlow {
    pub flow: FlowDefinition,
    writes: Vec<ProtectedHeaders>,
    committed: bool,
}

pub struct ProtectedHttpInputs {
    pub inputs: HashMap<String, Value>,
    writes: Vec<ProtectedHeaders>,
    committed: bool,
}
impl ProtectedHttpInputs {
    pub fn commit(mut self) -> HashMap<String, Value> {
        self.committed = true;
        self.inputs.clone()
    }
}
impl Drop for ProtectedHttpInputs {
    fn drop(&mut self) {
        if !self.committed {
            for write in self.writes.drain(..) {
                write.rollback();
            }
        }
    }
}

pub fn protect_resolved_inputs(
    inputs: HashMap<String, Value>,
) -> Result<ProtectedHttpInputs, String> {
    let mut protected = ProtectedHttpInputs {
        inputs,
        writes: Vec::new(),
        committed: false,
    };
    rewrite_inputs(&mut protected.inputs, |value| {
        let write = protect_headers(
            HashMap::from([("Authorization".into(), value.to_string())]),
            &[],
        )?;
        let reference = write.headers["Authorization"].clone();
        protected.writes.push(write);
        Ok(reference)
    })?;
    Ok(protected)
}

impl ProtectedFlow {
    pub fn commit(mut self) -> FlowDefinition {
        self.committed = true;
        self.flow.clone()
    }
}

impl Drop for ProtectedFlow {
    fn drop(&mut self) {
        if !self.committed {
            for write in self.writes.drain(..) {
                write.rollback();
            }
        }
    }
}

/// The database adapter calls this inside its bounded blocking worker. A failed
/// persistence operation drops the guard and compensates only newly created keys.
pub fn protect_flow(flow: &FlowDefinition) -> Result<ProtectedFlow, String> {
    let mut protected = ProtectedFlow {
        flow: flow.clone(),
        writes: Vec::new(),
        committed: false,
    };
    for node in &mut protected.flow.nodes {
        if node.node_type != "http_request" {
            continue;
        }
        let mut inputs = protect_resolved_inputs(node.inputs.clone())?;
        node.inputs = inputs.inputs.clone();
        protected.writes.append(&mut inputs.writes);
        inputs.committed = true;
    }
    Ok(protected)
}

fn rewrite_inputs(
    inputs: &mut HashMap<String, Value>,
    mut protect: impl FnMut(&str) -> Result<String, String>,
) -> Result<(), String> {
    fn field(
        value: &mut Value,
        protect: &mut impl FnMut(&str) -> Result<String, String>,
    ) -> Result<(), String> {
        let Some(text) = value.as_str() else {
            return Err("HTTP authentication values must be strings".into());
        };
        if text.starts_with(crate::services::api::credentials::CREDENTIAL_PREFIX)
            && !is_credential_reference(text)
        {
            return Err("Invalid HTTP credential reference".into());
        }
        if !text.is_empty() && !is_credential_reference(text) {
            *value = Value::String(protect(text)?);
        }
        Ok(())
    }
    if let Some(Value::Object(headers)) = inputs.get_mut("headers") {
        for (name, value) in headers {
            if requires_protection(name) {
                field(value, &mut protect)?;
            }
        }
    }
    if let Some(Value::Object(auth)) = inputs.get_mut("authentication") {
        match auth.get("type").and_then(Value::as_str).unwrap_or("none") {
            "bearer" => {
                if let Some(value) = auth.get_mut("token") {
                    field(value, &mut protect)?;
                }
            }
            "basic" => {
                for key in ["username", "password"] {
                    if let Some(value) = auth.get_mut(key) {
                        field(value, &mut protect)?;
                    }
                }
            }
            "api_key" => {
                if let Some(value) = auth.get_mut("api_key") {
                    field(value, &mut protect)?;
                }
            }
            "custom_headers" => {
                if let Some(Value::Object(headers)) = auth.get_mut("headers") {
                    for value in headers.values_mut() {
                        field(value, &mut protect)?;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn literal_authentication_is_protected_once_and_references_survive_recovery() {
        let mut values = HashMap::from([
            (
                "headers".into(),
                json!({"Authorization":"Bearer private-header","Content-Type":"application/json"}),
            ),
            (
                "authentication".into(),
                json!({"type":"basic","username":"private-user","password":"private-password"}),
            ),
            ("body".into(), json!({"public":"keep this payload"})),
        ]);
        let mut stored = Vec::new();
        rewrite_inputs(&mut values, |secret| {
            stored.push(secret.to_string());
            Ok(format!(
                "{}{}",
                crate::services::api::credentials::CREDENTIAL_PREFIX,
                uuid::Uuid::new_v4()
            ))
        })
        .unwrap();
        assert_eq!(stored.len(), 3);
        let checkpoint = serde_json::to_string(&values).unwrap();
        assert!(!checkpoint.contains("private-"));
        assert!(checkpoint.contains("keep this payload"));
        let mut recovered: HashMap<String, Value> = serde_json::from_str(&checkpoint).unwrap();
        rewrite_inputs(&mut recovered, |_| {
            panic!("existing generation must not be replaced")
        })
        .unwrap();
        assert_eq!(recovered, values);
        values.insert(
            "authentication".into(),
            json!({"type":"custom_headers","headers":{"X-Service":"custom-private"}}),
        );
        assert!(
            rewrite_inputs(&mut values, |_| Err("credential store unavailable".into())).is_err()
        );
    }
}
