use crate::client_registry;
use crate::overlay::{CustomOverlayRequirement, OverlayElementsRequirement};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tauri_plugin_store::StoreExt;
use tokio::sync::RwLock;
use vortideck_core::core::StateManager;
// Import overlay types from the new modular structure
use crate::overlay::{
    generate_overlay_html, get_server_url, try_get_server_url, OverlayDesign, OverlayElement,
    OverlayState, Position, Size,
};

pub(crate) fn ensure_trusted_overlay_window(label: &str) -> Result<(), String> {
    match label {
        "main" | "streaming" | "window__streaming_overlays" => Ok(()),
        _ => Err("Overlay capability access is not allowed from this window".to_string()),
    }
}

fn validate_canvas_size(canvas: &Size) -> Result<(), String> {
    if !canvas.width.is_finite()
        || !canvas.height.is_finite()
        || canvas.width.fract() != 0.0
        || canvas.height.fract() != 0.0
        || !(320.0..=7680.0).contains(&canvas.width)
        || !(320.0..=4320.0).contains(&canvas.height)
    {
        return Err(
            "Canvas dimensions must be whole pixels between 320×320 and 7680×4320".to_string(),
        );
    }
    Ok(())
}

async fn persist_overlay_runtime(
    app: &AppHandle,
    state: &Arc<RwLock<OverlayState>>,
) -> Result<(), String> {
    state.read().await.persist(app)
}

async fn effective_package_authority(
    core: &vortideck_core::VortiDeckCore,
    package_id: &str,
) -> Result<Option<(vortideck_core::plugins::EffectivePluginScope, String)>, String> {
    let checker = core.plugin_manager().get_permission_checker();
    if !checker.has_grant(package_id) {
        return Ok(None);
    }
    let Some(manifest) = core.plugin_manager().package_manifest(package_id).await else {
        return Ok(None);
    };
    Ok(Some((
        checker.effective_scope(package_id),
        vortideck_core::plugins::manifest_consent_hash(&manifest)?,
    )))
}

/// Invalidate every connected and future overlay frame owned by a package.
/// Saved layout remains intact while the package is suspended, but generated
/// HTML excludes it before clients are notified, so reconnect cannot recover a
/// stale capability-bearing frame.
pub async fn invalidate_overlay_package(
    app: &AppHandle,
    package_id: &str,
) -> Result<Vec<String>, String> {
    let Some(state) = app.try_state::<Arc<RwLock<OverlayState>>>() else {
        return Err(format!(
            "Overlay state is unavailable while invalidating package '{}'",
            vortideck_core::plugins::canonical_package_id(package_id)
        ));
    };
    let suspended = state.write().await.suspend_package(package_id);
    if let Some(clients) = app.try_state::<Arc<client_registry::ClientRegistry>>() {
        clients
            .send_to_overlays(
                &serde_json::json!({
                    "type": "OVERLAY_UPDATED",
                    "reason": "package_authority_changed",
                    "packageId": vortideck_core::plugins::canonical_package_id(package_id),
                    "removedElementIds": &suspended,
                    "timestamp": chrono::Utc::now().timestamp_millis(),
                })
                .to_string(),
            )
            .await;
    }
    Ok(suspended)
}

/// Make preserved package elements renderable again only after lifecycle code
/// has restored a valid grant/runtime state.
pub async fn restore_overlay_package(
    app: &AppHandle,
    package_id: &str,
) -> Result<Vec<String>, String> {
    let Some(state) = app.try_state::<Arc<RwLock<OverlayState>>>() else {
        return Err(format!(
            "Overlay state is unavailable while restoring package '{}'",
            vortideck_core::plugins::canonical_package_id(package_id)
        ));
    };
    let Some(core) = app.try_state::<vortideck_core::VortiDeckCore>() else {
        return Err(format!(
            "Plugin permission state is unavailable while restoring package '{}'",
            vortideck_core::plugins::canonical_package_id(package_id)
        ));
    };
    let Some((scope, content_hash)) = effective_package_authority(core.inner(), package_id).await?
    else {
        // Runtime-only packages and startup phases before grants are loaded
        // have no overlay authority to restore. The package remains suspended.
        return Ok(Vec::new());
    };
    let restored = state
        .write()
        .await
        .resume_package(package_id, scope, content_hash);
    if restored.is_empty() {
        return Ok(restored);
    }
    if let Some(clients) = app.try_state::<Arc<client_registry::ClientRegistry>>() {
        clients
            .send_to_overlays(
                &serde_json::json!({
                    "type": "OVERLAY_UPDATED",
                    "reason": "package_authority_restored",
                    "packageId": vortideck_core::plugins::canonical_package_id(package_id),
                    "restoredElementIds": &restored,
                    "timestamp": chrono::Utc::now().timestamp_millis(),
                })
                .to_string(),
            )
            .await;
    }
    Ok(restored)
}

/// Broadcast CSS variable updates to all overlay WebSocket clients.
/// Used for live theme changes — avoids regenerating srcdoc, preventing unnecessary iframe reloads.
#[tauri::command]
pub async fn broadcast_overlay_vars(
    vars: std::collections::HashMap<String, String>,
    client_registry: State<'_, Arc<client_registry::ClientRegistry>>,
) -> Result<(), String> {
    let message = serde_json::json!({
        "type": "UPDATE_VARS",
        "payload": vars
    });
    client_registry.send_to_overlays(&message.to_string()).await;
    log::debug!("🎨 Broadcasted {} CSS vars to overlay clients", vars.len());
    Ok(())
}

