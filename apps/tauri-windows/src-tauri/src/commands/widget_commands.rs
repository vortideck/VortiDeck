//! Package capability commands — grant, revoke, and query durable consent.
//!
//! Consent is compiled from the backend-discovered manifest. The frontend never
//! supplies executable content or permissions, so widget-only, runtime-only, and
//! mixed packages all use the same package-level security boundary.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tauri::{Emitter, Manager, State};
use vortideck_core::core::StateManager;
use vortideck_core::db::database::Database;
use vortideck_core::db::db::DbPool;
use vortideck_core::db::models::{
    DeckItemBinding, DeckItemDocument, DeckWidgetDocument, PackagePermissionGrant,
};
use vortideck_core::plugins::{compile_manifest_permissions, manifest_consent_hash};
use vortideck_core::VortiDeckCore;

fn deck_item_instance(document: &DeckItemDocument) -> serde_json::Value {
    let layouts = document
        .layouts
        .iter()
        .map(|layout| {
            (
                layout.layout_key.clone(),
                serde_json::json!({
                    "x": layout.x,
                    "y": layout.y,
                    "cols": layout.col_span,
                    "rows": layout.row_span,
                }),
            )
        })
        .collect::<serde_json::Map<String, serde_json::Value>>();
    let bindings: Vec<_> = document
        .bindings
        .iter()
        .map(|binding| {
            serde_json::json!({
                "id": binding.id,
                "portId": binding.port_id,
                "action": serde_json::from_str::<serde_json::Value>(&binding.action_json)
                    .unwrap_or(serde_json::Value::Null),
                "policy": serde_json::from_str::<serde_json::Value>(&binding.policy_json)
                    .unwrap_or(serde_json::json!({ "type": "run_once" })),
                "transform": binding.transform_json.as_ref()
                    .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok()),
            })
        })
        .collect();
    serde_json::json!({
        "id": document.item.id,
        "definitionId": document.item.definition_id,
        "packageId": document.item.package_id,
        "contributionId": document.item.contribution_id,
        "configVersion": document.item.config_version,
        "config": serde_json::from_str::<serde_json::Value>(&document.item.config_json)
            .unwrap_or(serde_json::json!({})),
        "appearance": serde_json::from_str::<serde_json::Value>(&document.item.appearance_json)
            .unwrap_or(serde_json::json!({})),
        "layouts": layouts,
        "bindings": bindings,
        "legacyButtonId": document.item.legacy_button_id,
        "enabled": document.item.enabled,
    })
}

fn project_public_config(source: &serde_json::Value, paths: &[String]) -> serde_json::Value {
    let mut target = serde_json::json!({});
    for path in paths {
        let segments = path.split('.').collect::<Vec<_>>();
        let mut current = source;
        let mut found = true;
        for segment in &segments {
            let Some(next) = current.get(*segment) else {
                found = false;
                break;
            };
            current = next;
        }
        if !found {
            continue;
        }
        let mut cursor = &mut target;
        for (index, segment) in segments.iter().enumerate() {
            if index + 1 == segments.len() {
                cursor[*segment] = current.clone();
            } else {
                if !cursor
                    .get(*segment)
                    .is_some_and(serde_json::Value::is_object)
                {
                    cursor[*segment] = serde_json::json!({});
                }
                cursor = &mut cursor[*segment];
            }
        }
    }
    target
}

