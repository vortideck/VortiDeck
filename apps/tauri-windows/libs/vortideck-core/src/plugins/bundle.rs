//! Plugin Bundle Support (.vdext)
//!
//! This module handles ZIP-based plugin bundles containing:
//! - plugin.wasm (WASM logic)
//! - manifest.json (metadata + permissions)
//! - web/ directory (optional: JS/CSS/assets)
//! - signature.json (optional: digital signature)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

use super::PluginError;

/// Current VortiDeck Plugin API schema version (semantic versioning)
///
/// This tracks the plugin API/SDK interface version, not the host app version.
/// Breaking changes increment major version (e.g., 1.0 -> 2.0)
///
/// Version history:
/// - 1.0: Initial release with permissions system
pub const CURRENT_SCHEMA_VERSION: &str = "1.0";
pub const MAX_BUNDLE_BYTES: u64 = 50 * 1024 * 1024;

/// Stable public package identity. Older runtime registrations used a
/// `plugin.` prefix; storage, marketplace and UI boundaries use the canonical
/// package id without that implementation detail.
pub fn canonical_package_id(package_id: &str) -> &str {
    package_id.strip_prefix("plugin.").unwrap_or(package_id)
}

pub fn validate_package_id(package_id: &str) -> Result<(), String> {
    if package_id.len() < 2
        || package_id.len() > 128
        || !package_id.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
        || !package_id
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric())
    {
        return Err(format!("Unsafe package id '{package_id}'"));
    }
    Ok(())
}

pub fn validate_manifest_identity_and_host(manifest: &PluginManifest) -> Result<(), String> {
    validate_package_id(&manifest.id)?;
    if manifest.name.trim().is_empty() {
        return Err("Plugin name is empty".to_string());
    }
    semver::Version::parse(&manifest.version).map_err(|error| {
        format!(
            "Plugin version '{}' is not valid semantic versioning: {error}",
            manifest.version
        )
    })?;
    let host_version = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("VortiDeck host version is invalid: {error}"))?;
    let host_requirement =
        semver::VersionReq::parse(&manifest.vortideck_version).map_err(|error| {
            format!(
                "VortiDeck version requirement '{}' is invalid: {error}",
                manifest.vortideck_version
            )
        })?;
    if !host_requirement.matches(&host_version) {
        return Err(format!(
            "Plugin requires VortiDeck {}, but this host is {}",
            manifest.vortideck_version, host_version
        ));
    }
    // A service entry launches a native process. It must request
    // ExecuteCommands explicitly so the consent prompt names that authority.
    if let Some(service) = manifest.get_service_config() {
        if !manifest
            .permissions
            .contains(&PluginPermission::ExecuteCommands)
        {
            return Err(format!(
                "Package '{}' declares a service entry but does not request the 'executecommands' permission",
                manifest.id
            ));
        }
        if let Some(cwd) = service
            .cwd
            .as_deref()
            .filter(|cwd| !cwd.is_empty() && *cwd != ".")
        {
            validate_relative_artifact_path(cwd)?;
        }
    }
    for contribution in &manifest.contributions {
        match contribution {
            super::contributions::Contribution::Widget {
                deck_surface,
                config_schema,
                commands,
                ..
            } => {
                if let Some(surface) = deck_surface {
                    super::contributions::validate_deck_surface(&manifest.id, surface)?;
                }
                if let Some(schema) = config_schema {
                    super::widget_settings::compile_config_schema(schema)?;
                }
                if let Some(commands) = commands {
                    super::widget_commands::validate_command_catalog(commands)?;
                }
            }
            super::contributions::Contribution::FlowNode { .. } => {
                return Err(format!(
                    "Package '{}' declares a typed flowNode contribution, but schema 1.x executes flow nodes only through WASM metadata or service registerNodes",
                    manifest.id
                ));
            }
            super::contributions::Contribution::UiPanel { .. } => {
                return Err(format!(
                    "Package '{}' declares uiPanel, which this host does not support",
                    manifest.id
                ));
            }
            super::contributions::Contribution::Macro { .. } => {
                return Err(format!(
                    "Package '{}' declares macro, which this host does not support",
                    manifest.id
                ));
            }
        }
    }
    Ok(())
}