/// Apply theme CSS vars to the overlay end-to-end:
///   1. Store them in OverlayState and regenerate the overlay HTML (bakes the
///      vars into :root so future overlay loads are colored on first paint).
///   2. Broadcast UPDATE_VARS live, which recolors already-open overlays and
///      their iframe widgets through the CSS cascade — no iframe reload.
#[tauri::command]
pub async fn set_overlay_theme_vars(
    vars: std::collections::HashMap<String, String>,
    app: AppHandle,
    state: State<'_, Arc<RwLock<OverlayState>>>,
    client_registry: State<'_, Arc<client_registry::ClientRegistry>>,
) -> Result<(), String> {
    {
        let mut state_guard = state.write().await;
        state_guard.set_theme_vars(vars.clone());
    }
    persist_overlay_runtime(&app, state.inner()).await?;

    let message = serde_json::json!({
        "type": "UPDATE_VARS",
        "payload": vars
    });
    client_registry.send_to_overlays(&message.to_string()).await;
    log::debug!("🎨 Applied theme vars to overlay (stored + broadcast UPDATE_VARS)");
    Ok(())
}

/// 🔄 Batch sync: clear + re-add all elements in one shot, single broadcast at end.
/// Avoids the race where OVERLAY_CLEARED triggers an empty fetch before elements are re-added.
#[tauri::command]
pub async fn batch_sync_overlay(
    elements: Vec<serde_json::Value>,
    canvas: Option<Size>,
    composition_id: Option<String>,
    expected_revision: Option<u64>,
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, Arc<RwLock<OverlayState>>>,
    core: State<'_, vortideck_core::VortiDeckCore>,
    state_manager: State<'_, Arc<StateManager>>,
    client_registry: State<'_, Arc<client_registry::ClientRegistry>>,
) -> Result<String, String> {
    ensure_trusted_overlay_window(window.label())?;
    let authoring = app.try_state::<Arc<vortideck_core::services::overlay_authoring::OverlayAuthoringService>>();
    let publication = match (composition_id, expected_revision) {
        (Some(id), Some(revision)) => Some(authoring.as_ref().ok_or("Composition library unavailable; original file retained")?.publication(&id, revision).await?),
        (None, None) => None,
        _ => return Err("Publication needs a composition ID and expected revision".into()),
    };
    if let Some(publication) = &publication {
        let content = &publication.document().content;
        if content.elements.len() != elements.len() || canvas.as_ref().is_none_or(|canvas|
            Some(canvas.width) != content.settings["canvasWidth"].as_f64() || Some(canvas.height) != content.settings["canvasHeight"].as_f64()) {
            return Err("Publication does not match the saved draft canvas/elements".into());
        }
        for (draft, rendered) in content.elements.iter().zip(&elements) {
            for key in ["id", "x", "y", "width", "height"] {
                if draft.get(key) != rendered.get(key) { return Err("Publication geometry does not match its draft".into()); }
            }
        }
    }
    println!("🔄 Batch sync: {} elements", elements.len());

    let mut package_authorities = std::collections::HashMap::new();
    for package_id in elements
        .iter()
        .filter_map(|element| element.get("packageId").and_then(|value| value.as_str()))
    {
        let canonical = vortideck_core::plugins::canonical_package_id(package_id).to_string();
        if package_authorities.contains_key(&canonical) {
            continue;
        }
        if let Some(authority) = effective_package_authority(core.inner(), &canonical).await? {
            package_authorities.insert(canonical, authority);
        }
    }

    let mut validated_configs = Vec::with_capacity(elements.len());
    for el in &elements {
        let schema = crate::overlay::settings::schema_for(
            core.inner(),
            el.get("packageId").and_then(|v| v.as_str()),
            el.get("contributionId").and_then(|v| v.as_str()),
        )
        .await?;
        validated_configs.push(
            vortideck_core::plugins::widget_settings::validate_widget_config(
                schema.as_ref(),
                el.get("config").unwrap_or(&serde_json::json!({})),
            )?,
        );
    }
    let mut published_content = publication.as_ref().map(|guard| guard.document().content.clone());
    if let Some(content) = &mut published_content {
        for (element, config) in content.elements.iter_mut().zip(&validated_configs) {
            element["config"] = config.clone();
        }
    }
    {
        let mut active = state.write().await;
        let mut state_guard = active.clone();
        let prior_settings: std::collections::HashMap<_, _> = active
            .current_design
            .elements
            .iter()
            .map(|el| {
                (
                    el.id.clone(),
                    (
                        el.config.clone(),
                        el.config_revision,
                        el.package_id.clone(),
                        el.contribution_id.clone(),
                    ),
                )
            })
            .collect();
        let prior_revisions: std::collections::HashMap<String, u64> = state_guard
            .current_design
            .elements
            .iter()
            .map(|element| (element.id.clone(), element.revision))
            .collect();
        // Clear without broadcast
        state_guard.clear_elements();

        // Add all elements without individual broadcasts
        for (el, config) in elements.iter().zip(validated_configs) {
            let id = el
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let x = el.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let y = el.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let width = el.get("width").and_then(|v| v.as_f64()).unwrap_or(200.0);
            let height = el.get("height").and_then(|v| v.as_f64()).unwrap_or(100.0);
            let html = el
                .get("html")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let z_index = el.get("zIndex").and_then(|v| v.as_i64());

            let prior = prior_settings.get(&id);
            let package_id = el
                .get("packageId")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let contribution_id = el
                .get("contributionId")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let config_revision = match prior {
                Some((previous, revision, package, contribution))
                    if previous == &config
                        && package == &package_id
                        && contribution == &contribution_id =>
                {
                    *revision
                }
                Some((_, revision, _, _)) => {
                    if el
                        .get("configRevision")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                        != *revision
                    {
                        return Err(format!(
                            "Widget '{}' settings changed; reload before syncing",
                            id
                        ));
                    }
                    revision.saturating_add(1)
                }
                None => 0,
            };
            let element = OverlayElement {
                contribution_id,
                config,
                config_revision,
                schema_version: el
                    .get("schemaVersion")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1) as u16,
                revision: prior_revisions
                    .get(&id)
                    .copied()
                    .unwrap_or(0)
                    .saturating_add(1),
                id,
                element_type: el
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .unwrap_or("custom")
                    .to_string(),
                package_id: el
                    .get("packageId")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                package_content_hash: el
                    .get("packageId")
                    .and_then(|v| v.as_str())
                    .and_then(|id| {
                        package_authorities.get(vortideck_core::plugins::canonical_package_id(id))
                    })
                    .map(|(_, hash)| hash.clone()),
                schema_id: el
                    .get("schemaId")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                trust: el.get("trust").and_then(|v| v.as_str()).map(str::to_string),
                capabilities: el.get("capabilities").cloned(),
                data: el.get("data").cloned(),
                content: String::new(),
                position: Position { x, y },
                size: Size { width, height },
                style: None,
                visible: el.get("visible").and_then(|v| v.as_bool()).unwrap_or(true),
                render_html: Some(html),
                custom_props: None,
                z_index,
            };
            // Direct push — no regenerate_html per element
            state_guard.current_design.elements.push(element);
        }
        for (package_id, (scope, content_hash)) in package_authorities {
            state_guard.resume_package(&package_id, scope, content_hash);
        }
        // Single regenerate at the end
        if let Some(canvas) = canvas {
            validate_canvas_size(&canvas)?;
            state_guard.current_design.canvas_size = canvas;
        }
        if let Some(content) = &published_content {
            if let Some(vars) = content.settings.get("theme").and_then(|theme| theme.get("cssVars")) {
                state_guard.theme_vars = serde_json::from_value(vars.clone()).map_err(|_| "Invalid composition theme variables")?;
            }
        }
        state_guard.regenerate_html();
        state_guard.update_timestamp();
        state_guard.publication_receipt = match (&publication, &published_content) {
            (Some(guard), Some(content)) => Some(crate::overlay::state::SavedPublicationReceipt {
                receipt: vortideck_core::services::overlay_authoring::PublicationReceipt {
                    id: guard.document().id.clone(), expected_revision: guard.document().revision, content: content.clone(),
                },
                output_hash: state_guard.output_hash()?,
            }),
            _ => None,
        };
        state_guard.persist(&app)?;
        *active = state_guard;
    }

    let publication_receipt = if let Some(publication) = publication {
        publication.acknowledge(published_content).await.map(|_| ())
    } else { Ok(()) };
    // Single broadcast: overlay updated
    let message = serde_json::json!({
        "type": "OVERLAY_UPDATED",
        "timestamp": chrono::Utc::now().timestamp_millis()
    });
    send_overlay_message_new(&state_manager, &client_registry, &message).await?;
    publication_receipt.map_err(|error| format!("Live output was saved, but its draft publication receipt failed: {error}. Inspect live output before retrying."))?;

    Ok(format!("Batch synced {} elements", elements.len()))
}

