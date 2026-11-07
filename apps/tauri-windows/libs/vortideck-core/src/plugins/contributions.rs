//! Plugin contributions — typed declaration of what a package contributes.
//!
//! Generalizes the legacy `runtime` + `web` fields on `PluginManifest` into a
//! list of typed contributions (`widget`, `flow-node`, `ui-panel`, `macro`).
//! The legacy `runtime` / `entry` / `web` fields remain for the code/web
//! contribution (backward compatibility); a future schema version may fold
//! those into a `Code` variant here.
//!
//! See:
//! - `docs/WIDGET_CAPABILITY_ARCHITECTURE.md` §3.5 (Package & Contribution Model)
//! - `docs/WIDGET_CAPABILITY_DIALOG_PLAN.md` Phase 0

use super::bundle::PluginPermission;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Unified view of any installable package — the unit the "My Plugins" surface
/// lists and manages. Merges the runtime plugin registry (`PluginManager.plugins`)
/// with the available-widgets list (`available_widgets`) so the UI has ONE place
/// for everything: runtime plugins, widget packages, and bundles (plugin + widget).
///
/// "Capability" is the user-facing word for widget permissions; both widget
/// `capabilities` and legacy `permissions` compile down to `PluginPermission`
/// (one system). The CapabilityDialog is package-general — it reviews whatever a
/// package requests. See `docs/WIDGET_CAPABILITY_ARCHITECTURE.md` §4.
///
/// `is_granted` is true when the package is installed: a widget has a stored
/// package-permission grant row or a runtime plugin is loaded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Optional icon path (runtime plugins). Frontend converts via `convertFileSrc`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub icon_path: Option<String>,
    /// "builtin" | "verified" | "community".
    pub trust: String,
    /// Typed contributions (widget/flowNode/uiPanel/macro) copied from the
    /// package manifest and merged with runtime state by `PluginManager`.
    #[serde(default)]
    pub contributions: Vec<Contribution>,
    /// Legacy package-level permissions (PluginPermission). Empty for widget-only
    /// packages — they declare `capabilities` (compiled to permissions at install).
    #[serde(default)]
    pub permissions: Vec<super::bundle::PluginPermission>,
    /// True if this package contains a runtime (WASM/service) contribution.
    #[serde(default)]
    pub has_runtime: bool,
    /// True only while that runtime is loaded in this engine process. Consent
    /// can remain granted while activation is deferred by quota or a load error.
    #[serde(default)]
    pub runtime_active: bool,
    /// True if installed/granted (widget grant row exists, or runtime loaded).
    #[serde(default)]
    pub is_granted: bool,
    /// True if a granted widget's current content/capabilities differ from the
    /// stored grant (re-consent on update). Widget-only signal — runtime plugins
    /// have no persisted grant to compare yet.
    #[serde(default)]
    pub needs_reconsent: bool,
}

/// A folder that was scanned as a package but could not be safely discovered.
/// Keeping this separate from `PackageInfo` avoids treating malformed metadata
/// as executable authority while still making the failure visible to the UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageDiscoveryError {
    pub folder: String,
    pub manifest_path: String,
    pub error: String,
}