fn remote_deck_item_instance(
    document: &DeckItemDocument,
    public_config_paths: &HashMap<String, Vec<String>>,
) -> serde_json::Value {
    let layouts = document
        .layouts
        .iter()
        .map(|layout| {
            (
                layout.layout_key.clone(),
                serde_json::json!({
                    "x": layout.x,
                    "y": layout.y,
                    "cols": layout.col_span,
                    "rows": layout.row_span,
                }),
            )
        })
        .collect::<serde_json::Map<String, serde_json::Value>>();
    let available_ports = document
        .bindings
        .iter()
        .map(|binding| binding.port_id.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let parsed_config = serde_json::from_str::<serde_json::Value>(&document.item.config_json)
        .unwrap_or(serde_json::json!({}));
    let config = if document.item.definition_id.starts_with("vortideck.") {
        parsed_config
    } else {
        project_public_config(
            &parsed_config,
            public_config_paths
                .get(&document.item.definition_id)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
        )
    };
    serde_json::json!({
        "id": document.item.id,
        "definitionId": document.item.definition_id,
        "packageId": document.item.package_id,
        "contributionId": document.item.contribution_id,
        "configVersion": document.item.config_version,
        "config": config,
        "appearance": serde_json::from_str::<serde_json::Value>(&document.item.appearance_json)
            .unwrap_or(serde_json::json!({})),
        "layouts": layouts,
        "availablePorts": available_ports,
        "legacyButtonId": document.item.legacy_button_id,
        "enabled": document.item.enabled,
    })
}

pub(crate) fn deck_widget_delta(
    previous: &DeckWidgetDocument,
    current: &DeckWidgetDocument,
) -> serde_json::Value {
    let current_ids: HashSet<_> = current
        .items
        .iter()
        .map(|document| document.item.id.as_str())
        .collect();
    let remove: Vec<_> = previous
        .items
        .iter()
        .filter(|document| !current_ids.contains(document.item.id.as_str()))
        .map(|document| document.item.id.clone())
        .collect();
    serde_json::json!({
        "action": "deck_widget_delta",
        "protocolVersion": 1,
        "deckId": current.deck_id,
        "baseRevision": previous.revision,
        "revision": current.revision,
        "upsert": changed_deck_items(previous, current)
            .into_iter()
            .map(deck_item_instance)
            .collect::<Vec<_>>(),
        "remove": remove,
        "state": [],
    })
}

fn remote_deck_widget_delta(
    previous: &DeckWidgetDocument,
    current: &DeckWidgetDocument,
    public_config_paths: &HashMap<String, Vec<String>>,
    definitions: Vec<serde_json::Value>,
) -> serde_json::Value {
    let current_ids: HashSet<_> = current
        .items
        .iter()
        .map(|document| document.item.id.as_str())
        .collect();
    let remove: Vec<_> = previous
        .items
        .iter()
        .filter(|document| !current_ids.contains(document.item.id.as_str()))
        .map(|document| document.item.id.clone())
        .collect();
    serde_json::json!({
        "action": "deck_widget_delta",
        "protocolVersion": 1,
        "deckId": current.deck_id,
        "baseRevision": previous.revision,
        "revision": current.revision,
        "upsert": changed_deck_items(previous, current).into_iter()
            .map(|document| remote_deck_item_instance(document, public_config_paths))
            .collect::<Vec<_>>(),
        "remove": remove,
        "state": [],
        "definitions": definitions,
    })
}

pub(crate) fn changed_deck_items<'a>(
    previous: &DeckWidgetDocument,
    current: &'a DeckWidgetDocument,
) -> Vec<&'a DeckItemDocument> {
    let previous_by_id = previous
        .items
        .iter()
        .map(|item| (item.item.id.as_str(), serde_json::to_value(item).ok()))
        .collect::<HashMap<_, _>>();
    current
        .items
        .iter()
        .filter(|item| {
            previous_by_id.get(item.item.id.as_str()).cloned().flatten()
                != serde_json::to_value(item).ok()
        })
        .collect()
}