/// 🎨 Generate overlay HTML from current design
#[tauri::command]
pub async fn generate_overlay_html_command(
    state: State<'_, Arc<RwLock<OverlayState>>>,
) -> Result<String, String> {
    log::debug!("🎨 generate_overlay_html_command called");
    let state_guard = state.read().await;
    let html = state_guard.generated_html.clone();
    log::debug!("🎨 Generated HTML length: {} characters", html.len());
    Ok(html)
}

// Legacy V1 add_overlay_element removed - V3 uses streaming elements system
// V3 element creation is handled by:
// - Frontend: useOverlayV3Sync.ts syncElementAdd()/syncElementsToBackend()
// - Backend: batch_sync_overlay owns the active composition
// - Marketplace: Dynamic templates instead of hardcoded types

/// 🎨 Add element with custom HTML (V3 scalable)
#[tauri::command]
pub async fn add_overlay_element_v3(
    id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    html: String,
    metadata: Option<serde_json::Value>,
    core: State<'_, vortideck_core::VortiDeckCore>,
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, Arc<RwLock<OverlayState>>>,
    state_manager: State<'_, Arc<StateManager>>,
    client_registry: State<'_, Arc<client_registry::ClientRegistry>>,
) -> Result<String, String> {
    use crate::overlay::state::{Position, Size};
    ensure_trusted_overlay_window(window.label())?;
    println!("🎨 Adding V3 element with HTML: {} at ({}, {})", id, x, y);

    let metadata = metadata.unwrap_or(serde_json::json!({}));
    let mut element = crate::overlay::state::OverlayElement {
        contribution_id: metadata
            .get("contributionId")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        config: metadata
            .get("config")
            .cloned()
            .unwrap_or(serde_json::json!({})),
        config_revision: 0,
        schema_version: 1,
        revision: 0,
        id: id.clone(),
        element_type: "custom".to_string(),
        package_id: metadata
            .get("packageId")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        package_content_hash: None,
        schema_id: metadata
            .get("schemaId")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        trust: metadata
            .get("trust")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        capabilities: metadata.get("capabilities").cloned(),
        data: metadata.get("data").cloned(),
        content: String::new(),
        position: Position { x, y },
        size: Size { width, height },
        style: None,
        visible: true,
        render_html: Some(html),
        custom_props: None,
        z_index: None,
    };

    crate::overlay::settings::validate_element(core.inner(), &mut element).await?;
    let authority = match element.package_id.as_deref() {
        Some(id) => effective_package_authority(core.inner(), id).await?,
        None => None,
    };
    element.package_content_hash = authority.as_ref().map(|(_, hash)| hash.clone());
    {
        let mut active = state.write().await;
        if active.get_element(&id).is_some() {
            return Err("Overlay element already exists".into());
        }
        let mut candidate = active.clone();
        candidate.add_element(element.clone());
        if let (Some(id), Some((scope, hash))) = (element.package_id.as_deref(), authority) {
            candidate.resume_package(id, scope, hash);
        }
        candidate.persist(&app)?;
        *active = candidate;
    }

    let message = serde_json::json!({
        "type": "ELEMENT_ADDED",
        "element": element,
        "timestamp": chrono::Utc::now().timestamp_millis()
    });

    send_overlay_message_new(&state_manager, &client_registry, &message).await?;

    Ok(format!("V3 element '{}' added with HTML", id))
}