/// A typed contribution declared by a plugin package.
///
/// Serialized as an internally-tagged enum: `{ "type": "widget", ... }`.
/// `rename_all = "camelCase"` so variant/field names use camelCase on the wire
/// (e.g. `flowNode`, `macroSpec`, matching the TS frontend conventions).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Contribution {
    /// Flow node contribution. `node` is the node spec; kept as a JSON value
    /// here to avoid coupling to the flow executor's evolving types.
    FlowNode { node: serde_json::Value },

    /// Overlay widget contribution — a sandboxed iframe with a capability model.
    /// `content` is the widget's HTML/CSS/JS; `capabilities` is what it may do
    /// (and what the user grants at install).
    Widget {
        /// Stable identity inside a multi-widget package.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        icon: Option<String>,
        /// Host schema renderer. Accepted only for an embedded builtin package;
        /// folder-discovered packages continue to provide sandboxed `content`.
        #[serde(default, skip_serializing_if = "Option::is_none", alias = "schema_id")]
        schema_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<u32>,
        /// Optional JSON Schema for author-controlled settings, separate from live data.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        config_schema: Option<serde_json::Value>,
        /// Inbound overlay commands, separate from requests this widget may send
        /// to the host. Absent preserves legacy delivery; an empty map accepts none.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        commands: Option<BTreeMap<String, serde_json::Value>>,
        /// Optional deck-surface contract. When absent, this contribution is
        /// an overlay-only widget and is never exposed in the deck palette.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        deck_surface: Option<DeckSurfaceContribution>,
        capabilities: WidgetCapabilities,
        #[serde(default)]
        content: String,
    },

    /// UI panel contribution — a settings/interaction panel (HTML path).
    UiPanel { panel: String },

    /// Macro contribution. `macro_spec` is the macro definition; kept as a JSON
    /// value here to avoid coupling to the macro system's types.
    Macro {
        #[serde(alias = "macro_spec")]
        macro_spec: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeckGridSpan {
    pub cols: u32,
    pub rows: u32,
}

/// Declarative metadata required to safely expose a package widget on a deck.
/// Executable actions remain host bindings; package content can only emit a
/// declared port intent and never receives raw credentials or arbitrary state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckSurfaceContribution {
    pub definition_id: String,
    /// Builtins use a reviewed host renderer id. Community packages use
    /// `sandboxed-content`, which renders the contribution content in an iframe.
    pub renderer_id: String,
    pub interaction_type: String,
    pub default_span: DeckGridSpan,
    pub min_span: DeckGridSpan,
    pub max_span: DeckGridSpan,
    #[serde(default = "empty_object")]
    pub config_schema: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub inputs: Vec<serde_json::Value>,
    #[serde(default)]
    pub state_subscriptions: Vec<String>,
    #[serde(default = "default_deck_mobile_support")]
    pub mobile_support: String,
    /// Safe, non-executable view tree rendered by the trusted deck host on
    /// desktop/mobile. Arbitrary package HTML remains desktop-only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portable_view: Option<serde_json::Value>,
    /// Explicit allowlists for values projected to companion clients.
    #[serde(default)]
    pub public_config_paths: Vec<String>,
    #[serde(default)]
    pub public_state_paths: Vec<String>,
}

fn empty_object() -> serde_json::Value {
    serde_json::json!({})
}

fn default_deck_mobile_support() -> String {
    "fallback_only".to_string()
}

pub fn validate_deck_surface(
    package_id: &str,
    surface: &DeckSurfaceContribution,
) -> Result<(), String> {
    if !surface.definition_id.starts_with(&format!("{package_id}."))
        || surface.definition_id.starts_with("vortideck.")
    {
        return Err(format!(
            "Deck definition '{}' must be namespaced by package '{}'",
            surface.definition_id, package_id
        ));
    }
    if !matches!(
        surface.renderer_id.as_str(),
        "sandboxed-content" | "portable-schema"
    ) {
        return Err(format!(
            "Untrusted deck renderer '{}' is forbidden",
            surface.renderer_id
        ));
    }
    for span in [&surface.default_span, &surface.min_span, &surface.max_span] {
        if span.cols == 0 || span.rows == 0 || span.cols > 8 || span.rows > 8 {
            return Err("Deck widget spans must be between 1 and 8".into());
        }
    }
    if surface.min_span.cols > surface.default_span.cols
        || surface.min_span.rows > surface.default_span.rows
        || surface.default_span.cols > surface.max_span.cols
        || surface.default_span.rows > surface.max_span.rows
    {
        return Err("Deck widget span constraints are inconsistent".into());
    }
    for path in surface
        .public_config_paths
        .iter()
        .chain(surface.public_state_paths.iter())
    {
        if path.is_empty()
            || path.len() > 128
            || !path.split('.').all(|segment| {
                !segment.is_empty()
                    && segment.chars().all(|character| {
                        character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
                    })
            })
        {
            return Err(format!("Unsafe public projection path '{path}'"));
        }
    }
    if surface.renderer_id == "portable-schema" && surface.portable_view.is_none() {
        return Err("portable-schema renderer requires portableView".into());
    }
    if let Some(view) = &surface.portable_view {
        validate_portable_view(view, &surface.inputs)?;
    }
    Ok(())
}

