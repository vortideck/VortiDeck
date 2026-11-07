use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::Manager;

fn empty_widget_config() -> serde_json::Value {
    serde_json::json!({})
}

const OVERLAY_RUNTIME_STORE: &str = "overlay_runtime.json";
const OVERLAY_RUNTIME_KEY: &str = "active_overlay";
const OVERLAY_RUNTIME_SCHEMA_VERSION: u16 = 1;

fn overlay_runtime_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    app_handle
        .path()
        .app_data_dir()
        .map(|directory| directory.join(OVERLAY_RUNTIME_STORE))
        .map_err(|error| format!("Failed to resolve overlay runtime path: {error}"))
}

fn decode_overlay_runtime(bytes: &[u8]) -> Result<Option<PersistedOverlayRuntime>, String> {
    let root: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("Failed to decode overlay runtime store: {error}"))?;
    let Some(value) = root.get(OVERLAY_RUNTIME_KEY) else {
        return Ok(None);
    };
    let snapshot: PersistedOverlayRuntime = serde_json::from_value(value.clone())
        .map_err(|error| format!("Failed to decode saved overlay runtime: {error}"))?;
    if snapshot.schema_version != OVERLAY_RUNTIME_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported overlay runtime schema {}",
            snapshot.schema_version
        ));
    }
    Ok(Some(snapshot))
}

fn write_overlay_runtime_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or("Overlay runtime path has no parent directory")?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("Failed to create overlay runtime directory: {error}"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("Failed to create overlay runtime temporary file: {error}"))?;
    temporary
        .write_all(bytes)
        .and_then(|_| temporary.flush())
        .and_then(|_| temporary.as_file().sync_all())
        .map_err(|error| format!("Failed to write overlay runtime temporary file: {error}"))?;
    temporary.persist(path).map_err(|error| {
        format!(
            "Failed to commit overlay runtime atomically: {}",
            error.error
        )
    })?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