/// Validate a package-relative path before it is joined to a writable root.
/// Marketplace packages use canonical forward slashes on every platform.
pub fn validate_relative_artifact_path(path: &str) -> Result<PathBuf, String> {
    use std::path::Component;
    if path.is_empty() || path.contains('\\') || path.contains('\0') {
        return Err(format!("Unsafe package artifact path '{path}'"));
    }
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        return Err(format!(
            "Absolute package artifact path '{path}' is forbidden"
        ));
    }
    for component in candidate.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(format!("Non-canonical package artifact path '{path}'"));
        }
    }
    Ok(candidate.to_path_buf())
}

/// Schema version compatibility result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaCompatibility {
    /// Plugin schema matches current version
    Compatible,
    /// Plugin schema is newer than host (potential incompatibility)
    NewerMinor,
    /// Plugin schema has breaking changes (incompatible)
    Incompatible {
        plugin_version: String,
        host_version: String,
    },
}

/// Check if plugin schema version is compatible with current version
///
/// Compatibility rules (semantic versioning):
/// - Major version must match (1.x compatible with 1.y, not with 2.x)
/// - Minor version can be newer (host 1.0 can load plugin 1.2, with warnings)
/// - Minor version cannot be older than host (host 1.2 cannot load plugin 1.0)
pub fn check_schema_compatibility(plugin_version: &str) -> SchemaCompatibility {
    let parse_version = |v: &str| -> Option<(u32, u32)> {
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() >= 2 {
            let major = parts[0].parse().ok()?;
            let minor = parts[1].parse().ok()?;
            Some((major, minor))
        } else {
            None
        }
    };

    let Some((host_major, host_minor)) = parse_version(CURRENT_SCHEMA_VERSION) else {
        return SchemaCompatibility::Incompatible {
            plugin_version: plugin_version.to_string(),
            host_version: CURRENT_SCHEMA_VERSION.to_string(),
        };
    };

    let Some((plugin_major, plugin_minor)) = parse_version(plugin_version) else {
        return SchemaCompatibility::Incompatible {
            plugin_version: plugin_version.to_string(),
            host_version: CURRENT_SCHEMA_VERSION.to_string(),
        };
    };

    // Major version must match
    if host_major != plugin_major {
        return SchemaCompatibility::Incompatible {
            plugin_version: plugin_version.to_string(),
            host_version: CURRENT_SCHEMA_VERSION.to_string(),
        };
    }

    // Minor version can be newer (with warnings)
    if plugin_minor > host_minor {
        return SchemaCompatibility::NewerMinor;
    }

    SchemaCompatibility::Compatible
}

/// Plugin manifest structure (manifest.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique plugin ID (e.g., "audio-visualizer")
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Semantic version (e.g., "1.0.0")
    pub version: String,

    /// Author name or email
    pub author: String,

    /// Plugin description
    pub description: String,

    /// Minimum VortiDeck version required (e.g., ">=0.1.0")
    pub vortideck_version: String,

    /// Plugin API schema version (e.g., "1.0")
    /// Defaults to "1.0" for backward compatibility with older plugins
    #[serde(default = "default_schema_version")]
    pub schema_version: String,

    /// Requested permissions
    pub permissions: Vec<PluginPermission>,

    /// Whether this plugin has web extensions
    #[serde(default)]
    pub web_extensions: bool,

    /// Main WASM file path (default: "plugin.wasm")
    /// DEPRECATED: Use runtime and entry fields instead
    #[serde(default = "default_main")]
    pub main: String,

    /// Plugin runtime type (wasm or service)
    #[serde(default)]
    pub runtime: PluginRuntime,

    /// Entry points for different runtime types
    #[serde(default)]
    pub entry: HashMap<String, EntryConfig>,

    /// Web resources configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web: Option<WebManifest>,

    /// Typed contributions (widget, flow-node, ui-panel, macro). Generalizes the
    /// legacy `runtime` + `web` fields into a list of typed entries. Optional
    /// and defaults empty, so legacy manifests that only use `runtime`/`web`
    /// continue to load unchanged. See `plugins/contributions.rs` and
    /// `docs/WIDGET_CAPABILITY_ARCHITECTURE.md` §3.5.
    #[serde(default)]
    pub contributions: Vec<super::contributions::Contribution>,
}

impl PluginManifest {
    /// Get the entry configuration for the current runtime
    pub fn get_entry(&self) -> Option<&EntryConfig> {
        match self.runtime {
            PluginRuntime::Wasm => self.entry.get("wasm"),
            PluginRuntime::Service => self.entry.get("service"),
        }
    }

