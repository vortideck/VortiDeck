//! Portable editable setups, not serialized runtime caches or permission grants.
use super::overlay_authoring::{CompositionContent, CompositionDocument};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const MAX_SETUP_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageDependency {
    pub version: String,
    pub consent_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortableSetup {
    pub format: String,
    pub version: u16,
    pub compositions: Vec<CompositionContent>,
    pub packages: BTreeMap<String, PackageDependency>,
}

fn secret_key(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().replace(['-', '_'], "").as_str(),
        "authorization"
            | "proxyauthorization"
            | "cookie"
            | "setcookie"
            | "password"
            | "accesstoken"
            | "refreshtoken"
            | "overlaytoken"
            | "apikey"
            | "xapikey"
            | "clientsecret"
    )
}

fn check_value(value: &Value, diagnostics: &mut Vec<Value>) -> Result<(), String> {
    match value {
        Value::Object(fields) => {
            for (key, value) in fields {
                if secret_key(key) && !value.is_null() && value != "" {
                    return Err(format!(
                        "Remove private '{key}' values before sharing this setup"
                    ));
                }
                check_value(value, diagnostics)?;
            }
        }
        Value::Array(values) => {
            for value in values {
                check_value(value, diagnostics)?;
            }
        }
        Value::String(text) => {
            if text.contains("vortideck-credential://") {
                return Err("Credential references cannot be exported or imported".into());
            }
            if let Ok(url) = url::Url::parse(text) {
                if !url.username().is_empty()
                    || url.password().is_some()
                    || url.query_pairs().any(|(key, _)| secret_key(&key))
                {
                    return Err("Remove credentials from asset/service URLs before sharing".into());
                }
                if matches!(url.scheme(), "file" | "asset" | "tauri") {
                    diagnostics.push(json!({"code":"local_asset","message":"A machine-local asset needs replacement or embedding on this computer"}));
                }
                if url.scheme() == "data"
                    && ![
                        "data:image/png;base64,",
                        "data:image/jpeg;base64,",
                        "data:image/gif;base64,",
                        "data:image/webp;base64,",
                        "data:font/woff2;base64,",
                    ]
                    .iter()
                    .any(|prefix| text.starts_with(prefix))
                {
                    return Err(
                        "Portable embedded assets must be PNG, JPEG, GIF, WebP or WOFF2".into(),
                    );
                }
            }
            if text.contains(":\\") || text.starts_with("\\\\") {
                diagnostics.push(json!({"code":"local_asset","message":"A machine-local path needs an explicit replacement on import"}));
            }
        }
        _ => {}
    }
    Ok(())
}

fn clear_runtime(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            let element = fields.contains_key("id")
                && fields.contains_key("width")
                && fields.contains_key("x");
            let package = fields
                .get("widgetId")
                .or_else(|| fields.get("packageId"))
                .and_then(Value::as_str)
                .is_some();
            if element {
                for key in [
                    "selected",
                    "nativeSyncPending",
                    "configRevision",
                    "configSchema",
                    "packageInfo",
                    "packageContentHash",
                    "contentHash",
                    "is_granted",
                    "needs_reconsent",
                    "capabilities",
                    "trust",
                ] {
                    fields.remove(key);
                }
                if package {
                    for key in [
                        "customHTML",
                        "innerHTML",
                        "content",
                        "template",
                        "customJS",
                        "script",
                        "html",
                        "render_html",
                        "schemaId",
                    ] {
                        fields.remove(key);
                    }
                }
            }
            for value in fields.values_mut() {
                clear_runtime(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                clear_runtime(value);
            }
        }
        _ => {}
    }
}