fn validate_portable_view(
    view: &serde_json::Value,
    inputs: &[serde_json::Value],
) -> Result<(), String> {
    let encoded = serde_json::to_vec(view).map_err(|error| error.to_string())?;
    if encoded.len() > 64 * 1024 {
        return Err("portableView exceeds 64 KiB".into());
    }
    let object = view.as_object().ok_or("portableView must be an object")?;
    if object
        .get("schemaVersion")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        return Err("portableView schemaVersion must be 1".into());
    }
    if object
        .keys()
        .any(|key| !matches!(key.as_str(), "schemaVersion" | "root" | "assets"))
    {
        return Err("portableView contains unsupported fields".into());
    }
    let ports = inputs
        .iter()
        .filter_map(|input| {
            input
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .collect::<std::collections::HashSet<_>>();
    let mut asset_ids = std::collections::HashSet::new();
    if let Some(assets) = object.get("assets") {
        let assets = assets
            .as_array()
            .ok_or("portableView.assets must be an array")?;
        if assets.len() > 16 {
            return Err("portableView has too many assets".into());
        }
        for asset in assets {
            let asset = asset
                .as_object()
                .ok_or("portableView asset must be an object")?;
            let hash = asset
                .get("sha256")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let byte_length = asset
                .get("byteLength")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let id = asset
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let media_type = asset
                .get("mediaType")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let purpose = asset
                .get("purpose")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if asset.keys().any(|key| {
                !matches!(
                    key.as_str(),
                    "id" | "sha256" | "mediaType" | "byteLength" | "purpose"
                )
            }) || id.is_empty()
                || id.len() > 256
                || !asset_ids.insert(id.to_string())
                || hash.len() != 64
                || !hash.chars().all(|character| {
                    character.is_ascii_hexdigit() && !character.is_ascii_uppercase()
                })
                || byte_length == 0
                || byte_length > 2 * 1024 * 1024
                || !matches!(media_type, "image/png" | "image/jpeg" | "image/webp")
                || !matches!(purpose, "icon" | "artwork" | "background")
            {
                return Err("portableView asset declaration is invalid".into());
            }
        }
    }
    let mut count = 0usize;
    let mut referenced_assets = std::collections::HashSet::new();
    validate_portable_node(
        object.get("root").ok_or("portableView.root is required")?,
        1,
        &mut count,
        &ports,
        &mut referenced_assets,
    )?;
    if let Some(undeclared) = referenced_assets.difference(&asset_ids).next() {
        return Err(format!(
            "portableView references undeclared asset '{undeclared}'"
        ));
    }
    Ok(())
}

fn validate_portable_node(
    node: &serde_json::Value,
    depth: usize,
    count: &mut usize,
    ports: &std::collections::HashSet<String>,
    referenced_assets: &mut std::collections::HashSet<String>,
) -> Result<(), String> {
    *count += 1;
    if depth > 8 || *count > 64 {
        return Err("portableView tree exceeds depth or node limit".into());
    }
    let object = node
        .as_object()
        .ok_or("portableView node must be an object")?;
    const TYPES: &[&str] = &[
        "stack",
        "row",
        "column",
        "grid",
        "panel",
        "text",
        "icon",
        "image",
        "badge",
        "progress",
        "meter",
        "press-zone",
        "toggle",
        "slider",
        "rotary",
        "spacer",
        "divider",
    ];
    let node_type = object
        .get("type")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if !TYPES.contains(&node_type) {
        return Err(format!("Unsupported portableView node '{node_type}'"));
    }
    const KEYS: &[&str] = &[
        "type",
        "id",
        "children",
        "value",
        "label",
        "source",
        "icon",
        "direction",
        "align",
        "justify",
        "gap",
        "columns",
        "min",
        "max",
        "step",
        "colorToken",
        "backgroundToken",
        "radius",
        "padding",
        "fontSize",
        "fontWeight",
        "maxLines",
        "gestures",
    ];
    if object.keys().any(|key| !KEYS.contains(&key.as_str())) {
        return Err("portableView node contains unsupported fields".into());
    }
    if let Some(asset_id) = object
        .get("source")
        .and_then(serde_json::Value::as_object)
        .and_then(|source| source.get("asset"))
        .and_then(serde_json::Value::as_str)
    {
        referenced_assets.insert(asset_id.to_string());
    }
    if let Some(gestures) = object.get("gestures") {
        let gestures = gestures
            .as_array()
            .ok_or("portableView gestures must be an array")?;
        if gestures.len() > 8 {
            return Err("portableView node has too many gestures".into());
        }
        for gesture in gestures {
            let gesture = gesture
                .as_object()
                .ok_or("portableView gesture must be an object")?;
            let port = gesture
                .get("portId")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if !ports.contains(port) {
                return Err(format!(
                    "portableView gesture references unknown port '{port}'"
                ));
            }
        }
    }
    if let Some(children) = object.get("children") {
        let children = children
            .as_array()
            .ok_or("portableView children must be an array")?;
        if children.len() > 16 {
            return Err("portableView node has too many children".into());
        }
        for child in children {
            validate_portable_node(child, depth + 1, count, ports, referenced_assets)?;
        }
    }
    Ok(())
}

/// Embedded first-party widgets use the same package/contribution contract as
/// third-party widgets, but reference reviewed host schemas instead of shipping
/// duplicate HTML. They are always available, non-removable, and require no
/// network or user-granted authority.
pub fn builtin_overlay_package() -> PackageInfo {
    fn widget(
        id: &str,
        name: &str,
        description: &str,
        icon: &str,
        width: u32,
        height: u32,
        subscribe: &[&str],
    ) -> Contribution {
        Contribution::Widget {
            id: Some(id.to_string()),
            name: Some(name.to_string()),
            description: Some(description.to_string()),
            icon: Some(icon.to_string()),
            schema_id: Some(id.to_string()),
            width: Some(width),
            height: Some(height),
            config_schema: None,
            commands: Some(BTreeMap::new()),
            deck_surface: None,
            capabilities: WidgetCapabilities {
                subscribe: subscribe.iter().map(|value| (*value).to_string()).collect(),
                ..WidgetCapabilities::default()
            },
            content: String::new(),
        }
    }

    PackageInfo {
        id: "builtin.vortideck-widgets".to_string(),
        name: "VortiDeck Essentials".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        author: Some("VortiDeck".to_string()),
        description: Some(
            "First-party chat, goals, counters, activity, and timer widgets".to_string(),
        ),
        icon_path: None,
        trust: "builtin".to_string(),
        contributions: vec![
            widget(
                "chat",
                "Live Chat",
                "Readable, platform-aware live chat with safe message rendering",
                "heroicons:chat-bubble-left-right",
                360,
                300,
                &["chatMessage"],
            ),
            widget(
                "goal",
                "Stream Goal",
                "Follower, subscriber, or cheer goal with live progress",
                "heroicons:flag",
                340,
                104,
                &["follow", "subscription", "bits"],
            ),
            widget(
                "follower-counter-gradient",
                "Follower Counter",
                "Live follower total with a polished compact presentation",
                "heroicons:user-group",
                240,
                104,
                &["follow"],
            ),
            widget(
                "donation-goal-neon",
                "Cheer Goal",
                "Animated bits or donation-style progress tracker",
                "heroicons:currency-dollar",
                340,
                112,
                &["bits"],
            ),
            widget(
                "subscriber-milestone",
                "Subscriber Milestone",
                "Live subscriber milestone and completion progress",
                "heroicons:trophy",
                300,
                112,
                &["subscription"],
            ),
            widget(
                "recent",
                "Activity Feed",
                "Recent follows, subscriptions, raids, and cheers",
                "heroicons:queue-list",
                340,
                260,
                &["follow", "subscription", "raid", "bits"],
            ),
            widget(
                "timer",
                "Stream Timer",
                "Configurable countdown or elapsed-time display",
                "heroicons:clock",
                240,
                104,
                &[],
            ),
            widget(
                "counter",
                "Custom Counter",
                "A customizable numeric counter for challenges and sessions",
                "heroicons:hashtag",
                220,
                104,
                &[],
            ),
        ],
        permissions: Vec::new(),
        has_runtime: false,
        runtime_active: false,
        is_granted: true,
        needs_reconsent: false,
    }
}

/// Capabilities a widget declares (and the user grants at install).
///
/// These are the user-facing "capabilities"; under the hood they map to
/// `PluginPermission` variants enforced by `PermissionChecker`. See
/// `WIDGET_CAPABILITY_ARCHITECTURE.md` §4 (capabilities vs permissions — one
/// system, "capability" is the user-facing word).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WidgetCapabilities {
    /// Inbound event types the widget wants to receive, e.g. `"follow"`,
    /// `"chatMessage"`, `"viewerCount"`. The overlay bridge filters delivery by
    /// this list (least privilege: a widget never receives events it didn't ask
    /// for).
    #[serde(default)]
    pub subscribe: Vec<String>,

    /// Outbound actions the widget may call, e.g. `"triggerFlow:flow_42"`,
    /// `"openUi"`. Checked by `PermissionChecker` before execution.
    #[serde(default)]
    pub actions: Vec<String>,

    /// Whether the widget may open a UI panel.
    #[serde(default)]
    pub ui: bool,

    /// Network access level — drives the sandbox iframe's CSP `connect-src`.
    #[serde(default)]
    pub network: NetworkCapability,
}

