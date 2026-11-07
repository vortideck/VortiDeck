//! Plugin file system watcher for hot reload
//!
//! This module provides real-time file system monitoring for plugin changes,
//! enabling hot reload functionality when plugins are added, removed, or modified.
//! Uses the `notify` crate for efficient event-driven file watching.

use log::{error, info, warn};
use notify::{event::ModifyKind, Event, EventKind, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginChangeEvent {
    pub event_type: String, // "added", "removed", "modified"
    pub plugin_path: String,
    pub plugin_id: Option<String>,
}

/// Plugin watcher with event emission capability
pub struct PluginWatcher {
    plugins_dir: PathBuf,
    event_sender: Option<mpsc::UnboundedSender<PluginChangeEvent>>,
}

impl PluginWatcher {
    /// Create a new plugin watcher
    pub fn new(plugins_dir: PathBuf) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "🔥 VortiDeckCore Watcher: Plugin watcher created for directory: {:?}",
            plugins_dir
        );

        // Ensure the directory exists
        if !plugins_dir.exists() {
            std::fs::create_dir_all(&plugins_dir)?;
            println!(
                "✅ VortiDeckCore Watcher: Created plugins directory: {:?}",
                plugins_dir
            );
        }

        Ok(PluginWatcher {
            plugins_dir,
            event_sender: None,
        })
    }

    /// Create a plugin watcher with event emission capability
    pub fn with_events(
        plugins_dir: PathBuf,
    ) -> Result<
        (Self, mpsc::UnboundedReceiver<PluginChangeEvent>),
        Box<dyn std::error::Error + Send + Sync>,
    > {
        let (sender, receiver) = mpsc::unbounded_channel();

        // Ensure the directory exists
        if !plugins_dir.exists() {
            std::fs::create_dir_all(&plugins_dir)?;
            println!(
                "✅ VortiDeckCore Watcher: Created plugins directory: {:?}",
                plugins_dir
            );
        }

        let watcher = PluginWatcher {
            plugins_dir,
            event_sender: Some(sender),
        };

        println!(
            "🔥 VortiDeckCore Watcher: Plugin watcher with events created for directory: {:?}",
            watcher.plugins_dir
        );

        Ok((watcher, receiver))
    }

    /// Start watching for file system changes using event-driven notify
    pub async fn start_watching(
        &self,
        vortideck_core: Arc<crate::VortiDeckCore>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let plugins_dir = self.plugins_dir.clone();
        let _core = vortideck_core;
        let event_sender = self.event_sender.clone();

        // Create channel for file system events
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        // Create file watcher
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        })?;

        // Watch the plugins directory recursively
        watcher.watch(&plugins_dir, RecursiveMode::Recursive)?;
        println!(
            "🔥 VortiDeckCore Watcher: Started event-driven file watching for: {:?}",
            plugins_dir
        );

        // Spawn task to handle events
        tokio::spawn(async move {
            // Keep watcher alive by moving it into the task
            let _watcher = watcher;
            let mut known_package_ids = HashMap::<PathBuf, String>::new();
            if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
                for entry in entries.flatten() {
                    let folder = entry.path();
                    if let Some(id) = Self::read_manifest_id(&folder) {
                        known_package_ids.insert(folder, id);
                    }
                }
            }

            while let Some(event) = rx.recv().await {
                // Log ALL events for debugging
                println!(
                    "🔍 Watcher event: {:?} - paths: {:?}",
                    event.kind, event.paths
                );

                // Handle different event types
                match event.kind {
                    EventKind::Create(_) => {
                        for path in event.paths {
                            if Self::is_package_artifact(&plugins_dir, &path) {
                                println!("📥 File created: {:?}", path);
                                Self::emit_change_candidate(
                                    "added",
                                    &path,
                                    &plugins_dir,
                                    &event_sender,
                                    &mut known_package_ids,
                                );
                            }
                        }
                    }
                    // Handle ANY modify event (not just Data)
                    EventKind::Modify(_) => {
                        for path in event.paths {
                            if Self::is_package_artifact(&plugins_dir, &path) {
                                println!("📝 File modified: {:?}", path);
                                Self::emit_change_candidate(
                                    "modified",
                                    &path,
                                    &plugins_dir,
                                    &event_sender,
                                    &mut known_package_ids,
                                );
                            }
                        }
                    }
                    EventKind::Remove(_) => {
                        for path in event.paths {
                            if Self::is_package_artifact(&plugins_dir, &path) {
                                println!("🗑️ File removed: {:?}", path);
                                Self::emit_change_candidate(
                                    "removed",
                                    &path,
                                    &plugins_dir,
                                    &event_sender,
                                    &mut known_package_ids,
                                );
                            }
                        }
                    }
                    _ => {
                        // Log ignored events
                        println!("⏭️ Ignored event type: {:?}", event.kind);
                    }
                }
            }
        });

        Ok(())
    }

    /// Every immutable file inside a package can affect its runtime or widget
    /// content. Restricting this to manifest.json/plugin.wasm missed service
    /// entry files and external web bundles, so those updates were only caught
    /// after a restart.
    fn is_package_artifact(plugins_dir: &Path, path: &Path) -> bool {
        Self::package_folder(plugins_dir, path).is_some()
    }

    fn package_folder(plugins_dir: &Path, path: &Path) -> Option<PathBuf> {
        let relative = path.strip_prefix(plugins_dir).ok()?;
        let first = relative.components().next()?;
        Some(plugins_dir.join(first.as_os_str()))
    }

    /// Emit a filesystem candidate only. Consent-aware discovery, runtime
    /// activation, rollback, and flow refresh belong to the host lifecycle
    /// coordinator, which also has access to the durable grant database.
    fn emit_change_candidate(
        event_type: &str,
        path: &Path,
        plugins_dir: &Path,
        event_sender: &Option<mpsc::UnboundedSender<PluginChangeEvent>>,
        known_package_ids: &mut HashMap<PathBuf, String>,
    ) {
        let plugin_folder = match Self::package_folder(plugins_dir, path) {
            Some(folder) => folder,
            None => {
                println!("⚠️ Plugin file has no parent folder: {:?}", path);
                return;
            }
        };

        let plugin_id = Self::read_manifest_id(&plugin_folder)
            .map(|id| {
                known_package_ids.insert(plugin_folder.clone(), id.clone());
                id
            })
            .or_else(|| known_package_ids.get(&plugin_folder).cloned());
        if let Some(sender) = event_sender {
            let _ = sender.send(PluginChangeEvent {
                event_type: event_type.to_string(),
                plugin_path: plugin_folder.to_string_lossy().to_string(),
                plugin_id,
            });
        }
    }

    fn read_manifest_id(plugin_folder: &Path) -> Option<String> {
        std::fs::read_to_string(plugin_folder.join("manifest.json"))
            .ok()
            .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
            .and_then(|manifest| manifest.get("id")?.as_str().map(str::to_string))
    }

    pub fn get_plugins_dir(&self) -> &Path {
        &self.plugins_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_runtime_and_widget_files_resolve_to_package_root() {
        let plugins = PathBuf::from("plugins");
        assert_eq!(
            PluginWatcher::package_folder(&plugins, &plugins.join("sample/web/assets/widget.js")),
            Some(plugins.join("sample"))
        );
        assert_eq!(
            PluginWatcher::package_folder(&plugins, &plugins.join("sample/service/index.js")),
            Some(plugins.join("sample"))
        );
        assert_eq!(
            PluginWatcher::package_folder(&plugins, &PathBuf::from("outside/plugin.wasm")),
            None
        );
    }
}
