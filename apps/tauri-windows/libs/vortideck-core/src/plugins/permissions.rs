use super::bundle::canonical_package_id;
use super::bundle::PluginPermission;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Permission checker for plugin API calls
pub struct PermissionChecker {
    /// Granted permissions per plugin (plugin_id -> permissions)
    granted_permissions: Arc<RwLock<HashMap<String, Vec<PluginPermission>>>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectivePluginScope {
    pub subscriptions: Vec<String>,
    pub actions: Vec<String>,
    pub can_run_flows: bool,
    pub can_use_storage: bool,
    pub unrestricted_network: bool,
    pub network_allowlist: Vec<String>,
}

impl PermissionChecker {
    pub fn new() -> Self {
        Self {
            granted_permissions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Grant permissions to a plugin (called during plugin loading)
    pub fn grant_permissions(&self, plugin_id: &str, permissions: Vec<PluginPermission>) {
        let mut granted = self.granted_permissions.write().unwrap();
        granted.insert(
            canonical_package_id(plugin_id).to_string(),
            permissions.clone(),
        );
        println!("🔐 Granted permissions to {}: {:?}", plugin_id, permissions);
    }

    /// Check if plugin has a specific permission (sync version for host functions)
    pub fn has_permission(&self, plugin_id: &str, permission: &PluginPermission) -> bool {
        let granted = self.granted_permissions.read().unwrap();

        if let Some(permissions) = granted.get(canonical_package_id(plugin_id)) {
            permissions.contains(permission)
        } else {
            false
        }
    }

    /// Enforce permission (returns error if not granted) (sync version for host functions)
    pub fn enforce_permission(
        &self,
        plugin_id: &str,
        permission: &PluginPermission,
    ) -> Result<(), String> {
        if self.has_permission(plugin_id, permission) {
            Ok(())
        } else {
            Err(format!(
                "Permission denied: Plugin '{}' does not have '{:?}' permission",
                plugin_id, permission
            ))
        }
    }

    /// Authorize one HTTP(S) destination against the same durable grants used
    /// by WASM, service, and widget adapters. Scoped entries are URL origins;
    /// an optional non-root path narrows access to that path prefix.
    pub fn enforce_network_url(&self, plugin_id: &str, url: &str) -> Result<(), String> {
        let requested = reqwest::Url::parse(url)
            .map_err(|error| format!("Invalid plugin network URL: {error}"))?;
        if !matches!(requested.scheme(), "http" | "https") {
            return Err("Plugin network access supports only HTTP and HTTPS".to_string());
        }
        let permissions = self.get_granted_permissions(plugin_id);
        if permissions.contains(&PluginPermission::Network) {
            return Ok(());
        }
        let allowed = permissions.iter().any(|permission| {
            let PluginPermission::NetworkScoped(entries) = permission else {
                return false;
            };
            entries.iter().any(|entry| {
                let Ok(scope) = reqwest::Url::parse(entry) else {
                    return false;
                };
                if scope.scheme() != requested.scheme()
                    || scope.host_str() != requested.host_str()
                    || scope.port_or_known_default() != requested.port_or_known_default()
                {
                    return false;
                }
                let scope_path = scope.path().trim_end_matches('/');
                scope_path.is_empty()
                    || requested.path() == scope.path()
                    || requested
                        .path()
                        .strip_prefix(scope_path)
                        .is_some_and(|suffix| suffix.starts_with('/'))
            })
        });
        if allowed {
            Ok(())
        } else {
            Err(format!(
                "Permission denied: Plugin '{}' cannot access network URL '{}'",
                plugin_id, url
            ))
        }
    }

    /// Get all granted permissions for a plugin
    pub fn get_granted_permissions(&self, plugin_id: &str) -> Vec<PluginPermission> {
        let granted = self.granted_permissions.read().unwrap();
        granted
            .get(canonical_package_id(plugin_id))
            .cloned()
            .unwrap_or_default()
    }

    /// Whether the package has an authority record at all. An explicitly
    /// granted package may legitimately have an empty capability set, so this
    /// cannot be inferred from `get_granted_permissions().is_empty()`.
    pub fn has_grant(&self, plugin_id: &str) -> bool {
        self.granted_permissions
            .read()
            .unwrap()
            .contains_key(canonical_package_id(plugin_id))
    }

    /// Compile the transport-independent effective scope from the same grants
    /// used by protected host calls. Transport adapters may choose how to
    /// deliver these scopes, but must not reinterpret manifest claims directly.
    pub fn effective_scope(&self, plugin_id: &str) -> EffectivePluginScope {
        let permissions = self.get_granted_permissions(plugin_id);
        let mut scope = EffectivePluginScope::default();
        for permission in permissions {
            match permission {
                PluginPermission::SubscribeEvent(event) => scope.subscriptions.push(event),
                PluginPermission::TriggerAction(action) => scope.actions.push(action),
                PluginPermission::Flows => scope.can_run_flows = true,
                PluginPermission::Storage => scope.can_use_storage = true,
                PluginPermission::Network => scope.unrestricted_network = true,
                PluginPermission::NetworkScoped(urls) => scope.network_allowlist.extend(urls),
                _ => {}
            }
        }
        scope.subscriptions.sort();
        scope.subscriptions.dedup();
        scope.actions.sort();
        scope.actions.dedup();
        scope.network_allowlist.sort();
        scope.network_allowlist.dedup();
        scope
    }

    /// Revoke all permissions from a plugin
    pub fn revoke_all_permissions(&self, plugin_id: &str) {
        let mut granted = self.granted_permissions.write().unwrap();
        granted.remove(canonical_package_id(plugin_id));
        println!("🔓 Revoked all permissions from {}", plugin_id);
    }
}

fn plugin_workspace_root() -> Result<PathBuf, String> {
    Ok(dirs::document_dir()
        .ok_or("Failed to get documents directory")?
        .join("VortiDeck")
        .join("workspace"))
}

fn canonical_workspace() -> Result<(PathBuf, PathBuf), String> {
    let root = plugin_workspace_root()?;
    std::fs::create_dir_all(&root)
        .map_err(|error| format!("Failed to create plugin workspace: {error}"))?;
    let canonical = std::fs::canonicalize(&root)
        .map_err(|error| format!("Failed to resolve plugin workspace: {error}"))?;
    Ok((root, canonical))
}

pub(crate) fn resolve_workspace_read_path(path: &str) -> Result<PathBuf, String> {
    let relative = super::bundle::validate_relative_artifact_path(path)?;
    let (root, canonical_root) = canonical_workspace()?;
    let resolved = std::fs::canonicalize(root.join(relative))
        .map_err(|error| format!("Failed to resolve plugin workspace path: {error}"))?;
    if !resolved.starts_with(&canonical_root) {
        return Err("Security error: path is outside the plugin workspace".to_string());
    }
    Ok(resolved)
}

pub(crate) fn resolve_workspace_write_path(path: &str) -> Result<PathBuf, String> {
    let relative = super::bundle::validate_relative_artifact_path(path)?;
    let (root, canonical_root) = canonical_workspace()?;
    let target = root.join(relative);
    let parent = target
        .parent()
        .ok_or_else(|| "Plugin workspace path has no parent".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("Failed to create plugin workspace directory: {error}"))?;
    let canonical_parent = std::fs::canonicalize(parent)
        .map_err(|error| format!("Failed to resolve plugin workspace directory: {error}"))?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err("Security error: output path is outside the plugin workspace".to_string());
    }
    if target.exists() {
        let resolved = std::fs::canonicalize(&target)
            .map_err(|error| format!("Failed to resolve plugin workspace file: {error}"))?;
        if !resolved.starts_with(&canonical_root) {
            return Err("Security error: output file is outside the plugin workspace".to_string());
        }
    }
    Ok(target)
}

impl Default for PermissionChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grant_and_check_permission() {
        let checker = PermissionChecker::new();
        let plugin_id = "test-plugin";