    /// Get the WASM path (from entry or fallback to main field)
    pub fn get_wasm_path(&self) -> String {
        if let Some(EntryConfig::Wasm(path)) = self.entry.get("wasm") {
            path.clone()
        } else {
            self.main.clone()
        }
    }

    /// Get the service configuration
    pub fn get_service_config(&self) -> Option<&ServiceEntry> {
        if let Some(EntryConfig::Service(config)) = self.entry.get("service") {
            Some(config)
        } else {
            None
        }
    }
}

fn default_schema_version() -> String {
    CURRENT_SCHEMA_VERSION.to_string()
}

fn default_main() -> String {
    "plugin.wasm".to_string()
}

/// Plugin runtime type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PluginRuntime {
    /// WebAssembly plugin (in-process, sandboxed)
    Wasm,
    /// Service plugin (external process, any language)
    Service,
}

impl Default for PluginRuntime {
    fn default() -> Self {
        PluginRuntime::Wasm
    }
}

/// Entry point configuration for different plugin types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EntryConfig {
    /// WASM entry (path to .wasm file)
    Wasm(String),
    /// Service entry (process configuration)
    Service(ServiceEntry),
}

/// Service plugin process configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEntry {
    /// Command to execute (e.g., "node", "python3", "./plugin")
    pub command: String,

    /// Command arguments (e.g., ["index.js"])
    #[serde(default)]
    pub args: Vec<String>,

    /// Working directory (relative to plugin folder, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,

    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Web resources configuration in manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebManifest {
    /// Path to main JavaScript extension file (e.g., "web/extension.js")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<String>,

    /// Path to CSS styles file (e.g., "web/styles.css")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub styles: Option<String>,

    /// List of asset files (e.g., ["web/assets/icon.svg"])
    #[serde(default)]
    pub assets: Vec<String>,
}

/// Plugin permissions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PluginPermission {
    /// Can modify DOM
    Dom,

    /// Can inject CSS
    Css,

    /// Can make HTTP requests — full/unrestricted network access (legacy
    /// flow-plugin permission).
    Network,

    /// Widget may access a scoped network whitelist. Non-lossy: the URLs are
    /// preserved so the sandbox iframe's CSP `connect-src` can be built from the
    /// permission itself (single source of truth). Widget-capability variant.
    NetworkScoped(Vec<String>),

    /// Can read/write files
    FileSystem,

    /// Can execute system commands
    ExecuteCommands,

    /// Can access webcam
    AccessCamera,

    /// Can access microphone
    AccessMicrophone,

    /// Can run flows (sandboxed webview API)
    Flows,

    /// Can use plugin-scoped storage (sandboxed webview API)
    Storage,

    /// Widget may subscribe to a stream/event type (e.g. "follow", "chatMessage").
    /// Widget-capability variant — the user-facing "capability" maps to this.
    /// See `contributions::WidgetCapabilities`.
    SubscribeEvent(String),

    /// Widget may call an outbound action (e.g. "triggerFlow:flow_42").
    /// Widget-capability variant.
    TriggerAction(String),

    /// Widget may open a UI panel. Widget-capability variant.
    OpenUi,
}

/// Digital signature for plugin verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSignature {
    /// Author identifier (email or verified ID)
    pub author_id: String,

    /// Base64-encoded public key
    pub author_pubkey: String,

    /// Base64-encoded signature
    pub signature: String,

    /// Timestamp (ISO 8601)
    pub timestamp: String,

    /// SHA-256 hash of package (hex-encoded)
    pub package_hash: String,

    /// VortiDeck version used for signing
    pub vortideck_version: String,
}

/// Extracted web resources from plugin bundle
#[derive(Debug, Clone)]
pub struct WebResources {
    /// JavaScript extension code
    pub extension_js: Option<String>,

    /// CSS styles
    pub styles_css: Option<String>,

    /// Asset files (path -> binary data)
    pub assets: HashMap<String, Vec<u8>>,
}

impl WebResources {
    pub fn new() -> Self {
        Self {
            extension_js: None,
            styles_css: None,
            assets: HashMap::new(),
        }
    }

    pub fn has_content(&self) -> bool {
        self.extension_js.is_some() || self.styles_css.is_some() || !self.assets.is_empty()
    }
}