/// ➖ Remove element from overlay
#[tauri::command]
pub async fn remove_overlay_element(
    element_id: String,
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, Arc<RwLock<OverlayState>>>,
    state_manager: State<'_, Arc<StateManager>>,
    client_registry: State<'_, Arc<client_registry::ClientRegistry>>,
) -> Result<String, String> {
    ensure_trusted_overlay_window(window.label())?;
    let removed = {
        let mut state_guard = state.write().await;
        state_guard.remove_element(&element_id)
    };

    if removed {
        persist_overlay_runtime(&app, state.inner()).await?;
        // Broadcast element removal via WebSocket
        let remove_message = serde_json::json!({
            "type": "ELEMENT_REMOVED",
            "element_id": element_id,
            "timestamp": chrono::Utc::now().timestamp_millis()
        });

        send_overlay_message_new(&state_manager, &client_registry, &remove_message).await?;
        Ok(format!("Element '{}' removed successfully", element_id))
    } else {
        Err(format!("Element '{}' not found", element_id))
    }
}

/// 🔄 Update element in overlay
#[tauri::command]
pub async fn update_overlay_element(
    element_id: String,
    updates: serde_json::Value,
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, Arc<RwLock<OverlayState>>>,
    core: State<'_, vortideck_core::VortiDeckCore>,
    state_manager: State<'_, Arc<StateManager>>,
    client_registry: State<'_, Arc<client_registry::ClientRegistry>>,
) -> Result<String, String> {
    ensure_trusted_overlay_window(window.label())?;
    println!(
        "🔄 update_overlay_element called: id={}, updates={}",
        element_id, updates
    );

    let package_id = if let Some(package_id) = updates.get("package_id").and_then(|v| v.as_str()) {
        Some(package_id.to_string())
    } else {
        state
            .read()
            .await
            .get_element(&element_id)
            .and_then(|element| element.package_id.clone())
    };
    // Position/data-only updates do not prove that the editor regenerated the
    // package iframe. Only a fresh render_html may acquire current provenance.
    let refreshed_authority = if updates.get("render_html").is_some() {
        match package_id.as_deref() {
            Some(id) => effective_package_authority(core.inner(), id).await?,
            None => None,
        }
    } else {
        None
    };

    if updates.get("config").is_some() || updates.get("contribution_id").is_some() {
        return Err(
            "Use configure_overlay_widget for settings, or batch sync to replace a contribution"
                .into(),
        );
    }
    let updated = {
        let mut state_guard = state.write().await;
        if let Some(current) = state_guard.get_element(&element_id) {
            if package_id != current.package_id {
                return Err("Use batch sync to replace a widget package".into());
            }
        }
        let changed = state_guard.update_element(&element_id, |element| {
            // Update element properties based on the updates JSON
            if let Some(content) = updates.get("content").and_then(|v| v.as_str()) {
                element.content = content.to_string();
            }
            if let Some(x) = updates.get("x").and_then(|v| v.as_f64()) {
                element.position.x = x;
            }
            if let Some(y) = updates.get("y").and_then(|v| v.as_f64()) {
                element.position.y = y;
            }
            if let Some(width) = updates.get("width").and_then(|v| v.as_f64()) {
                element.size.width = width;
            }
            if let Some(height) = updates.get("height").and_then(|v| v.as_f64()) {
                element.size.height = height;
            }
            if let Some(visible) = updates.get("visible").and_then(|v| v.as_bool()) {
                element.visible = visible;
            }
            if let Some(html) = updates.get("render_html").and_then(|v| v.as_str()) {
                element.render_html = Some(html.to_string());
            }
            if let Some(z) = updates.get("zIndex").and_then(|v| v.as_i64()) {
                element.z_index = Some(z);
            }
            if let Some(revision) = updates.get("revision").and_then(|v| v.as_u64()) {
                element.revision = revision;
            }
            if let Some(element_type) = updates.get("element_type").and_then(|v| v.as_str()) {
                element.element_type = element_type.to_string();
            }
            if let Some(package_id) = updates.get("package_id").and_then(|v| v.as_str()) {
                element.package_id = Some(package_id.to_string());
            }
            if let Some(schema_id) = updates.get("schema_id").and_then(|v| v.as_str()) {
                element.schema_id = Some(schema_id.to_string());
            }
            if let Some(trust) = updates.get("trust").and_then(|v| v.as_str()) {
                element.trust = Some(trust.to_string());
            }
            if let Some(capabilities) = updates.get("capabilities") {
                element.capabilities = Some(capabilities.clone());
            }
            if let Some(data) = updates.get("data") {
                element.data = Some(data.clone());
            }
            if let Some((_, content_hash)) = refreshed_authority.as_ref() {
                element.package_content_hash = Some(content_hash.clone());
            }
        });
        if let (Some(package_id), Some((scope, content_hash))) =
            (package_id.as_deref(), refreshed_authority)
        {
            state_guard.resume_package(package_id, scope, content_hash);
        }
        changed
    };

    if updated {
        persist_overlay_runtime(&app, state.inner()).await?;
        // Broadcast element update via WebSocket
        let update_message = serde_json::json!({
            "type": "ELEMENT_UPDATE",
            "element_id": element_id,
            "updates": updates,
            "timestamp": chrono::Utc::now().timestamp_millis()
        });

        send_overlay_message_new(&state_manager, &client_registry, &update_message).await?;
        Ok(format!("Element '{}' updated successfully", element_id))
    } else {
        Err(format!("Element '{}' not found", element_id))
    }
}