/// Reconciles legacy button mutations into the first-class deck widget document.
///
/// When a button is updated or reset via legacy Tauri commands, this ensures
/// that the authoritative `deck_widget_documents` table stays in sync so
/// widget execution and simulator views never run stale actions.
pub async fn sync_button_to_widget_document(
    app_handle: &tauri::AppHandle,
    db: &Database,
    button_id: i32,
) -> Result<(), String> {
    let button = match db.get_button_by_id(button_id).await {
        Ok(Some(b)) => b,
        _ => return Ok(()),
    };
    let deck_id = button.deck_id;
    let previous = match db.get_deck_widget_document(deck_id).await {
        Ok(doc) => doc,
        Err(_) => return Ok(()),
    };
    if previous.revision == 0 && previous.items.is_empty() {
        return Ok(());
    }

    let mut items = previous.items.clone();
    let legacy_item_id = format!("button:{}", button_id);
    let mut found = false;

    for doc_item in &mut items {
        if doc_item.item.legacy_button_id == Some(button_id) || doc_item.item.id == legacy_item_id {
            found = true;
            if button.mode.as_deref() == Some("rotary") {
                doc_item.item.definition_id = "vortideck.rotary".to_string();
            } else {
                doc_item.item.definition_id = "vortideck.action-button".to_string();
            }

            let mut config: serde_json::Value = serde_json::from_str(&doc_item.item.config_json)
                .unwrap_or_else(|_| serde_json::json!({}));
            config["label"] = serde_json::Value::String(button.label.clone());
            config["position"] = serde_json::json!(button.position);
            config["iconRef"] = serde_json::Value::String(button_id.to_string());
            config["imageFit"] = serde_json::Value::String(button.icon_fit.clone());
            if let Ok(config_str) = serde_json::to_string(&config) {
                doc_item.item.config_json = config_str;
            }

            let port_id = if button.mode.as_deref() == Some("rotary") {
                "value"
            } else {
                "primary"
            };

            let policy_json = if button.mode.as_deref() == Some("rotary") {
                serde_json::json!({
                    "type": "continuous",
                    "throttleMs": 50,
                    "commitOnRelease": true
                })
                .to_string()
            } else {
                let exec_config: serde_json::Value = button
                    .execution_config
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_else(|| serde_json::json!({}));
                match button.execution_mode.as_deref().unwrap_or("run_once") {
                    "repeat_while_pressing" => serde_json::json!({
                        "type": "repeat",
                        "holdMs": exec_config.get("hold_threshold_ms").and_then(|v| v.as_u64()).unwrap_or(300),
                        "intervalMs": exec_config.get("repeat_interval_ms").and_then(|v| v.as_u64()).unwrap_or(100)
                    })
                    .to_string(),
                    "switch" => serde_json::json!({ "type": "toggle" }).to_string(),
                    "queue" => serde_json::json!({ "type": "queue" }).to_string(),
                    _ => serde_json::json!({ "type": "run_once" }).to_string(),
                }
            };

            let action_json =
                serde_json::to_string(&button.action).unwrap_or_else(|_| "null".to_string());

            let binding_id = format!("button:{}:primary", button_id);
            if let Some(binding) = doc_item
                .bindings
                .iter_mut()
                .find(|b| b.id == binding_id || b.port_id == port_id)
            {
                binding.port_id = port_id.to_string();
                binding.action_json = action_json;
                binding.policy_json = policy_json;
            } else {
                doc_item.bindings.push(DeckItemBinding {
                    id: binding_id,
                    item_id: doc_item.item.id.clone(),
                    port_id: port_id.to_string(),
                    action_json,
                    policy_json,
                    transform_json: None,
                });
            }
        }
    }

    if !found {
        return Ok(());
    }

    let mut cur_rev = previous.revision;
    let mut retry_prev = previous;
    for _ in 0..3 {
        match db.save_deck_widget_document(deck_id, cur_rev, &items).await {
            Ok(_) => {
                crate::widget_document_cache::invalidate(deck_id).await;
                if let Ok(current) = db.get_deck_widget_document(deck_id).await {
                    let delta = deck_widget_delta(&retry_prev, &current);
                    let _ = app_handle.emit("deck_widget_delta", delta);
                }
                return Ok(());
            }
            Err(_) => {
                if let Ok(fresh) = db.get_deck_widget_document(deck_id).await {
                    cur_rev = fresh.revision;
                    retry_prev = fresh;
                }
            }
        }
    }

    Ok(())
}

fn with_definition_hash(mut definition: serde_json::Value) -> serde_json::Value {
    use sha2::{Digest, Sha256};
    let canonical =
        serde_json_canonicalizer::to_string(&definition).unwrap_or_else(|_| definition.to_string());
    definition["contentHash"] =
        serde_json::Value::String(format!("{:x}", Sha256::digest(canonical.as_bytes())));
    definition
}