/// Complete plugin bundle with all extracted components
#[derive(Debug)]
pub struct PluginBundle {
    /// Plugin manifest
    pub manifest: PluginManifest,

    /// WASM binary
    pub wasm_bytes: Vec<u8>,

    /// Web resources (if present)
    pub web_resources: Option<WebResources>,

    /// Digital signature (if present)
    pub signature: Option<PluginSignature>,

    /// Original bundle path
    pub bundle_path: PathBuf,
}

impl PluginBundle {
    /// Extract a plugin bundle from a .vdext file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, PluginError> {
        let path = path.as_ref();
        let total_start = std::time::Instant::now();

        let bundle_size = std::fs::metadata(path)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to inspect bundle: {e}")))?
            .len();
        if bundle_size == 0 || bundle_size > MAX_BUNDLE_BYTES {
            return Err(PluginError::LoadFailed(format!(
                "Bundle size {bundle_size} is outside the supported range"
            )));
        }

        // Open ZIP archive
        let zip_start = std::time::Instant::now();
        let file = File::open(path)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to open bundle: {}", e)))?;

        let mut archive = ZipArchive::new(file)
            .map_err(|e| PluginError::LoadFailed(format!("Invalid ZIP archive: {}", e)))?;
        println!("  ⏱️ ZIP open: {:.2?}", zip_start.elapsed());

        // Canonical bundles store manifest.json at the ZIP root. Releases
        // before 0.7 sometimes wrapped every entry in one top-level folder;
        // accept exactly one such prefix for a safe one-time migration.
        let manifest_entries = archive
            .file_names()
            .filter(|name| *name == "manifest.json" || name.ends_with("/manifest.json"))
            .map(str::to_string)
            .collect::<Vec<_>>();
        let prefix = match manifest_entries.as_slice() {
            [entry] if entry == "manifest.json" => String::new(),
            [entry] => entry
                .strip_suffix("manifest.json")
                .unwrap_or_default()
                .to_string(),
            [] => {
                return Err(PluginError::LoadFailed(
                    "manifest.json not found in bundle".into(),
                ));
            }
            _ => {
                return Err(PluginError::LoadFailed(
                    "Bundle contains multiple package manifests".into(),
                ));
            }
        };

        // 1. Extract manifest.json (required)
        let manifest_start = std::time::Instant::now();
        let manifest = Self::extract_manifest(&mut archive, &prefix)?;
        println!("  ⏱️ Manifest extract: {:.2?}", manifest_start.elapsed());

        // 2. Extract WASM file (required)
        let wasm_start = std::time::Instant::now();
        let wasm_bytes = Self::extract_wasm(&mut archive, &prefix, &manifest.main)?;
        println!(
            "  ⏱️ WASM extract ({} bytes): {:.2?}",
            wasm_bytes.len(),
            wasm_start.elapsed()
        );

        // 3. Extract signature when declared by presence. A malformed
        // signature must fail closed instead of being treated as unsigned.
        let signature_path = format!("{}signature.json", prefix);
        let signature = if archive.file_names().any(|name| name == signature_path) {
            Some(Self::extract_signature(&mut archive, &prefix)?)
        } else {
            None
        };

        // 4. Extract web resources (optional)
        let web_start = std::time::Instant::now();
        let web_resources = if manifest.web_extensions {
            Some(Self::extract_web_resources(
                &mut archive,
                &prefix,
                &manifest,
            )?)
        } else {
            None
        };
        if manifest.web_extensions {
            println!("  ⏱️ Web resources extract: {:.2?}", web_start.elapsed());
        }

        println!(
            "  ⏱️ Total bundle extraction: {:.2?}",
            total_start.elapsed()
        );