/// 🧹 Clear all elements from overlay
#[tauri::command]
pub async fn clear_overlay_elements(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, Arc<RwLock<OverlayState>>>,
    state_manager: State<'_, Arc<StateManager>>,
    client_registry: State<'_, Arc<client_registry::ClientRegistry>>,
) -> Result<String, String> {
    ensure_trusted_overlay_window(window.label())?;
    {
        let mut state_guard = state.write().await;
        state_guard.clear_elements();
    }
    persist_overlay_runtime(&app, state.inner()).await?;

    // Broadcast overlay clear via WebSocket
    let clear_message = serde_json::json!({
        "type": "OVERLAY_CLEARED",
        "timestamp": chrono::Utc::now().timestamp_millis()
    });

    send_overlay_message_new(&state_manager, &client_registry, &clear_message).await?;
    Ok("All overlay elements cleared successfully".to_string())
}

/// 🎨 Set overlay design
#[tauri::command]
pub async fn set_overlay_design(
    mut design: OverlayDesign,
    core: State<'_, vortideck_core::VortiDeckCore>,
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, Arc<RwLock<OverlayState>>>,
    state_manager: State<'_, Arc<StateManager>>,
    client_registry: State<'_, Arc<client_registry::ClientRegistry>>,
) -> Result<String, String> {
    ensure_trusted_overlay_window(window.label())?;
    for element in &mut design.elements {
        crate::overlay::settings::validate_element(core.inner(), element).await?;
    }
    {
        let mut active = state.write().await;
        for element in &mut design.elements {
            if let Some(current) = active.get_element(&element.id) {
                let changed = element.config != current.config
                    || element.package_id != current.package_id
                    || element.contribution_id != current.contribution_id;
                if changed && element.config_revision != current.config_revision {
                    return Err(
                        "Widget settings changed; reload before replacing the design".into(),
                    );
                }
                element.config_revision =
                    current.config_revision.saturating_add(u64::from(changed));
            }
        }
        let mut candidate = active.clone();
        candidate.set_design(design.clone());
        candidate.persist(&app)?;
        *active = candidate;
    }

    // Broadcast design update via WebSocket
    let design_message = serde_json::json!({
        "type": "OVERLAY_DESIGN_UPDATED",
        "design": design,
        "timestamp": chrono::Utc::now().timestamp_millis()
    });

    send_overlay_message_new(&state_manager, &client_registry, &design_message).await?;
    Ok("Overlay design updated successfully".to_string())
}

/// 📐 Get saved canvas size from persistent store
#[tauri::command]
pub async fn get_saved_canvas_size(
    state: State<'_, Arc<RwLock<OverlayState>>>,
) -> Result<serde_json::Value, String> {
    let state = state.read().await;
    let canvas = &state.current_design.canvas_size;
    Ok(serde_json::json!({
        "width": canvas.width,
        "height": canvas.height,
        "found": true,
        "last_updated": state.last_updated,
    }))
}

/// 📊 Get overlay status (fast - no HTTP requests)
#[tauri::command]
pub async fn get_overlay_status(
    window: WebviewWindow,
    state: State<'_, Arc<RwLock<OverlayState>>>,
    state_manager: State<'_, Arc<StateManager>>,
) -> Result<serde_json::Value, String> {
    ensure_trusted_overlay_window(window.label())?;
    let state_guard = state.read().await;

    // The overlay is available only after the unified server has issued its
    // process-scoped capability. Do not panic while the host is still starting.
    let server_url = try_get_server_url(&state_manager);
    let server_running = server_url.is_some();

    Ok(serde_json::json!({
        "elements_count": state_guard.current_design.elements.len(),
        "elements": state_guard.current_design.elements,
        "last_updated": state_guard.last_updated,
        "canvas_size": state_guard.current_design.canvas_size,
        "design_name": state_guard.current_design.name,
        "server_url": server_url,
        "server_running": server_running // ✅ Check unified server status
    }))
}