/// Widget network access — selects the sandbox iframe's `connect-src` policy.
///
/// Wire format (matches the TS `NetworkCapability` type so Rust ↔ TS round-trips
/// losslessly): `"none"` | `{ "whitelist": ["https://...", ...] }` | `"full"`.
/// Custom serde (not the default newtype-as-array) so `Whitelist` becomes the
/// object form the frontend's CapabilityDialog + CSP builder expect.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum NetworkCapability {
    /// No network access (default). CSP: `connect-src 'none'`.
    #[default]
    None,

    /// Network restricted to a whitelist of origins. CSP: `connect-src <urls>`.
    Whitelist(Vec<String>),

    /// Unrestricted network access (high-risk; trusted packages only).
    Full,
}

impl Serialize for NetworkCapability {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        match self {
            NetworkCapability::None => serializer.serialize_str("none"),
            NetworkCapability::Full => serializer.serialize_str("full"),
            NetworkCapability::Whitelist(urls) => {
                let mut st = serializer.serialize_struct("NetworkCapability", 1)?;
                st.serialize_field("whitelist", urls)?;
                st.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for NetworkCapability {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Helper {
            Str(String),
            Obj { whitelist: Vec<String> },
        }
        match Helper::deserialize(deserializer)? {
            Helper::Obj { whitelist } => Ok(NetworkCapability::Whitelist(whitelist)),
            Helper::Str(s) => match s.as_str() {
                "none" => Ok(NetworkCapability::None),
                "full" => Ok(NetworkCapability::Full),
                other => Err(serde::de::Error::unknown_variant(other, &["none", "full"])),
            },
        }
    }
}

/// Compile a widget's declared capabilities into the `PluginPermission` set that
/// `PermissionChecker` grants/enforces at install. One-time, non-lossy
/// parse-compile (the manifest struct → the enforcement enum); after install,
/// the enum is the single source of truth for gating + CSP.
///
/// Mapping:
/// - `subscribe: ["follow"]`            → `SubscribeEvent("follow")`
/// - `actions: ["triggerFlow:flow_42"]` → `TriggerAction("triggerFlow:flow_42")`
/// - `ui: true`                         → `OpenUi`   (use the flag, not an action)
/// - `network: Whitelist(urls)`         → `NetworkScoped(urls)`  (URLs preserved for CSP)
/// - `network: Full`                    → `Network`
/// - `network: None`                    → (nothing)
pub fn capabilities_to_permissions(cap: &WidgetCapabilities) -> Vec<PluginPermission> {
    let mut perms = Vec::new();
    for evt in &cap.subscribe {
        perms.push(PluginPermission::SubscribeEvent(evt.clone()));
    }
    for action in &cap.actions {
        perms.push(PluginPermission::TriggerAction(action.clone()));
    }
    if cap.ui {
        perms.push(PluginPermission::OpenUi);
    }
    match &cap.network {
        NetworkCapability::None => {}
        NetworkCapability::Whitelist(urls) => {
            perms.push(PluginPermission::NetworkScoped(urls.clone()))
        }
        NetworkCapability::Full => perms.push(PluginPermission::Network),
    }
    perms
}

/// Compile the complete permission set for the package consent boundary. This
/// is shared by persistence, startup activation, and re-consent checks so those
/// paths cannot drift apart.
pub fn compile_manifest_permissions(
    manifest: &super::bundle::PluginManifest,
) -> Vec<PluginPermission> {
    let mut by_wire_value = BTreeMap::new();
    for permission in manifest.permissions.iter().cloned().chain(
        manifest
            .contributions
            .iter()
            .filter_map(|contribution| match contribution {
                Contribution::Widget { capabilities, .. } => Some(capabilities),
                _ => None,
            })
            .flat_map(capabilities_to_permissions),
    ) {
        let key =
            serde_json::to_string(&permission).unwrap_or_else(|_| format!("{:?}", permission));
        by_wire_value.insert(key, permission);
    }
    by_wire_value.into_values().collect()
}

/// Hash only security-relevant manifest fields. Cosmetic metadata changes do
/// not force consent, while runtime entry, web access, permissions, or any
/// contribution change does.
pub fn manifest_consent_hash(manifest: &super::bundle::PluginManifest) -> Result<String, String> {
    let permissions = compile_manifest_permissions(manifest);
    let descriptor = serde_json::json!({
        "permissions": permissions,
        "contributions": manifest.contributions,
        "runtime": manifest.runtime,
        "entry": manifest.entry,
        "main": manifest.main,
        "webExtensions": manifest.web_extensions,
        "web": manifest.web,
    });
    let bytes = serde_json::to_vec(&descriptor)
        .map_err(|e| format!("Failed to serialize package consent descriptor: {}", e))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::bundle::{PluginManifest, PluginPermission};

    fn manifest() -> PluginManifest {
        serde_json::from_value(serde_json::json!({
            "id": "multi-widget",
            "name": "Multi Widget",
            "version": "1.0.0",
            "author": "VortiDeck",
            "description": "cosmetic metadata",
            "vortideck_version": ">=0.7.0",
            "schema_version": "1.0",
            "permissions": ["storage"],
            "contributions": [
                {
                    "type": "widget",
                    "capabilities": {
                        "subscribe": ["raid", "follow"],
                        "actions": ["triggerFlow:alerts"],
                        "network": { "whitelist": ["https://one.example"] }
                    },
                    "content": "<p>one</p>"
                },
                {
                    "type": "widget",
                    "capabilities": {
                        "subscribe": ["follow", "chatMessage"],
                        "actions": ["openUi", "triggerFlow:alerts"],
                        "ui": true,
                        "network": { "whitelist": ["https://two.example"] }
                    },
                    "content": "<p>two</p>"
                }
            ]
        }))
        .expect("valid test manifest")
    }

    #[test]
    fn package_permission_compile_is_complete_and_deterministic() {
        let manifest = manifest();
        let first = compile_manifest_permissions(&manifest);
        let second = compile_manifest_permissions(&manifest);
        assert_eq!(first, second);
        assert!(first.contains(&PluginPermission::Storage));
        assert!(first.contains(&PluginPermission::SubscribeEvent("raid".into())));
        assert!(first.contains(&PluginPermission::SubscribeEvent("follow".into())));
        assert!(first.contains(&PluginPermission::SubscribeEvent("chatMessage".into())));
        assert!(first.contains(&PluginPermission::TriggerAction("openUi".into())));
        assert!(first.contains(&PluginPermission::OpenUi));
        assert_eq!(
            first
                .iter()
                .filter(
                    |permission| **permission == PluginPermission::SubscribeEvent("follow".into())
                )
                .count(),
            1
        );
    }

    #[test]
    fn consent_hash_ignores_cosmetic_metadata_but_detects_security_changes() {
        let original = manifest();
        let original_hash = manifest_consent_hash(&original).expect("hash");

        let mut cosmetic = original.clone();
        cosmetic.name = "Renamed".into();
        cosmetic.description = "New description".into();
        assert_eq!(
            original_hash,
            manifest_consent_hash(&cosmetic).expect("cosmetic hash")
        );

        let mut changed = original;
        changed.permissions.push(PluginPermission::FileSystem);
        assert_ne!(
            original_hash,
            manifest_consent_hash(&changed).expect("changed hash")
        );
    }

    #[test]
    fn builtin_overlay_widgets_use_stable_package_contributions() {
        let package = builtin_overlay_package();
        assert_eq!(package.id, "builtin.vortideck-widgets");
        assert_eq!(package.trust, "builtin");
        assert!(package.is_granted);
        assert_eq!(package.contributions.len(), 8);

        let widgets: Vec<_> = package
            .contributions
            .iter()
            .map(|contribution| match contribution {
                Contribution::Widget {
                    id,
                    schema_id,
                    width,
                    height,
                    capabilities,
                    content,
                    ..
                } => {
                    assert_eq!(id, schema_id);
                    assert!(width.is_some_and(|value| value > 0));
                    assert!(height.is_some_and(|value| value > 0));
                    assert!(content.is_empty());
                    assert_eq!(capabilities.network, NetworkCapability::None);
                    assert!(capabilities.actions.is_empty());
                    id.as_deref().expect("builtin contribution id")
                }
                _ => panic!("builtin overlay package contains a non-widget contribution"),
            })
            .collect();

        assert_eq!(
            widgets,
            [
                "chat",
                "goal",
                "follower-counter-gradient",
                "donation-goal-neon",
                "subscriber-milestone",
                "recent",
                "timer",
                "counter",
            ]
        );
    }

    #[test]
    fn builtin_overlay_package_serializes_widget_product_metadata() {
        let value = serde_json::to_value(builtin_overlay_package()).expect("serialize package");
        let first = &value["contributions"][0];
        assert_eq!(first["type"], "widget");
        assert_eq!(first["id"], "chat");
        assert_eq!(first["schemaId"], "chat");
        assert_eq!(first["width"], 360);
        assert_eq!(first["capabilities"]["subscribe"][0], "chatMessage");
    }

    #[test]
    fn portable_deck_view_is_validated_at_manifest_boundary() {
        let surface: DeckSurfaceContribution = serde_json::from_value(serde_json::json!({
            "definitionId": "community.player.control",
            "rendererId": "portable-schema",
            "interactionType": "atomic_button",
            "defaultSpan": { "cols": 1, "rows": 1 },
            "minSpan": { "cols": 1, "rows": 1 },
            "maxSpan": { "cols": 2, "rows": 2 },
            "inputs": [{ "id": "play", "kind": "trigger", "supportedPolicies": ["run_once"] }],
            "publicConfigPaths": ["label"],
            "publicStatePaths": ["playing"],
            "portableView": {
                "schemaVersion": 1,
                "root": {
                    "type": "press-zone",
                    "gestures": [{ "gesture": "press", "portId": "play", "delivery": "commit" }],
                    "children": [{ "type": "text", "value": { "config": "label" } }]
                }
            }
        }))
        .expect("surface");
        assert!(validate_deck_surface("community.player", &surface).is_ok());

        let mut with_asset = surface.clone();
        with_asset.portable_view.as_mut().unwrap()["root"] =
            serde_json::json!({ "type": "image", "source": { "asset": "cover.png" } });
        with_asset.portable_view.as_mut().unwrap()["assets"] = serde_json::json!([{
            "id": "cover.png",
            "sha256": "a".repeat(64),
            "mediaType": "image/png",
            "byteLength": 8,
            "purpose": "artwork"
        }]);
        assert!(validate_deck_surface("community.player", &with_asset).is_ok());
        with_asset.portable_view.as_mut().unwrap()["assets"] = serde_json::json!([]);
        assert!(validate_deck_surface("community.player", &with_asset).is_err());

        let mut hostile = surface;
        hostile.portable_view.as_mut().unwrap()["root"]["html"] =
            serde_json::json!("<script>alert(1)</script>");
        assert!(validate_deck_surface("community.player", &hostile).is_err());
    }
}