impl Default for Position {
    fn default() -> Self {
        Self { x: 50.0, y: 50.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Default for Size {
    fn default() -> Self {
        Self {
            width: 1920.0,
            height: 1080.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayElement {
    #[serde(default = "default_schema_version")]
    pub schema_version: u16,
    #[serde(default)]
    pub revision: u64,
    pub id: String,
    pub element_type: String,
    #[serde(default)]
    pub package_id: Option<String>,
    #[serde(default)]
    pub contribution_id: Option<String>,
    #[serde(default = "empty_widget_config")]
    pub config: serde_json::Value,
    #[serde(default)]
    pub config_revision: u64,
    /// Consent hash of the discovered package manifest whose content the
    /// trusted editor rendered into `render_html`. A grant for a newer package
    /// version must never revive an older cached iframe.
    #[serde(default)]
    pub package_content_hash: Option<String>,
    #[serde(default)]
    pub schema_id: Option<String>,
    #[serde(default)]
    pub trust: Option<String>,
    #[serde(default)]
    pub capabilities: Option<serde_json::Value>,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    pub content: String,
    pub position: Position,
    pub size: Size,
    pub style: Option<String>,
    pub visible: bool,
    // New field for custom HTML rendering - allows any custom element
    pub render_html: Option<String>,
    // Custom properties for dynamic elements
    pub custom_props: Option<serde_json::Value>,
    // Stack order, rendered as CSS z-index on the OUTER `.overlay-element` div
    // (the sibling that actually controls stacking). Kept off `render_html` so it
    // survives the overlay's fetch+diff, which rewrites the outer style attribute.
    #[serde(default)]
    pub z_index: Option<i64>,
}

// Legacy V1 constructors removed - V3 uses dynamic streaming elements from marketplace

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayDesign {
    pub name: String,
    pub canvas_size: Size,
    pub elements: Vec<OverlayElement>,
    pub background_color: Option<String>,
    pub background_image: Option<String>,
}

impl Default for OverlayDesign {
    fn default() -> Self {
        Self {
            name: "Default Overlay".to_string(),
            canvas_size: Size::default(),
            elements: Vec::new(),
            background_color: Some("transparent".to_string()),
            background_image: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OverlayState {
    pub current_design: OverlayDesign,
    pub generated_html: String,
    pub last_updated: u64,
    /// Packages whose saved elements remain in the editable design but must not
    /// be rendered until current authority is restored. This keeps revocation
    /// effective across reconnect/full-design sync without destroying layout
    /// state during an update or uninstall transaction.
    suspended_packages: std::collections::HashSet<String>,
    /// Effective capability scopes supplied by the canonical native permission
    /// checker. Package-authored/render-cache attributes are never delivery
    /// authority: every regeneration projects these scopes onto the document.
    effective_package_scopes:
        std::collections::HashMap<String, vortideck_core::plugins::EffectivePluginScope>,
    /// Current manifest hash paired with each effective scope. Both must match
    /// an element's render provenance before community content is emitted.
    effective_package_content_hashes: std::collections::HashMap<String, String>,
    /// Theme CSS vars (--theme-*) baked into the overlay :root for first paint
    /// and updated live via UPDATE_VARS. Overlay-wide runtime state (not part of
    /// the saved design).
    pub theme_vars: std::collections::HashMap<String, String>,
    pub publication_receipt: Option<SavedPublicationReceipt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedPublicationReceipt {
    pub receipt: vortideck_core::services::overlay_authoring::PublicationReceipt,
    pub output_hash: String,
}

impl OverlayState {
    pub fn new() -> Self {
        let default_design = OverlayDesign::default();
        let theme_vars = std::collections::HashMap::new();
        let generated_html = super::html::generate_overlay_html(&default_design, &theme_vars);

        Self {
            current_design: default_design,
            generated_html,
            last_updated: chrono::Utc::now().timestamp_millis() as u64,
            suspended_packages: std::collections::HashSet::new(),
            effective_package_scopes: std::collections::HashMap::new(),
            effective_package_content_hashes: std::collections::HashMap::new(),
            theme_vars,
            publication_receipt: None,
        }
    }

    /// Restore the last design that was accepted by the native overlay runtime.
    ///
    /// Browser localStorage remains the editor's project library, while this
    /// host-owned snapshot is the canonical active output used by OBS before
    /// the editor is opened after an application restart.
    pub fn load_persisted(&mut self, app_handle: &tauri::AppHandle) -> Result<bool, String> {
        let path = overlay_runtime_path(app_handle)?;
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(format!("Failed to read overlay runtime: {error}")),
        };
        let Some(snapshot) = decode_overlay_runtime(&bytes)? else {
            return Ok(false);
        };

        self.current_design = snapshot.design;
        self.theme_vars = snapshot.theme_vars;
        self.publication_receipt = snapshot.publication_receipt;
        self.regenerate_html();
        self.update_timestamp();
        Ok(true)
    }

    /// Persist only durable render state. Process-scoped package authority and
    /// capability data are deliberately reconstructed after every startup.
    pub fn persist(&self, app_handle: &tauri::AppHandle) -> Result<(), String> {
        let snapshot = PersistedOverlayRuntime {
            schema_version: OVERLAY_RUNTIME_SCHEMA_VERSION,
            design: self.current_design.clone(),
            theme_vars: self.theme_vars.clone(),
            publication_receipt: self.valid_publication_receipt(),
        };
        let mut root = serde_json::Map::new();
        root.insert(
            OVERLAY_RUNTIME_KEY.to_string(),
            serde_json::to_value(snapshot)
                .map_err(|error| format!("Failed to encode overlay runtime: {error}"))?,
        );
        let bytes = serde_json::to_vec(&root)
            .map_err(|error| format!("Failed to encode overlay runtime store: {error}"))?;
        write_overlay_runtime_atomically(&overlay_runtime_path(app_handle)?, &bytes)
    }

    pub fn output_hash(&self) -> Result<String, String> {
        use sha2::{Digest, Sha256};
        fn canonical(value: serde_json::Value) -> serde_json::Value {
            match value {
                serde_json::Value::Object(fields) => {
                    let ordered: std::collections::BTreeMap<_,_> = fields.into_iter().map(|(key,value)| (key,canonical(value))).collect();
                    serde_json::Value::Object(ordered.into_iter().collect())
                },
                serde_json::Value::Array(values) => serde_json::Value::Array(values.into_iter().map(canonical).collect()),
                other => other,
            }
        }
        let value = canonical(serde_json::json!({"design": self.current_design, "theme": self.theme_vars}));
        let bytes = serde_json::to_vec(&value).map_err(|_| "Could not fingerprint live overlay")?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }

    pub fn valid_publication_receipt(&self) -> Option<SavedPublicationReceipt> {
        self.publication_receipt.as_ref().filter(|saved|
            self.output_hash().is_ok_and(|hash| hash == saved.output_hash)).cloned()
    }

    /// Migrate the pre-runtime store only when no current snapshot was loaded.
    /// Observed OBS dimensions are not an ongoing authority for the design.
    pub fn load_legacy_canvas_size(&mut self, app_handle: &tauri::AppHandle) {
        use tauri_plugin_store::StoreExt;

        if let Ok(store) = app_handle.store_builder("obs_settings.json").build() {
            if let Some(canvas_data) = store.get("obs_canvas_size") {
                if let (Some(width), Some(height)) = (
                    canvas_data.get("width").and_then(|w| w.as_f64()),
                    canvas_data.get("height").and_then(|h| h.as_f64()),
                ) {
                    if !width.is_finite()
                        || !height.is_finite()
                        || width.fract() != 0.0
                        || height.fract() != 0.0
                        || !(320.0..=7680.0).contains(&width)
                        || !(320.0..=4320.0).contains(&height)
                    {
                        log::warn!("Ignoring invalid legacy overlay canvas dimensions");
                        return;
                    }
                    println!("📐 Loading saved canvas size: {}×{}", width, height);

                    // Update canvas size without scaling elements (they should maintain positions)
                    self.current_design.canvas_size.width = width;
                    self.current_design.canvas_size.height = height;

                    // Regenerate HTML with new canvas dimensions
                    self.regenerate_html();
                    self.update_timestamp();

                    println!("✅ Canvas size loaded from persistent store");
                    return;
                }
            }
        }

        println!("ℹ️ No saved canvas size found, using default 1920×1080");
    }

    pub fn add_element(&mut self, mut element: OverlayElement) {
        if element.revision == 0 {
            element.revision = 1;
        }
        self.current_design.elements.push(element);
        self.regenerate_html();
        self.update_timestamp();
    }

    pub fn remove_element(&mut self, element_id: &str) -> bool {
        let initial_len = self.current_design.elements.len();
        self.current_design.elements.retain(|e| e.id != element_id);

        if self.current_design.elements.len() < initial_len {
            self.regenerate_html();
            self.update_timestamp();
            true
        } else {
            false
        }
    }

    /// Remove every live element owned by a package whose authority changed.
    /// Canonical package IDs prevent `plugin.foo` aliases from retaining stale
    /// frames after the `foo` grant is revoked.
    pub fn remove_package_elements(&mut self, package_id: &str) -> Vec<String> {
        let canonical = vortideck_core::plugins::canonical_package_id(package_id);
        let mut removed = Vec::new();
        self.current_design.elements.retain(|element| {
            let belongs_to_package = element
                .package_id
                .as_deref()
                .is_some_and(|id| vortideck_core::plugins::canonical_package_id(id) == canonical);
            if belongs_to_package {
                removed.push(element.id.clone());
            }
            !belongs_to_package
        });
        if !removed.is_empty() {
            self.regenerate_html();
            self.update_timestamp();
        }
        removed
    }

    /// Hide package-owned elements from every live and future overlay document
    /// while preserving their saved layout for a possible successful re-grant
    /// or transactional rollback.
    pub fn suspend_package(&mut self, package_id: &str) -> Vec<String> {
        let canonical = vortideck_core::plugins::canonical_package_id(package_id).to_string();
        let affected = self
            .current_design
            .elements
            .iter()
            .filter(|element| {
                element.package_id.as_deref().is_some_and(|id| {
                    vortideck_core::plugins::canonical_package_id(id) == canonical
                })
            })
            .map(|element| element.id.clone())
            .collect::<Vec<_>>();
        let scope_removed = self.effective_package_scopes.remove(&canonical).is_some();
        let hash_removed = self
            .effective_package_content_hashes
            .remove(&canonical)
            .is_some();
        if self.suspended_packages.insert(canonical) || scope_removed || hash_removed {
            self.regenerate_html();
            self.update_timestamp();
        }
        affected
    }

    /// Restore rendering after current grants/runtime state have been proven.
    pub fn resume_package(
        &mut self,
        package_id: &str,
        scope: vortideck_core::plugins::EffectivePluginScope,
        content_hash: String,
    ) -> Vec<String> {
        let canonical = vortideck_core::plugins::canonical_package_id(package_id).to_string();
        let was_suspended = self.suspended_packages.remove(&canonical);
        let scope_changed = self
            .effective_package_scopes
            .insert(canonical.clone(), scope)
            .as_ref()
            != self.effective_package_scopes.get(&canonical);
        let hash_changed = self
            .effective_package_content_hashes
            .insert(canonical.clone(), content_hash)
            .as_ref()
            != self.effective_package_content_hashes.get(&canonical);
        let restored = self
            .current_design
            .elements
            .iter()
            .filter(|element| {
                element.package_id.as_deref().is_some_and(|id| {
                    vortideck_core::plugins::canonical_package_id(id) == canonical
                }) && element.package_content_hash.as_deref()
                    == self
                        .effective_package_content_hashes
                        .get(&canonical)
                        .map(String::as_str)
            })
            .map(|element| element.id.clone())
            .collect::<Vec<_>>();
        if was_suspended || scope_changed || hash_changed || !restored.is_empty() {
            self.regenerate_html();
            self.update_timestamp();
        }
        restored
    }

    pub fn update_element(
        &mut self,
        element_id: &str,
        updater: impl FnOnce(&mut OverlayElement),
    ) -> bool {
        if let Some(element) = self
            .current_design
            .elements
            .iter_mut()
            .find(|e| e.id == element_id)
        {
            updater(element);
            element.revision = element.revision.saturating_add(1);
            self.regenerate_html();
            self.update_timestamp();
            true
        } else {
            false
        }
    }

    pub fn get_element(&self, element_id: &str) -> Option<&OverlayElement> {
        self.current_design
            .elements
            .iter()
            .find(|e| e.id == element_id)
    }

    /// A saved instance and an old DB consent row are insufficient authority
    /// for an iframe left alive during hide, suspension or package replacement.
    pub fn ensure_widget_action_authority(
        &self,
        element_id: &str,
        package_id: &str,
        action: &str,
    ) -> Result<(), String> {
        use vortideck_core::plugins::canonical_package_id;
        let element = self
            .get_element(element_id)
            .ok_or("Overlay element instance is no longer active")?;
        let canonical = canonical_package_id(package_id);
        if element.package_id.as_deref().map(canonical_package_id) != Some(canonical) {
            return Err("Widget package identity does not match the placed element".into());
        }
        if !element.visible || self.suspended_packages.contains(canonical) {
            return Err("Widget instance is hidden or suspended".into());
        }
        let scope = self
            .effective_package_scopes
            .get(canonical)
            .ok_or("Widget package has no current action authority")?;
        let current_hash = self
            .effective_package_content_hashes
            .get(canonical)
            .ok_or("Widget package has no current render authority")?;
        if element.package_content_hash.as_ref() != Some(current_hash) {
            return Err("Widget content is from an outdated package".into());
        }
        let declared = element
            .capabilities
            .as_ref()
            .and_then(|value| value.get("actions"))
            .and_then(serde_json::Value::as_array)
            .is_some_and(|actions| actions.iter().any(|value| value.as_str() == Some(action)));
        if !declared || !scope.actions.iter().any(|allowed| allowed == action) {
            return Err(
                "Widget action is not allowed by the contribution and current grant".into(),
            );
        }
        Ok(())
    }

    pub fn clear_elements(&mut self) {
        self.current_design.elements.clear();
        self.regenerate_html();
        self.update_timestamp();
    }

    pub fn set_design(&mut self, design: OverlayDesign) {
        self.current_design = design;
        self.regenerate_html();
        self.update_timestamp();
    }

    pub fn regenerate_html(&mut self) {
        let mut effective_design = self.current_design.clone();
        effective_design.elements = effective_design
            .elements
            .into_iter()
            .filter_map(|mut element| {
                let Some(package_id) = element.package_id.as_deref() else {
                    return Some(element);
                };
                let canonical = vortideck_core::plugins::canonical_package_id(package_id);
                if self.suspended_packages.contains(canonical) {
                    return None;
                }
                if element.trust.as_deref() == Some("builtin") {
                    return Some(element);
                }
                let scope = self.effective_package_scopes.get(canonical)?;
                let current_content_hash = self.effective_package_content_hashes.get(canonical)?;
                if element.package_content_hash.as_ref() != Some(current_content_hash) {
                    return None;
                }
                element.capabilities = Some(serde_json::json!({
                    "subscribe": &scope.subscriptions,
                    "actions": &scope.actions,
                    "network": if scope.unrestricted_network {
                        serde_json::Value::String("all".to_string())
                    } else if scope.network_allowlist.is_empty() {
                        serde_json::Value::String("none".to_string())
                    } else {
                        serde_json::json!(&scope.network_allowlist)
                    },
                }));
                Some(element)
            })
            .collect();
        self.generated_html =
            super::html::generate_overlay_html(&effective_design, &self.theme_vars);
    }

    /// Use the same grant/content generation required to render this instance.
    pub fn is_element_renderable(&self, element: &OverlayElement) -> bool {
        if !element.visible { return false; }
        let Some(package) = element.package_id.as_deref() else { return true; };
        let canonical = vortideck_core::plugins::canonical_package_id(package);
        !self.suspended_packages.contains(canonical) && (element.trust.as_deref() == Some("builtin") ||
            (self.effective_package_scopes.contains_key(canonical) &&
             self.effective_package_content_hashes.get(canonical).is_some_and(|hash| element.package_content_hash.as_ref() == Some(hash))))
    }

    /// Store theme vars and regenerate the overlay HTML (bakes them into :root
    /// for future overlay loads), bumping the timestamp. Live recoloring of
    /// already-open overlays is handled separately by broadcasting UPDATE_VARS.
    pub fn set_theme_vars(&mut self, vars: std::collections::HashMap<String, String>) {
        self.theme_vars = vars;
        self.regenerate_html();
        self.update_timestamp();
    }

    pub fn update_timestamp(&mut self) {
        self.last_updated = chrono::Utc::now().timestamp_millis() as u64;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedOverlayRuntime {
    schema_version: u16,
    design: OverlayDesign,
    #[serde(default)]
    theme_vars: std::collections::HashMap<String, String>,
    #[serde(default)]
    publication_receipt: Option<SavedPublicationReceipt>,
}

impl Default for OverlayState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publication_receipt_survives_map_reordering_but_never_acks_changed_output() {
        use vortideck_core::services::overlay_authoring::{CompositionContent, PublicationReceipt};
        let mut state = OverlayState::new();
        state.theme_vars.insert("--brand".into(), "blue".into());
        state.theme_vars.insert("--text".into(), "white".into());
        let hash = state.output_hash().unwrap();
        state.publication_receipt = Some(SavedPublicationReceipt { output_hash: hash.clone(),
            receipt: PublicationReceipt { id: "gameplay".into(), expected_revision: 1,
                content: CompositionContent { name: "Gameplay".into(), elements: vec![], settings: serde_json::json!({"canvasWidth":1920,"canvasHeight":1080}) } } });
        state.theme_vars.clear();
        state.theme_vars.insert("--text".into(), "white".into());
        state.theme_vars.insert("--brand".into(), "blue".into());
        assert_eq!(state.output_hash().unwrap(), hash);
        assert!(state.valid_publication_receipt().is_some());
        state.current_design.canvas_size.width = 1080.0;
        assert!(state.valid_publication_receipt().is_none());
    }

    fn encoded_runtime(name: &str, schema_version: u16) -> Vec<u8> {
        let mut root = serde_json::Map::new();
        root.insert(
            OVERLAY_RUNTIME_KEY.to_string(),
            serde_json::json!({
                "schema_version": schema_version,
                "design": {
                    "name": name,
                    "canvas_size": { "width": 1920.0, "height": 1080.0 },
                    "elements": [],
                    "background_color": "transparent",
                    "background_image": null
                },
                "theme_vars": {}
            }),
        );
        serde_json::to_vec(&root).unwrap()
    }

    #[test]
    fn malformed_and_incompatible_runtime_snapshots_fail_without_a_candidate() {
        assert!(decode_overlay_runtime(b"{not-json")
            .unwrap_err()
            .contains("decode overlay runtime store"));
        assert!(decode_overlay_runtime(&encoded_runtime("future", 99))
            .unwrap_err()
            .contains("Unsupported overlay runtime schema 99"));
    }

    #[test]
    fn atomic_runtime_commit_survives_interrupted_temp_and_repeated_edits() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join(OVERLAY_RUNTIME_STORE);
        write_overlay_runtime_atomically(&destination, &encoded_runtime("first", 1)).unwrap();

        // A process interruption before persist leaves the committed path
        // untouched; NamedTempFile removes the abandoned candidate on drop.
        {
            let mut interrupted = tempfile::NamedTempFile::new_in(directory.path()).unwrap();
            interrupted.write_all(b"{partial").unwrap();
            interrupted.as_file().sync_all().unwrap();
        }
        let first = decode_overlay_runtime(&std::fs::read(&destination).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(first.design.name, "first");

        for name in ["second", "third", "final"] {
            write_overlay_runtime_atomically(&destination, &encoded_runtime(name, 1)).unwrap();
        }
        let final_snapshot = decode_overlay_runtime(&std::fs::read(destination).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(final_snapshot.design.name, "final");
    }

    #[test]
    fn legacy_overlay_elements_migrate_to_typed_defaults() {
        let element: OverlayElement = serde_json::from_value(serde_json::json!({
            "id": "legacy-1",
            "element_type": "custom",
            "content": "",
            "position": { "x": 1.0, "y": 2.0 },
            "size": { "width": 320.0, "height": 180.0 },
            "style": null,
            "visible": true,
            "render_html": "<p>legacy</p>",
            "custom_props": null
        }))
        .expect("legacy element should remain readable");

        assert_eq!(element.schema_version, 1);
        assert_eq!(element.revision, 0);
        assert_eq!(element.config, serde_json::json!({}));
        assert_eq!(element.config_revision, 0);
        assert!(element.contribution_id.is_none());
        assert!(element.package_id.is_none());
        assert!(element.package_content_hash.is_none());
        assert!(element.capabilities.is_none());
    }

    #[test]
    fn authoritative_updates_increment_element_revision() {
        let mut state = OverlayState::new();
        state.add_element(OverlayElement {
            contribution_id: None,
            config: serde_json::json!({}),
            config_revision: 0,
            schema_version: 1,
            revision: 0,
            id: "widget-1".to_string(),
            element_type: "widget".to_string(),
            package_id: Some("alerts".to_string()),
            package_content_hash: Some("alerts-v1".to_string()),
            schema_id: Some("raid-card".to_string()),
            trust: Some("community".to_string()),
            capabilities: Some(serde_json::json!({ "subscribe": ["raid"] })),
            data: Some(serde_json::json!({ "title": "Raid" })),
            content: String::new(),
            position: Position::default(),
            size: Size::default(),
            style: None,
            visible: true,
            render_html: Some("<p>raid</p>".to_string()),
            custom_props: None,
            z_index: Some(2),
        });
        assert_eq!(state.get_element("widget-1").unwrap().revision, 1);
        assert!(state.update_element("widget-1", |element| element.visible = false));
        assert_eq!(state.get_element("widget-1").unwrap().revision, 2);
    }

    #[test]
    fn persisted_runtime_restores_render_state_without_process_authority() {
        let mut state = OverlayState::new();
        state.resume_package(
            "alerts",
            vortideck_core::plugins::EffectivePluginScope {
                subscriptions: vec!["follow".into()],
                ..Default::default()
            },
            "alerts-v1".into(),
        );
        state.add_element(OverlayElement {
            contribution_id: None,
            config: serde_json::json!({}),
            config_revision: 0,
            schema_version: 1,
            revision: 1,
            id: "saved-widget".into(),
            element_type: "widget".into(),
            package_id: Some("alerts".into()),
            package_content_hash: Some("alerts-v1".into()),
            schema_id: None,
            trust: Some("community".into()),
            capabilities: None,
            data: None,
            content: String::new(),
            position: Position::default(),
            size: Size::default(),
            style: None,
            visible: true,
            render_html: Some("<p>saved widget</p>".into()),
            custom_props: None,
            z_index: None,
        });
        state
            .theme_vars
            .insert("--theme-primary".into(), "#123456".into());

        let encoded = serde_json::to_value(PersistedOverlayRuntime {
            schema_version: OVERLAY_RUNTIME_SCHEMA_VERSION,
            design: state.current_design.clone(),
            theme_vars: state.theme_vars.clone(),
            publication_receipt: None,
        })
        .unwrap();
        let encoded_text = encoded.to_string();
        assert!(!encoded_text.contains("effective_package_scopes"));
        assert!(!encoded_text.contains("overlayToken"));

        let decoded: PersistedOverlayRuntime = serde_json::from_value(encoded).unwrap();
        let mut restored = OverlayState::new();
        restored.current_design = decoded.design;
        restored.theme_vars = decoded.theme_vars;
        restored.regenerate_html();

        assert!(restored.get_element("saved-widget").is_some());
        assert!(!restored.generated_html.contains("saved widget"));
        assert!(restored.generated_html.contains("--theme-primary:#123456"));
    }

    #[test]
    fn revoking_package_removes_only_its_elements_and_future_html() {
        let mut state = OverlayState::new();
        for package in ["alerts", "goals"] {
            state.resume_package(
                package,
                vortideck_core::plugins::EffectivePluginScope {
                    subscriptions: vec!["follow".into()],
                    ..Default::default()
                },
                format!("{package}-v1"),
            );
        }
        for (id, package) in [("revoked", "plugin.alerts"), ("remaining", "goals")] {
            state.add_element(OverlayElement {
                contribution_id: None,
                config: serde_json::json!({}),
                config_revision: 0,
                schema_version: 1,
                revision: 1,
                id: id.into(),
                element_type: "widget".into(),
                package_id: Some(package.into()),
                package_content_hash: Some(format!(
                    "{}-v1",
                    vortideck_core::plugins::canonical_package_id(package)
                )),
                schema_id: None,
                trust: Some("community".into()),
                capabilities: Some(serde_json::json!({ "subscribe": ["follow"] })),
                data: None,
                content: String::new(),
                position: Position::default(),
                size: Size::default(),
                style: None,
                visible: true,
                render_html: Some(format!("<p>{id}</p>")),
                custom_props: None,
                z_index: None,
            });
        }

        assert_eq!(state.remove_package_elements("alerts"), vec!["revoked"]);
        assert!(state.get_element("revoked").is_none());
        assert!(state.get_element("remaining").is_some());
        assert!(!state.generated_html.contains("<p>revoked</p>"));
        assert!(state.generated_html.contains("<p>remaining</p>"));
    }

    #[test]
    fn suspension_survives_design_sync_and_can_be_resumed_without_layout_loss() {
        let mut state = OverlayState::new();
        let element = OverlayElement {
            contribution_id: None,
            config: serde_json::json!({}),
            config_revision: 0,
            schema_version: 1,
            revision: 1,
            id: "alerts-1".into(),
            element_type: "widget".into(),
            package_id: Some("plugin.alerts".into()),
            package_content_hash: Some("alerts-v1".into()),
            schema_id: None,
            trust: Some("community".into()),
            capabilities: Some(serde_json::json!({ "subscribe": ["follow", "viewerCount"] })),
            data: None,
            content: String::new(),
            position: Position::default(),
            size: Size::default(),
            style: None,
            visible: true,
            render_html: Some("<p>alerts</p>".into()),
            custom_props: None,
            z_index: None,
        };
        state.add_element(element.clone());

        assert_eq!(state.suspend_package("alerts"), vec!["alerts-1"]);
        assert!(state.get_element("alerts-1").is_some());
        assert!(!state.generated_html.contains("<p>alerts</p>"));

        state.set_design(OverlayDesign {
            name: "resynced".into(),
            canvas_size: Size::default(),
            elements: vec![element],
            background_color: None,
            background_image: None,
        });
        assert!(!state.generated_html.contains("<p>alerts</p>"));

        assert_eq!(
            state.resume_package(
                "plugin.alerts",
                vortideck_core::plugins::EffectivePluginScope {
                    subscriptions: vec!["follow".into()],
                    ..Default::default()
                },
                "alerts-v1".into(),
            ),
            vec!["alerts-1"]
        );
        assert!(state.generated_html.contains("<p>alerts</p>"));
        assert!(state
            .generated_html
            .contains("data-authorized-subscribe=\"follow\""));
        assert!(!state.generated_html.contains("follow,viewerCount"));
    }

    #[test]
    fn suspension_before_first_design_sync_blocks_late_stale_elements() {
        let mut state = OverlayState::new();
        assert!(state.suspend_package("alerts").is_empty());
        state.set_design(OverlayDesign {
            name: "late sync".into(),
            canvas_size: Size::default(),
            elements: vec![OverlayElement {
                contribution_id: None,
                config: serde_json::json!({}),
                config_revision: 0,
                schema_version: 1,
                revision: 1,
                id: "late-alert".into(),
                element_type: "widget".into(),
                package_id: Some("plugin.alerts".into()),
                package_content_hash: Some("alerts-v1".into()),
                schema_id: None,
                trust: Some("community".into()),
                capabilities: None,
                data: None,
                content: String::new(),
                position: Position::default(),
                size: Size::default(),
                style: None,
                visible: true,
                render_html: Some("<p>late stale alert</p>".into()),
                custom_props: None,
                z_index: None,
            }],
            background_color: None,
            background_image: None,
        });

        assert!(state.get_element("late-alert").is_some());
        assert!(!state.generated_html.contains("late stale alert"));
    }

    #[test]
    fn widget_actions_require_live_instance_and_current_render_authority() {
        let mut state = OverlayState::new();
        let element: OverlayElement = serde_json::from_value(serde_json::json!({
            "id": "widget-1", "element_type": "widget", "package_id": "alerts",
            "package_content_hash": "v1", "trust": "community", "content": "",
            "capabilities": {"actions": ["triggerFlow:one", "triggerFlow:ungranted"]},
            "position": {"x": 320, "y": 220}, "size": {"width": 480, "height": 160},
            "visible": true
        }))
        .unwrap();
        state.add_element(element);
        let allowed = |state: &OverlayState| {
            state.ensure_widget_action_authority("widget-1", "plugin.alerts", "triggerFlow:one")
        };
        assert!(
            allowed(&state).is_err(),
            "persisted identity is not a live grant"
        );
        let scope = vortideck_core::plugins::EffectivePluginScope {
            actions: vec!["triggerFlow:one".into()],
            ..Default::default()
        };
        state.resume_package("alerts", scope.clone(), "v1".into());
        assert!(allowed(&state).is_ok());
        assert!(state
            .ensure_widget_action_authority("widget-1", "other", "triggerFlow:one")
            .is_err());
        assert!(state
            .ensure_widget_action_authority("widget-1", "alerts", "triggerFlow:ungranted")
            .is_err());
        state.update_element("widget-1", |element| element.visible = false);
        assert!(allowed(&state).is_err());
        state.update_element("widget-1", |element| element.visible = true);
        state.suspend_package("alerts");
        assert!(allowed(&state).is_err());
        state.resume_package("alerts", scope.clone(), "v2".into());
        assert!(
            allowed(&state).is_err(),
            "new consent must not revive an old frame"
        );
        state.update_element("widget-1", |element| {
            element.package_content_hash = Some("v2".into())
        });
        assert!(allowed(&state).is_ok());
        state.remove_element("widget-1");
        assert!(allowed(&state).is_err());
    }

    #[test]
    fn newer_grant_does_not_revive_cached_content_from_an_older_manifest() {
        let mut state = OverlayState::new();
        state.add_element(OverlayElement {
            contribution_id: None,
            config: serde_json::json!({}),
            config_revision: 0,
            schema_version: 1,
            revision: 1,
            id: "stale-widget".into(),
            element_type: "widget".into(),
            package_id: Some("alerts".into()),
            package_content_hash: Some("alerts-v1".into()),
            schema_id: None,
            trust: Some("community".into()),
            capabilities: None,
            data: None,
            content: String::new(),
            position: Position::default(),
            size: Size::default(),
            style: None,
            visible: true,
            render_html: Some("<p>old vulnerable content</p>".into()),
            custom_props: None,
            z_index: None,
        });
        state.suspend_package("alerts");

        state.resume_package(
            "alerts",
            vortideck_core::plugins::EffectivePluginScope {
                subscriptions: vec!["follow".into()],
                ..Default::default()
            },
            "alerts-v2".into(),
        );

        assert!(state.get_element("stale-widget").is_some());
        assert!(!state.generated_html.contains("old vulnerable content"));
    }
}

fn default_schema_version() -> u16 {
    1
}