/// 🌐 Start overlay HTML server
#[tauri::command]
pub async fn start_overlay_server_command(
    window: WebviewWindow,
    _state: State<'_, Arc<RwLock<OverlayState>>>,
    state_manager: State<'_, Arc<StateManager>>,
) -> Result<String, String> {
    ensure_trusted_overlay_window(window.label())?;
    log::debug!("start_overlay_server_command called for the unified server");

    // There is no separate overlay server to start. Return only the canonical,
    // authenticated capability and fail while the unified host is unavailable.
    let url = try_get_server_url(&state_manager)
        .ok_or_else(|| "Unified server is not available".to_string())?;

    // Keep the capability inside the trusted main window instead of broadcasting
    // it to every webview (including plugin views).
    let _ = window.emit(
        "overlay-server-started",
        serde_json::json!({
            "status": true,
            "url": url,
            "timestamp": chrono::Utc::now().timestamp_millis()
        }),
    );

    log::debug!("Emitted overlay-server-started to the main window");
    Ok(url)
}

/// 🛑 Stop overlay HTML server
#[tauri::command]
pub async fn stop_overlay_server_command(
    window: WebviewWindow,
    state_manager: State<'_, Arc<StateManager>>,
) -> Result<String, String> {
    ensure_trusted_overlay_window(window.label())?;
    println!("🛑 stop_overlay_server_command called - overlay is part of unified Warp server");

    // Since overlay is now part of unified Warp server, we can't stop it separately
    // Check if server is actually running
    let server_running = state_manager.get_server_info().is_some();
    let message = if server_running {
        "Overlay server is integrated into unified Warp server (cannot stop separately)".to_string()
    } else {
        "Unified server is not running".to_string()
    };

    println!("🛑 {}", message);

    // Emit server stopped event
    let _ = window.emit(
        "overlay-server-stopped",
        serde_json::json!({
            "status": false,
            "message": message,
            "timestamp": chrono::Utc::now().timestamp_millis()
        }),
    );

    println!("📡 Emitted overlay-server-stopped event");
    Ok(message)
}

pub fn build_obs_overlay_capability_message(
    integration_guid: &str,
    server_url: &str,
) -> serde_json::Value {
    serde_json::json!({
        "path": "/api/v1/integration/sdk/action/invoke",
        "verb": "BROADCAST",
        "payload": {
            "actionId": "obs_overlay_set_data",
            "integrationGuid": integration_guid,
            "parameters": { "url": server_url }
        }
    })
}

pub fn send_overlay_capability_to_obs(
    sender: &tokio::sync::broadcast::Sender<tokio_tungstenite::tungstenite::Message>,
    integration_guid: &str,
    server_url: &str,
) -> Result<(), String> {
    let message = build_obs_overlay_capability_message(integration_guid, server_url);
    sender
        .send(tokio_tungstenite::tungstenite::Message::Text(
            message.to_string().into(),
        ))
        .map(|_| ())
        .map_err(|error| format!("Failed to deliver overlay capability to OBS: {error}"))
}

/// 🎬 Send overlay server URL directly to OBS plugin
#[tauri::command]
pub async fn send_overlay_to_obs(
    state: State<'_, Arc<RwLock<OverlayState>>>,
    state_manager: State<'_, Arc<StateManager>>,
) -> Result<serde_json::Value, String> {
    // No premium validation - basic overlay sending is free for all users

    // Get the overlay server URL from unified Warp server
    let server_url = get_server_url(&state_manager);

    // Send overlay URL directly to OBS plugin using the correct format
    if let Some(obs_plugin_sender) = state_manager.get_obs_plugin_sender().await {
        // Get integration GUID for OBS plugin
        let integration_guid = state_manager.get_obs_integration_guid().unwrap_or_default();

        match send_overlay_capability_to_obs(&obs_plugin_sender, &integration_guid, &server_url) {
            Ok(_) => {
                log::info!("Delivered refreshed overlay capability to OBS");
                Ok(serde_json::json!({
                    "success": true,
                    "message": "Overlay URL sent directly to OBS plugin",
                    "server_url": server_url,
                    "action_sent": "obs_overlay_set_data"
                }))
            }
            Err(e) => {
                log::error!("Failed to deliver overlay capability to OBS: {}", e);
                Err(e)
            }
        }
    } else {
        Err("OBS plugin not connected".to_string())
    }
}

/// 🎨 Generic live style patch for one element (the "CSS update" WS path).
///
/// The overlay applies `styles` inline on the matching `.overlay-element` div —
/// **no fetch, no reload**. This is the single instant-update channel for any
/// CSS property the editor changes live:
///   - live drag  → `{ left: "..px", top: "..px" }`
///   - z-order    → `{ zIndex: "N" }`
///   - (future)   → opacity, transform, etc.
///
/// Keys are camelCase CSS-DOM property names (`zIndex`, `left`, `top`); values
/// are strings. Persistence of structural changes (position, z-index, content)
/// into backend state / render_html is handled separately by the editor so
/// reloads and late-loading overlays stay correct.
#[tauri::command]
pub async fn broadcast_element_patch(
    element_id: String,
    styles: std::collections::HashMap<String, String>,
    state_manager: State<'_, Arc<StateManager>>,
    client_registry: State<'_, Arc<client_registry::ClientRegistry>>,
) -> Result<(), String> {
    let message = serde_json::json!({
        "type": "ELEMENT_PATCH",
        "element_id": element_id,
        "styles": styles,
        "timestamp": chrono::Utc::now().timestamp_millis()
    });

    send_overlay_message_new(&state_manager, &client_registry, &message).await?;
    Ok(())
}