impl PortableSetup {
    pub fn export(
        documents: &[CompositionDocument],
        packages: BTreeMap<String, PackageDependency>,
    ) -> Result<Self, String> {
        let mut compositions = Vec::with_capacity(documents.len());
        for document in documents {
            let mut value =
                serde_json::to_value(&document.content).map_err(|_| "Invalid composition")?;
            clear_runtime(&mut value);
            compositions
                .push(serde_json::from_value(value).map_err(|_| "Invalid portable composition")?);
        }
        let setup = Self {
            format: "vortideck-overlay-setup".into(),
            version: 1,
            compositions,
            packages,
        };
        setup.validate()?;
        Ok(setup)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_SETUP_BYTES {
            return Err("Setup exceeds 16 MiB".into());
        }
        let mut setup: Self =
            serde_json::from_slice(bytes).map_err(|_| "Invalid portable setup format")?;
        // Imported fields are data, never an authority to run package caches.
        for content in &mut setup.compositions {
            let mut value = serde_json::to_value(&*content).map_err(|_| "Invalid composition")?;
            clear_runtime(&mut value);
            *content = serde_json::from_value(value).map_err(|_| "Invalid composition")?;
            for element in &mut content.elements {
                // Indexing a non-object Value mutably panics; reject first.
                if !element.is_object() {
                    return Err("Composition elements must be objects".into());
                }
                let package = element
                    .get("widgetId")
                    .or_else(|| element.get("packageId"))
                    .and_then(Value::as_str)
                    .is_some();
                element["trust"] = json!(if package { "community" } else { "local" });
                if !package {
                    element["pendingImportReview"] = json!(true);
                }
            }
        }
        setup.validate()?;
        Ok(setup)
    }

    pub fn validate(&self) -> Result<Vec<Value>, String> {
        if self.format != "vortideck-overlay-setup" || self.version != 1 {
            return Err("Unsupported setup format/version".into());
        }
        if self.compositions.is_empty() || self.compositions.len() > 32 || self.packages.len() > 128
        {
            return Err(
                "Setup needs 1–32 compositions and at most 128 package dependencies".into(),
            );
        }
        if serde_json::to_vec(self).map_err(|_| "Invalid setup")?.len() > MAX_SETUP_BYTES {
            return Err("Setup exceeds 16 MiB".into());
        }
        let mut diagnostics = Vec::new();
        for content in &self.compositions {
            content.validate()?;
            let value = serde_json::to_value(content).map_err(|_| "Invalid composition")?;
            check_value(&value, &mut diagnostics)?;
            diagnostics.extend(content.diagnostics());
        }
        Ok(diagnostics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_editable_geometry_but_never_imports_package_code_or_authority() {
        let content = CompositionContent {
            name: "Gameplay".into(),
            elements: vec![
                json!({"id":"chat","x":40,"y":65,"width":300,"height":500,"widgetId":"chat-package","contributionId":"chat","trust":"builtin","capabilities":{"actions":["anything"]},"customHTML":"<script>bad()</script>","config":{"color":"purple"}}),
            ],
            settings: json!({"canvasWidth":1080,"canvasHeight":1920}),
        };
        let bytes = serde_json::to_vec(&PortableSetup {
            format: "vortideck-overlay-setup".into(),
            version: 1,
            compositions: vec![content],
            packages: BTreeMap::new(),
        })
        .unwrap();
        let imported = PortableSetup::decode(&bytes).unwrap();
        let element = &imported.compositions[0].elements[0];
        assert_eq!(element["x"], 40);
        assert_eq!(element["config"]["color"], "purple");
        assert!(element.get("customHTML").is_none());
        assert_eq!(element["trust"], "community");
        assert!(element.get("capabilities").is_none());
        let mut private = imported;
        private.compositions[0].elements[0]["config"]["api_key"] = json!("private");
        assert!(private.validate().is_err());
        private.compositions[0].elements[0]["config"] =
            json!({"url":"https://example.com/?access_token=private"});
        assert!(private.validate().is_err());
    }

    #[test]
    fn non_object_elements_are_rejected_without_panicking() {
        for element in [json!(1), json!("chat"), json!([1]), json!(true)] {
            let bytes = serde_json::to_vec(&json!({
                "format": "vortideck-overlay-setup",
                "version": 1,
                "compositions": [{
                    "name": "Gameplay",
                    "elements": [element],
                    "settings": {"canvasWidth": 1080, "canvasHeight": 1920}
                }],
                "packages": {}
            }))
            .unwrap();
            assert_eq!(
                PortableSetup::decode(&bytes).unwrap_err(),
                "Composition elements must be objects"
            );
        }
    }
}