        // Initially no permissions
        assert!(!checker.has_permission(plugin_id, &PluginPermission::Flows));

        // Grant permissions
        checker.grant_permissions(
            plugin_id,
            vec![PluginPermission::Flows, PluginPermission::Storage],
        );

        // Check granted permissions
        assert!(checker.has_permission(plugin_id, &PluginPermission::Flows));
        assert!(checker.has_permission(plugin_id, &PluginPermission::Storage));
        assert!(!checker.has_permission(plugin_id, &PluginPermission::Network));
    }

    #[test]
    fn test_enforce_permission() {
        let checker = PermissionChecker::new();
        let plugin_id = "test-plugin";

        // Deny without permission
        assert!(checker
            .enforce_permission(plugin_id, &PluginPermission::Flows)
            .is_err());

        // Grant permission
        checker.grant_permissions(plugin_id, vec![PluginPermission::Flows]);

        // Allow with permission
        assert!(checker
            .enforce_permission(plugin_id, &PluginPermission::Flows)
            .is_ok());
    }

    #[test]
    fn test_revoke_permissions() {
        let checker = PermissionChecker::new();
        let plugin_id = "test-plugin";

        // Grant permissions
        checker.grant_permissions(plugin_id, vec![PluginPermission::Flows]);
        assert!(checker.has_permission(plugin_id, &PluginPermission::Flows));

        // Revoke
        checker.revoke_all_permissions(plugin_id);
        assert!(!checker.has_permission(plugin_id, &PluginPermission::Flows));
    }