/// 📡 Broadcast element update for OBS refresh
#[tauri::command]
pub async fn broadcast_element_update(
    element_id: String,
    updates: serde_json::Value,
    state_manager: State<'_, Arc<StateManager>>,
    client_registry: State<'_, Arc<client_registry::ClientRegistry>>,
) -> Result<(), String> {
    let update_message = serde_json::json!({
        "type": "ELEMENT_UPDATE",
        "element_id": element_id,
        "updates": updates,
        "timestamp": chrono::Utc::now().timestamp_millis()
    });

    send_overlay_message_new(&state_manager, &client_registry, &update_message).await?;
    Ok(())
}

/// 📏 Handle OBS canvas size update from plugin
pub async fn handle_obs_canvas_size_update(
    message: &serde_json::Value,
    app_handle: &AppHandle,
) -> Result<(), String> {
    let width = message
        .get("width")
        .and_then(|value| value.as_u64())
        .ok_or("Invalid OBS canvas width")?;
    let height = message
        .get("height")
        .and_then(|value| value.as_u64())
        .ok_or("Invalid OBS canvas height")?;
    validate_canvas_size(&Size {
        width: width as f64,
        height: height as f64,
    })?;

    // An unsolicited OBS update is observation only. Geometry and the saved
    // active design change exclusively through the explicit correlated Apply
    // transaction, so reconnects and project loads cannot rescale a layout.
    if let Ok(store) = app_handle.store_builder("obs_settings.json").build() {
        store.set(
            "observed_obs_canvas_size",
            serde_json::json!({
                "width": width,
                "height": height,
                "last_updated": chrono::Utc::now().timestamp_millis(),
            }),
        );
        // Diagnostic persistence must not suppress the current live observation.
        if let Err(error) = store.save() {
            log::warn!("Failed to save observed OBS canvas size: {error}");
        }
    }

    app_handle
        .emit(
            "obs-canvas-size-changed",
            serde_json::json!({
                "width": width,
                "height": height,
                "synced_at": chrono::Utc::now().timestamp_millis(),
                "source": "obs_plugin",
                "streaming": message.get("streaming"),
                "recording": message.get("recording"),
                "managed_source_found": message.get("managed_source_found"),
                "managed_source_matches": message.get("managed_source_matches"),
            }),
        )
        .map_err(|error| format!("Failed to emit canvas size event: {error}"))
}

#[tauri::command]
pub async fn apply_obs_canvas_size(
    width: u32,
    height: u32,
    request_id: String,
    window: WebviewWindow,
    state_manager: State<'_, Arc<StateManager>>,
) -> Result<(), String> {
    ensure_trusted_overlay_window(window.label())?;
    validate_canvas_size(&Size {
        width: width as f64,
        height: height as f64,
    })?;
    if request_id.is_empty() || request_id.len() > 128 {
        return Err("Canvas Apply requires a valid request ID".to_string());
    }
    let sender = state_manager
        .get_obs_plugin_sender()
        .await
        .ok_or("OBS companion is not connected")?;
    sender
        .send(tokio_tungstenite::tungstenite::Message::Text(
            serde_json::json!({
                "type": "APPLY_OBS_CANVAS_SIZE",
                "request_id": request_id,
                "width": width,
                "height": height,
            })
            .to_string()
            .into(),
        ))
        .map(|_| ())
        .map_err(|error| format!("Failed to send OBS canvas Apply: {error}"))
}

/// 🔄 Request current canvas size from OBS plugin (direct WebSocket message)
#[tauri::command]
pub async fn request_obs_canvas_size(
    state_manager: State<'_, Arc<StateManager>>,
) -> Result<String, String> {
    println!("🔄 Requesting canvas size from OBS plugin");

    // Send direct WebSocket message to OBS (not wrapped in SDK format)
    let request_message = serde_json::json!({
        "type": "GET_OBS_CANVAS_SIZE",
        "timestamp": chrono::Utc::now().timestamp_millis()
    });

    if let Some(obs_tx) = state_manager.get_obs_plugin_sender().await {
        let message_str = request_message.to_string();
        println!("🔄 Sending canvas size request: {}", message_str);

        match obs_tx.send(tokio_tungstenite::tungstenite::Message::Text(
            message_str.into(),
        )) {
            Ok(_) => {
                println!("✅ Canvas size request sent to OBS plugin");
                Ok("Canvas size request sent to OBS plugin".to_string())
            }
            Err(e) => {
                println!("❌ Failed to send canvas size request: {}", e);
                Err(format!("Failed to send canvas size request: {}", e))
            }
        }
    } else {
        Err("OBS plugin not connected".to_string())
    }
}

// Legacy V1 add_test_overlay_element removed - V3 uses streaming elements

// /// 🧪 Trigger test advertisement using new banner system
// #[cfg(debug_assertions)]
// #[tauri::command]
// pub async fn trigger_test_overlay_ad(
//     state_manager: State<'_, Arc<StateManager>>,
// ) -> Result<String, String> {
//     use crate::commands::obs_banner_commands::{obs_banner_set_data, BannerData};

//     let test_banner = BannerData {
//         content_data: "https://via.placeholder.com/300x150/6366f1/ffffff?text=Test+Ad".to_string(),
//         content_type: "image".to_string(),
//         duration_seconds: 5,
//         premium_status: "free".to_string(),
//         interrupt_current: Some(false),
//         css: None,
//         css_locked: Some(false),
//         queue_mode: Some("false".to_string()),
//     };

//     match obs_banner_set_data(state_manager, test_banner).await {
//         Ok(_) => Ok("Test advertisement triggered successfully".to_string()),
//         Err(e) => Err(format!("Failed to trigger test advertisement: {}", e))
//     }
// }