fn package_remote_definitions(
    manifest: &vortideck_core::plugins::bundle::PluginManifest,
) -> Vec<serde_json::Value> {
    manifest
        .contributions
        .iter()
        .filter_map(|contribution| match contribution {
            vortideck_core::plugins::Contribution::Widget {
                id,
                name,
                deck_surface: Some(surface),
                ..
            } => Some(with_definition_hash(serde_json::json!({
                "id": surface.definition_id,
                "name": name.as_deref().or(id.as_deref()).unwrap_or("Plugin widget"),
                "category": "plugins",
                "rendererId": surface.renderer_id,
                "interactionType": surface.interaction_type,
                "defaultSpan": surface.default_span,
                "minSpan": surface.min_span,
                "maxSpan": surface.max_span,
                "configSchema": surface.config_schema,
                "stateSchema": surface.state_schema,
                "inputs": surface.inputs,
                "stateSubscriptions": surface.state_subscriptions,
                "mobileSupport": if surface.portable_view.is_some() { "host_schema" } else { surface.mobile_support.as_str() },
                "portableView": surface.portable_view,
            }))),
            _ => None,
        })
        .collect()
}

/// Revoke a widget package: remove its grant record. The widget loses all
/// granted capabilities (enforced wherever grants are read — overlay bridge,
/// PermissionChecker populated from the DB).
#[tauri::command]
pub async fn revoke_widget(
    widget_id: String,
    app: tauri::AppHandle,
    vortideck_core: State<'_, VortiDeckCore>,
    db_pool: State<'_, DbPool>,
) -> Result<(), String> {
    crate::package_lifecycle::invalidate_package_authority(&app, &widget_id).await;
    let removed_definitions = vortideck_core
        .plugin_manager()
        .package_manifest(&widget_id)
        .await
        .map(|manifest| {
            manifest
                .contributions
                .into_iter()
                .filter_map(|contribution| match contribution {
                    vortideck_core::plugins::Contribution::Widget {
                        deck_surface: Some(surface),
                        ..
                    } => Some(surface.definition_id),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let db = Database::new(db_pool.inner());
    db.remove_package_permission_grant(&widget_id).await?;
    vortideck_core
        .plugin_manager()
        .get_permission_checker()
        .revoke_all_permissions(&widget_id);
    let _ = vortideck_core
        .plugin_manager()
        .unload_package_runtime(&widget_id)
        .await;
    let _ = app.emit(
        "widget_package_revoked",
        serde_json::json!({ "packageId": widget_id }),
    );
    if let Some(state_manager) = app.try_state::<Arc<StateManager>>() {
        crate::broadcast_helper::broadcast_to_all_clients(
            &app,
            state_manager.inner(),
            serde_json::json!({
                "action": "deck_widget_definitions",
                "protocolVersion": 1,
                "definitions": [],
                "removeDefinitions": removed_definitions,
            }),
        )
        .await;
    }
    log::info!("🔓 Revoked widget '{}'", widget_id);
    Ok(())
}

/// Get the granted permission set (JSON) for an installed widget, or `None` if
/// not installed.
#[tauri::command]
pub async fn get_widget_permissions(
    widget_id: String,
    db_pool: State<'_, DbPool>,
) -> Result<Option<String>, String> {
    let db = Database::new(db_pool.inner());
    let pkg = db.get_package_permission_grant(&widget_id).await?;
    Ok(pkg.map(|p| p.permissions_json))
}

/// Load the first-class deck widget document. Revision 0 with an empty item
/// list means the deck is still legacy-backed and should be projected through
/// the V1 compatibility adapter.
#[tauri::command]
pub async fn get_deck_widget_document(
    deck_id: i32,
    db_pool: State<'_, DbPool>,
) -> Result<DeckWidgetDocument, String> {
    Database::new(db_pool.inner())
        .get_deck_widget_document(deck_id)
        .await
}

/// Transactionally replace a deck's widget document using optimistic
/// revision control, then return the canonical saved snapshot.
#[tauri::command]
pub async fn save_deck_widget_document(
    deck_id: i32,
    expected_revision: i32,
    items: Vec<DeckItemDocument>,
    app: tauri::AppHandle,
    state_manager: State<'_, Arc<StateManager>>,
    vortideck_core: State<'_, VortiDeckCore>,
    db_pool: State<'_, DbPool>,
) -> Result<DeckWidgetDocument, String> {
    let db = Database::new(db_pool.inner());
    let previous = db.get_deck_widget_document(deck_id).await?;
    db.save_deck_widget_document(deck_id, expected_revision, &items)
        .await?;
    crate::widget_document_cache::invalidate(deck_id).await;
    let current = db.get_deck_widget_document(deck_id).await?;
    let mut reset_runtime: Vec<String> = changed_deck_items(&previous, &current)
        .into_iter()
        .map(|item| item.item.id.clone())
        .collect();
    let current_ids = current
        .items
        .iter()
        .map(|item| item.item.id.as_str())
        .collect::<HashSet<_>>();
    reset_runtime.extend(
        previous
            .items
            .iter()
            .filter(|item| !current_ids.contains(item.item.id.as_str()))
            .map(|item| item.item.id.clone()),
    );
    let _ = app.emit(
        "deck_widget_runtime_reset",
        serde_json::json!({
            "deckId": deck_id,
            "instanceIds": reset_runtime,
        }),
    );
    let delta = deck_widget_delta(&previous, &current);
    let _ = app.emit("deck_widget_delta", delta.clone());
    let mut public_config_paths = HashMap::new();
    let mut remote_definitions = Vec::new();
    let mut remote_definition_ids = HashSet::new();
    for document in &current.items {
        let (Some(package_id), definition_id) = (
            document.item.package_id.as_deref(),
            document.item.definition_id.as_str(),
        ) else {
            continue;
        };
        let Some(manifest) = vortideck_core
            .plugin_manager()
            .package_manifest(package_id)
            .await
        else {
            continue;
        };
        for contribution in manifest.contributions {
            if let vortideck_core::plugins::Contribution::Widget {
                id,
                name,
                deck_surface: Some(surface),
                ..
            } = contribution
            {
                if surface.definition_id == definition_id {
                    public_config_paths.insert(
                        definition_id.to_string(),
                        surface.public_config_paths.clone(),
                    );
                    if remote_definition_ids.insert(definition_id.to_string()) {
                        remote_definitions.push(with_definition_hash(serde_json::json!({
                            "id": surface.definition_id,
                            "name": name.as_deref().or(id.as_deref()).unwrap_or("Plugin widget"),
                            "category": "plugins",
                            "rendererId": surface.renderer_id,
                            "interactionType": surface.interaction_type,
                            "defaultSpan": surface.default_span,
                            "minSpan": surface.min_span,
                            "maxSpan": surface.max_span,
                            "configSchema": surface.config_schema,
                            "stateSchema": surface.state_schema,
                            "inputs": surface.inputs,
                            "stateSubscriptions": surface.state_subscriptions,
                            "mobileSupport": if surface.portable_view.is_some() { "host_schema" } else { surface.mobile_support.as_str() },
                            "portableView": surface.portable_view,
                        })));
                    }
                    break;
                }
            }
        }
    }
    let remote_delta = remote_deck_widget_delta(
        &previous,
        &current,
        &public_config_paths,
        remote_definitions,
    );
    crate::broadcast_helper::broadcast_to_all_clients(&app, state_manager.inner(), remote_delta)
        .await;
    Ok(current)
}

#[tauri::command]
pub async fn revert_deck_widget_document(
    deck_id: i32,
    db_pool: State<'_, DbPool>,
) -> Result<(), String> {
    Database::new(db_pool.inner())
        .revert_deck_widget_document(deck_id)
        .await?;
    crate::widget_document_cache::invalidate(deck_id).await;
    Ok(())
}

/// List available widget packages scanned from the plugins/widgets folder
/// (packages with widget contributions). The editor lists these; "Add" opens the
/// CapabilityDialog → `grant_package_permissions`. Bundle packages (plugin +
/// widget) use the same grant as widget-only and runtime-only packages.
#[tauri::command]
pub async fn list_available_widgets(
    vortideck_core: State<'_, VortiDeckCore>,
) -> Result<Vec<vortideck_core::plugins::PackageInfo>, String> {
    let plugin_manager = vortideck_core.plugin_manager();
    Ok(plugin_manager.list_available_widgets().await)
}

/// List malformed package folders recorded during discovery. These packages
/// remain unloaded but are visible to the user with an actionable parse error.
#[tauri::command]
pub async fn list_package_discovery_errors(
    vortideck_core: State<'_, VortiDeckCore>,
) -> Result<Vec<vortideck_core::plugins::PackageDiscoveryError>, String> {
    Ok(vortideck_core
        .plugin_manager()
        .list_package_discovery_errors()
        .await)
}

/// Unified package list for "My Plugins": runtime plugins + widget packages in
/// one place. `is_granted` reflects durable package grants / loaded
/// runtime. The frontend groups by contribution type (Widgets / Runtime / …) and
/// opens the package-general CapabilityDialog on install. See
/// `PackageInfo` + `docs/WIDGET_CAPABILITY_ARCHITECTURE.md`.
#[tauri::command]
pub async fn list_packages(
    vortideck_core: State<'_, VortiDeckCore>,
    db_pool: State<'_, DbPool>,
) -> Result<Vec<vortideck_core::plugins::PackageInfo>, String> {
    list_packages_impl(vortideck_core.inner(), db_pool.inner()).await
}

/// Shared implementation used by both the compatibility command and the
/// marketplace/local library projection. Keeping consent comparison here
/// prevents the two surfaces from developing different installed state.
pub async fn list_packages_impl(
    vortideck_core: &VortiDeckCore,
    db_pool: &DbPool,
) -> Result<Vec<vortideck_core::plugins::PackageInfo>, String> {
    // All widget grants from the DB → drives is_granted AND needs_reconsent.
    let db = Database::new(db_pool);
    let grants: std::collections::HashMap<String, PackagePermissionGrant> =
        match db.get_all_package_permission_grants().await {
            Ok(rows) => rows
                .into_iter()
                .map(|p| (p.package_id.clone(), p))
                .collect(),
            Err(e) => {
                log::warn!("list_packages: failed to read widget grants: {}", e);
                std::collections::HashMap::new()
            }
        };
    let granted_ids: std::collections::HashSet<String> = grants.keys().cloned().collect();

    let mut pkgs = vortideck_core
        .plugin_manager()
        .list_packages(&granted_ids)
        .await;

    // Durable consent, not loaded runtime state, is authoritative. Compare the
    // same compiled package descriptor used by grant/startup activation.
    for pkg in pkgs.iter_mut() {
        // Embedded first-party packages are part of the application and have
        // no revocable database grant. They remain installed by definition.
        if pkg.trust == "builtin" {
            pkg.is_granted = true;
            pkg.needs_reconsent = false;
            continue;
        }
        let canonical_pkg_id = pkg.id.strip_prefix("plugin.").unwrap_or(&pkg.id);
        let stored_grant = grants.iter().find_map(|(id, grant)| {
            (id.strip_prefix("plugin.").unwrap_or(id) == canonical_pkg_id).then_some(grant)
        });
        pkg.is_granted = stored_grant.is_some();
        let reconsent = if !pkg.is_granted {
            false
        } else if let Some(grant) = stored_grant {
            if let Some(manifest) = vortideck_core
                .plugin_manager()
                .package_manifest(&pkg.id)
                .await
            {
                let current_permissions = compile_manifest_permissions(&manifest);
                let current_permissions_json =
                    serde_json::to_string(&current_permissions).unwrap_or_default();
                let current_hash = manifest_consent_hash(&manifest).unwrap_or_default();
                current_hash != grant.content_hash
                    || current_permissions_json != grant.permissions_json
            } else {
                true
            }
        } else {
            false
        };
        pkg.needs_reconsent = reconsent;
    }

    Ok(pkgs)
}

/// Persist and apply one package-level grant compiled from the discovered
/// manifest. This is the sole consent path for every contribution type.
#[tauri::command]
pub async fn grant_package_permissions(
    package_id: String,
    app: tauri::AppHandle,
    vortideck_core: State<'_, VortiDeckCore>,
    db_pool: State<'_, DbPool>,
) -> Result<usize, String> {
    let granted = crate::package_lifecycle::grant_package(
        &app,
        vortideck_core.inner(),
        db_pool.inner(),
        &package_id,
    )
    .await?;
    if let (Some(manifest), Some(state_manager)) = (
        vortideck_core
            .plugin_manager()
            .package_manifest(&package_id)
            .await,
        app.try_state::<Arc<StateManager>>(),
    ) {
        crate::broadcast_helper::broadcast_to_all_clients(
            &app,
            state_manager.inner(),
            serde_json::json!({
                "action": "deck_widget_definitions",
                "protocolVersion": 1,
                "definitions": package_remote_definitions(&manifest),
                "removeDefinitions": [],
            }),
        )
        .await;
    }
    Ok(granted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vortideck_core::db::models::{DeckItem, DeckItemBinding, DeckItemLayout};

    fn plugin_document() -> DeckItemDocument {
        DeckItemDocument {
            item: DeckItem {
                id: "widget-1".into(),
                deck_id: 7,
                definition_id: "community.player.control".into(),
                package_id: Some("community.player".into()),
                contribution_id: Some("control".into()),
                config_json: r#"{"label":"Player","secret":"do-not-send"}"#.into(),
                config_version: 1,
                appearance_json: "{}".into(),
                legacy_button_id: None,
                enabled: true,
            },
            layouts: vec![DeckItemLayout {
                item_id: "widget-1".into(),
                layout_key: "portrait".into(),
                x: 0,
                y: 0,
                col_span: 1,
                row_span: 1,
            }],
            bindings: vec![DeckItemBinding {
                id: "binding-1".into(),
                item_id: "widget-1".into(),
                port_id: "play".into(),
                action_json: r#"{"type":"secret-action","token":"private"}"#.into(),
                policy_json: r#"{"type":"run_once"}"#.into(),
                transform_json: None,
            }],
        }
    }

    #[test]
    fn remote_projection_redacts_actions_and_private_config() {
        let document = plugin_document();
        let paths = HashMap::from([(
            "community.player.control".to_string(),
            vec!["label".to_string()],
        )]);
        let remote = remote_deck_item_instance(&document, &paths);
        assert!(remote.get("bindings").is_none());
        assert_eq!(remote["availablePorts"], serde_json::json!(["play"]));
        assert_eq!(remote["config"], serde_json::json!({ "label": "Player" }));
        assert!(!remote.to_string().contains("secret-action"));
        assert!(!remote.to_string().contains("do-not-send"));

        let local = deck_item_instance(&document);
        assert_eq!(local["bindings"][0]["action"]["type"], "secret-action");
    }

    #[test]
    fn widget_delta_upserts_only_changed_instances() {
        let previous = DeckWidgetDocument {
            deck_id: 7,
            revision: 1,
            updated_at: "before".into(),
            items: vec![plugin_document()],
        };
        let unchanged = DeckWidgetDocument {
            deck_id: 7,
            revision: 2,
            updated_at: "after".into(),
            items: previous.items.clone(),
        };
        assert_eq!(
            deck_widget_delta(&previous, &unchanged)["upsert"],
            serde_json::json!([])
        );

        let mut changed = unchanged;
        changed.items[0].item.config_version = 2;
        let delta = deck_widget_delta(&previous, &changed);
        assert_eq!(delta["upsert"].as_array().map(Vec::len), Some(1));
        assert_eq!(delta["upsert"][0]["id"], "widget-1");
    }
}