    #[test]
    fn empty_capability_set_is_still_an_explicit_grant() {
        let checker = PermissionChecker::new();
        checker.grant_permissions("contribution-only", Vec::new());

        assert!(checker.has_grant("plugin.contribution-only"));
        assert_eq!(
            checker.effective_scope("contribution-only"),
            EffectivePluginScope::default()
        );
    }

    #[test]
    fn effective_scope_is_canonical_sorted_and_revocable() {
        let checker = PermissionChecker::new();
        checker.grant_permissions(
            "plugin.example",
            vec![
                PluginPermission::SubscribeEvent("raid".into()),
                PluginPermission::SubscribeEvent("follow".into()),
                PluginPermission::SubscribeEvent("raid".into()),
                PluginPermission::TriggerAction("overlay.show".into()),
                PluginPermission::Flows,
            ],
        );

        assert_eq!(
            checker.effective_scope("example"),
            EffectivePluginScope {
                subscriptions: vec!["follow".into(), "raid".into()],
                actions: vec!["overlay.show".into()],
                can_run_flows: true,
                ..EffectivePluginScope::default()
            }
        );
        checker.revoke_all_permissions("example");
        assert!(!checker.has_grant("plugin.example"));
        assert_eq!(
            checker.effective_scope("plugin.example"),
            EffectivePluginScope::default()
        );
    }

    #[test]
    fn scoped_network_permission_is_origin_and_path_bound() {
        let checker = PermissionChecker::new();
        checker.grant_permissions(
            "network-test",
            vec![PluginPermission::NetworkScoped(vec![
                "https://api.example.test/v1".into(),
            ])],
        );

        assert!(checker
            .enforce_network_url("network-test", "https://api.example.test/v1/items?limit=1")
            .is_ok());
        assert!(checker
            .enforce_network_url("network-test", "https://api.example.test/v10")
            .is_err());
        assert!(checker
            .enforce_network_url("network-test", "https://other.example.test/v1")
            .is_err());
        assert!(checker
            .enforce_network_url("network-test", "http://api.example.test/v1")
            .is_err());
    }

    #[test]
    fn unrestricted_network_permission_still_rejects_non_http_protocols() {
        let checker = PermissionChecker::new();
        checker.grant_permissions("network-test", vec![PluginPermission::Network]);

        assert!(checker
            .enforce_network_url("network-test", "https://example.test/anything")
            .is_ok());
        assert!(checker
            .enforce_network_url("network-test", "file:///private/data")
            .is_err());
    }

    #[test]
    fn workspace_resolvers_reject_parent_traversal_before_io() {
        assert!(resolve_workspace_read_path("../outside.txt").is_err());
        assert!(resolve_workspace_write_path("../outside.txt").is_err());
    }
}