/// 🎬 Send banner message directly to banner clients via ClientRegistry
#[tauri::command]
pub async fn send_banner_message(
    message: serde_json::Value,
    client_registry: State<'_, Arc<crate::client_registry::ClientRegistry>>,
) -> Result<String, String> {
    println!("🎬 Sending banner message: {}", message);

    let message_str = message.to_string();
    let sent_count = client_registry.send_to_banners(&message_str).await;

    if sent_count > 0 {
        println!("✅ Banner message sent to {} banner client(s)", sent_count);
        Ok(format!("Banner message sent to {} client(s)", sent_count))
    } else {
        let error = "No banner clients connected";
        println!("⚠️ {}", error);
        Ok(error.to_string()) // Return Ok since this isn't necessarily an error
    }
}

/// 🎬 Send overlay message directly to overlay clients via WebSocket
#[tauri::command]
pub async fn send_overlay_message(
    message: String,
    state_manager: State<'_, Arc<StateManager>>,
    client_registry: State<'_, Arc<crate::client_registry::ClientRegistry>>,
) -> Result<String, String> {
    log::debug!("Sending overlay message");

    let message_json: serde_json::Value =
        serde_json::from_str(&message).map_err(|e| format!("Invalid JSON message: {}", e))?;

    let recipients =
        send_overlay_message_new(&state_manager, &client_registry, &message_json).await?;

    Ok(format!(
        "Overlay message queued for {recipients} recipient(s)"
    ))
}

// Helper functions for WebSocket broadcasting

/// Send overlay message to OBS plugin only (for OBS Studio integration)
async fn send_overlay_message_to_obs(
    state_manager: &StateManager,
    message: &serde_json::Value,
) -> Result<bool, String> {
    if let Some(obs_tx) = state_manager.get_obs_plugin_sender().await {
        let obs_message = serde_json::json!({
            "path": "/api/v1/integration/sdk/action/invoke",
            "verb": "BROADCAST",
            "payload": message
        });

        if let Err(e) = obs_tx.send(tokio_tungstenite::tungstenite::Message::Text(
            obs_message.to_string().into(),
        )) {
            println!("⚠️ Failed to send overlay message to OBS plugin: {}", e);
            Err(format!("Failed to send to OBS plugin: {}", e))
        } else {
            log::debug!("Queued overlay message for OBS companion transport");
            Ok(true)
        }
    } else {
        println!("ℹ️ OBS plugin not connected");
        Ok(false) // No recipient; absence is not a transport failure
    }
}

/// Send overlay message to WebSocket overlay clients only
async fn send_overlay_message_to_clients(
    client_registry: &client_registry::ClientRegistry,
    message: &serde_json::Value,
) -> Result<usize, String> {
    let overlay_sent = client_registry.send_to_overlays(&message.to_string()).await;

    if overlay_sent > 0 {
        println!("🎨 Sent message to {} overlay client(s)", overlay_sent);
    } else {
        println!("ℹ️ No overlay clients connected");
    }

    Ok(overlay_sent)
}

/// Send overlay message to both OBS plugin and overlay clients (use for full broadcast)
async fn send_overlay_message_new(
    state_manager: &StateManager,
    client_registry: &client_registry::ClientRegistry,
    message: &serde_json::Value,
) -> Result<usize, String> {
    let mut total_recipients = 0;
    let mut obs_sent = false;
    let mut client_count = 0;

    // Send to OBS plugin if connected (only once)
    if matches!(
        send_overlay_message_to_obs(state_manager, message).await,
        Ok(true)
    ) {
        obs_sent = true;
        total_recipients += 1;
    }

    // Send to overlay browser clients (only once)
    match send_overlay_message_to_clients(client_registry, message).await {
        Ok(count) => {
            client_count = count;
            total_recipients += count;
        }
        Err(e) => println!("⚠️ Failed to send to overlay clients: {}", e),
    }

    // Single consolidated log message
    if obs_sent && client_count > 0 {
        println!(
            "📡 Overlay message queued for {} recipient(s) (1 OBS plugin + {} overlay client(s))",
            total_recipients, client_count
        );
    } else if obs_sent {
        println!("📡 Overlay message queued for 1 recipient (OBS plugin only)");
    } else if client_count > 0 {
        println!(
            "📡 Overlay message queued for {} overlay client(s)",
            client_count
        );
    } else {
        println!("⚠️ No overlay recipients available");
    }

    Ok(total_recipients)
}

#[cfg(test)]
mod overlay_capability_tests {
    use super::{build_obs_overlay_capability_message, ensure_trusted_overlay_window};

    #[test]
    fn capability_commands_reject_untrusted_webviews() {
        assert!(ensure_trusted_overlay_window("main").is_ok());
        assert!(ensure_trusted_overlay_window("streaming").is_ok());
        assert!(ensure_trusted_overlay_window("window__streaming_overlays").is_ok());
        assert!(ensure_trusted_overlay_window("window__community_widget").is_err());
        assert!(ensure_trusted_overlay_window("plugin-community-widget").is_err());
        assert!(ensure_trusted_overlay_window("assistant-1").is_err());
    }

    #[test]
    fn capability_message_uses_the_existing_obs_action_contract() {
        let message = build_obs_overlay_capability_message(
            "integration-guid",
            "http://127.0.0.1:9001/overlay.html?overlayToken=secret",
        );
        assert_eq!(message["payload"]["actionId"], "obs_overlay_set_data");
        assert_eq!(message["payload"]["integrationGuid"], "integration-guid");
        assert_eq!(
            message["payload"]["parameters"]["url"],
            "http://127.0.0.1:9001/overlay.html?overlayToken=secret"
        );
    }
}