        Ok(PluginBundle {
            manifest,
            wasm_bytes,
            web_resources,
            signature,
            bundle_path: path.to_path_buf(),
        })
    }

    /// Deterministic digest of the extracted package content, excluding the
    /// signature itself. Legacy creator signatures sign this digest.
    pub fn content_hash(&self) -> Result<String, String> {
        use sha2::{Digest, Sha256};
        let manifest = serde_json::to_value(&self.manifest)
            .map_err(|error| format!("Failed to canonicalize manifest: {error}"))?;
        let manifest = serde_json::to_vec(&manifest)
            .map_err(|error| format!("Failed to canonicalize manifest: {error}"))?;
        let mut hasher = Sha256::new();
        hasher.update(b"manifest.json\0");
        hasher.update((manifest.len() as u64).to_le_bytes());
        hasher.update(manifest);
        hasher.update(b"plugin.wasm\0");
        hasher.update((self.wasm_bytes.len() as u64).to_le_bytes());
        hasher.update(&self.wasm_bytes);
        if let Some(resources) = &self.web_resources {
            if let Some(js) = &resources.extension_js {
                hasher.update(b"web/extension.js\0");
                hasher.update((js.len() as u64).to_le_bytes());
                hasher.update(js.as_bytes());
            }
            if let Some(css) = &resources.styles_css {
                hasher.update(b"web/styles.css\0");
                hasher.update((css.len() as u64).to_le_bytes());
                hasher.update(css.as_bytes());
            }
            let mut assets = resources.assets.iter().collect::<Vec<_>>();
            assets.sort_by(|left, right| left.0.cmp(right.0));
            for (path, bytes) in assets {
                validate_relative_artifact_path(path)?;
                hasher.update(path.as_bytes());
                hasher.update([0]);
                hasher.update((bytes.len() as u64).to_le_bytes());
                hasher.update(bytes);
            }
        }
        Ok(hex::encode(hasher.finalize()))
    }

    /// Extract manifest.json from archive
    fn extract_manifest<R: Read + Seek>(
        archive: &mut ZipArchive<R>,
        prefix: &str,
    ) -> Result<PluginManifest, PluginError> {
        let mut manifest_file = archive
            .by_name(&format!("{}manifest.json", prefix))
            .map_err(|_| PluginError::LoadFailed("manifest.json not found in bundle".into()))?;

        let mut content = String::new();
        manifest_file
            .read_to_string(&mut content)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to read manifest: {}", e)))?;

        serde_json::from_str(&content)
            .map_err(|e| PluginError::LoadFailed(format!("Invalid manifest.json: {}", e)))
    }

    /// Extract WASM file from archive
    fn extract_wasm<R: Read + Seek>(
        archive: &mut ZipArchive<R>,
        prefix: &str,
        wasm_path: &str,
    ) -> Result<Vec<u8>, PluginError> {
        validate_relative_artifact_path(wasm_path).map_err(PluginError::Validation)?;
        let archive_path = format!("{}{}", prefix, wasm_path);
        let mut wasm_file = archive
            .by_name(&archive_path)
            .map_err(|_| PluginError::LoadFailed(format!("{} not found in bundle", wasm_path)))?;

        let mut bytes = Vec::new();
        wasm_file
            .read_to_end(&mut bytes)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to read WASM: {}", e)))?;

        // Validate WASM magic number
        if bytes.len() < 8 || &bytes[0..4] != b"\0asm" {
            return Err(PluginError::LoadFailed(format!(
                "{} is not a valid WASM file",
                wasm_path
            )));
        }

        Ok(bytes)
    }

    /// Extract signature.json from archive (optional)
    fn extract_signature<R: Read + Seek>(
        archive: &mut ZipArchive<R>,
        prefix: &str,
    ) -> Result<PluginSignature, PluginError> {
        let mut sig_file = archive
            .by_name(&format!("{}signature.json", prefix))
            .map_err(|_| PluginError::LoadFailed("signature.json not found".into()))?;

        let mut content = String::new();
        sig_file
            .read_to_string(&mut content)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to read signature: {}", e)))?;

        serde_json::from_str(&content)
            .map_err(|e| PluginError::LoadFailed(format!("Invalid signature.json: {}", e)))
    }

    /// Extract web resources from archive
    fn extract_web_resources<R: Read + Seek>(
        archive: &mut ZipArchive<R>,
        prefix: &str,
        manifest: &PluginManifest,
    ) -> Result<WebResources, PluginError> {
        let mut resources = WebResources::new();

        let web_manifest = manifest.web.as_ref().ok_or_else(|| {
            PluginError::LoadFailed("web_extensions=true but no web config".into())
        })?;

        // DEBUG: List all files in archive
        println!("📂 Files in archive:");
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                println!("  - {}", file.name());
            }
        }

        // Extract JavaScript extension
        if let Some(js_path) = &web_manifest.extension {
            match Self::extract_text_file(archive, prefix, js_path) {
                Ok(content) => resources.extension_js = Some(content),
                Err(e) => println!("⚠️ Warning: Failed to extract {}: {}", js_path, e),
            }
        }

        // Extract CSS styles
        if let Some(css_path) = &web_manifest.styles {
            match Self::extract_text_file(archive, prefix, css_path) {
                Ok(content) => resources.styles_css = Some(content),
                Err(e) => println!("⚠️ Warning: Failed to extract {}: {}", css_path, e),
            }
        }

        // Extract assets
        for asset_path in &web_manifest.assets {
            match Self::extract_binary_file(archive, prefix, asset_path) {
                Ok(bytes) => {
                    resources.assets.insert(asset_path.clone(), bytes);
                }
                Err(e) => println!("⚠️ Warning: Failed to extract asset {}: {}", asset_path, e),
            }
        }

        Ok(resources)
    }

    /// Extract a text file from archive
    fn extract_text_file<R: Read + Seek>(
        archive: &mut ZipArchive<R>,
        prefix: &str,
        path: &str,
    ) -> Result<String, PluginError> {
        validate_relative_artifact_path(path).map_err(PluginError::Validation)?;
        let path = format!("{}{}", prefix, path);
        // Try both Unix (/) and Windows (\) path separators
        // Manifest uses forward slashes, but ZIP may contain backslashes on Windows
        let normalized_path = path.replace('/', "\\");

        // Check which path exists without holding a borrow
        let found_at_unix_path = archive.by_name(&path).is_ok();

        let content = if found_at_unix_path {
            // Use Unix-style path
            let mut file = archive
                .by_name(&path)
                .map_err(|e| PluginError::LoadFailed(format!("Failed to open {}: {}", path, e)))?;
            let mut s = String::new();
            file.read_to_string(&mut s)
                .map_err(|e| PluginError::LoadFailed(format!("Failed to read {}: {}", path, e)))?;
            s
        } else {
            // Use Windows-style path
            let mut file = archive.by_name(&normalized_path).map_err(|_| {
                PluginError::LoadFailed(format!("{} not found (tried both / and \\)", path))
            })?;
            let mut s = String::new();
            file.read_to_string(&mut s)
                .map_err(|e| PluginError::LoadFailed(format!("Failed to read {}: {}", path, e)))?;
            s
        };

        Ok(content)
    }

    /// Extract a binary file from archive
    fn extract_binary_file<R: Read + Seek>(
        archive: &mut ZipArchive<R>,
        prefix: &str,
        path: &str,
    ) -> Result<Vec<u8>, PluginError> {
        validate_relative_artifact_path(path).map_err(PluginError::Validation)?;
        let path = format!("{}{}", prefix, path);
        // Try both Unix (/) and Windows (\) path separators
        // Manifest uses forward slashes, but ZIP may contain backslashes on Windows
        let normalized_path = path.replace('/', "\\");

        // Check which path exists without holding a borrow
        let found_at_unix_path = archive.by_name(&path).is_ok();

        let bytes = if found_at_unix_path {
            // Use Unix-style path
            let mut file = archive
                .by_name(&path)
                .map_err(|e| PluginError::LoadFailed(format!("Failed to open {}: {}", path, e)))?;
            let mut b = Vec::new();
            file.read_to_end(&mut b)
                .map_err(|e| PluginError::LoadFailed(format!("Failed to read {}: {}", path, e)))?;
            b
        } else {
            // Use Windows-style path
            let mut file = archive.by_name(&normalized_path).map_err(|_| {
                PluginError::LoadFailed(format!("{} not found (tried both / and \\)", path))
            })?;
            let mut b = Vec::new();
            file.read_to_end(&mut b)
                .map_err(|e| PluginError::LoadFailed(format!("Failed to read {}: {}", path, e)))?;
            b
        };

        Ok(bytes)
    }

    /// Validate manifest against VortiDeck version
    pub fn validate_manifest(&self) -> Result<(), PluginError> {
        validate_manifest_identity_and_host(&self.manifest).map_err(PluginError::Validation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_manifest_deserialization() {
        let json = r#"{
            "id": "test-plugin",
            "name": "Test Plugin",
            "version": "1.0.0",
            "author": "Test Author",
            "description": "A test plugin",
            "vortideck_version": ">=0.1.0",
            "permissions": ["dom", "css"],
            "web_extensions": true,
            "main": "plugin.wasm",
            "web": {
                "extension": "web/extension.js",
                "styles": "web/styles.css",
                "assets": ["web/assets/icon.svg"]
            }
        }"#;

        let manifest: PluginManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.id, "test-plugin");
        assert_eq!(manifest.permissions.len(), 2);
        assert!(manifest.web_extensions);
        assert!(manifest.web.is_some());
    }

    fn bundle_requiring(requirement: &str) -> PluginBundle {
        let manifest = serde_json::from_value(serde_json::json!({
            "id": "compatibility-test",
            "name": "Compatibility Test",
            "version": "1.0.0",
            "author": "Test",
            "description": "Test",
            "vortideck_version": requirement,
            "permissions": []
        }))
        .expect("manifest");
        PluginBundle {
            manifest,
            wasm_bytes: Vec::new(),
            web_resources: None,
            signature: None,
            bundle_path: PathBuf::new(),
        }
    }

    #[test]
    fn manifest_accepts_compatible_host_version() {
        assert!(bundle_requiring(">=0.7.0, <0.8.0")
            .validate_manifest()
            .is_ok());
    }

    #[test]
    fn manifest_rejects_incompatible_host_version() {
        let error = bundle_requiring(">=99.0.0")
            .validate_manifest()
            .expect_err("future-only package must be rejected");
        assert!(error.to_string().contains("requires VortiDeck"));
    }

    #[test]
    fn manifest_rejects_contributions_without_runtime_consumers() {
        for contribution in [
            super::super::contributions::Contribution::FlowNode {
                node: serde_json::json!({ "id": "unused" }),
            },
            super::super::contributions::Contribution::UiPanel {
                panel: "panel.html".to_string(),
            },
            super::super::contributions::Contribution::Macro {
                macro_spec: serde_json::json!({ "name": "unused" }),
            },
        ] {
            let mut bundle = bundle_requiring(">=0.7.0, <0.8.0");
            bundle.manifest.contributions = vec![contribution];
            assert!(bundle.validate_manifest().is_err());
        }
    }

    #[test]
    fn service_manifest_must_request_execute_commands_and_safe_cwd() {
        let service_bundle = |permissions: serde_json::Value, cwd: &str| {
            let mut bundle = bundle_requiring(">=0.7.0, <0.8.0");
            bundle.manifest = serde_json::from_value(serde_json::json!({
                "id": "service-test",
                "name": "Service Test",
                "version": "1.0.0",
                "author": "Test",
                "description": "Test",
                "vortideck_version": ">=0.7.0, <0.8.0",
                "runtime": "service",
                "entry": { "service": { "command": "node", "args": ["index.js"], "cwd": cwd } },
                "permissions": permissions
            }))
            .expect("service manifest");
            bundle
        };
        let error = service_bundle(serde_json::json!(["network"]), ".")
            .validate_manifest()
            .expect_err("service without executecommands must be invalid");
        assert!(error.to_string().contains("executecommands"), "{error}");
        assert!(service_bundle(serde_json::json!(["executecommands"]), ".")
            .validate_manifest()
            .is_ok());
        assert!(
            service_bundle(serde_json::json!(["executecommands"]), "../outside")
                .validate_manifest()
                .is_err()
        );
    }

    #[test]
    fn legacy_single_folder_bundle_is_read_without_weakening_root_validation() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("vortideck-wrapped-{nonce}.vdext"));
        let file = File::create(&path).expect("archive file");
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        writer
            .start_file("wrapped/manifest.json", options)
            .expect("manifest entry");
        writer
            .write_all(
                br#"{"id":"wrapped","name":"Wrapped","version":"1.0.0","author":"Test","description":"Legacy","vortideck_version":">=0.7.0","permissions":[],"web_extensions":false,"main":"plugin.wasm"}"#,
            )
            .expect("manifest bytes");
        writer
            .start_file("wrapped/plugin.wasm", options)
            .expect("wasm entry");
        writer.write_all(b"\0asm\x01\0\0\0").expect("wasm bytes");
        writer.finish().expect("finish archive");

        let bundle = PluginBundle::from_file(&path).expect("wrapped bundle");
        assert_eq!(bundle.manifest.id, "wrapped");
        assert_eq!(bundle.wasm_bytes, b"\0asm\x01\0\0\0");
        std::fs::remove_file(path).expect("cleanup archive");
    }
}
