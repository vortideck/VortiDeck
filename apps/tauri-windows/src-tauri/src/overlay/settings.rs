//! One authoring-settings boundary for the editor and assistant tools.
use super::{OverlayElement, OverlayState};
use serde_json::{json, Value};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::RwLock;
use vortideck_core::plugins::{
    contributions::Contribution, widget_settings::validate_widget_config,
};

pub struct DesktopOverlayWidgetSettings(pub AppHandle);

#[async_trait::async_trait]
impl vortideck_core::services::mcp::overlay_settings::OverlayWidgetSettingsApi
    for DesktopOverlayWidgetSettings
{
    async fn list(&self) -> anyhow::Result<Value> {
        let state = self.0.state::<Arc<RwLock<OverlayState>>>();
        let guard = state.read().await;
        Ok(json!(guard.current_design.elements.iter().map(|el| json!({
            "elementId": el.id, "packageId": el.package_id, "contributionId": el.contribution_id,
            "visible": el.visible, "configRevision": el.config_revision,
        })).collect::<Vec<_>>()))
    }
    async fn inspect(&self, element_id: &str) -> anyhow::Result<Value> {
        inspect(&self.0, element_id)
            .await
            .map_err(anyhow::Error::msg)
    }
    async fn configure(
        &self,
        element_id: &str,
        config: Value,
        expected_revision: u64,
    ) -> anyhow::Result<Value> {
        configure(&self.0, element_id, config, expected_revision)
            .await
            .map_err(anyhow::Error::msg)
    }
}

pub async fn schema_for(
    core: &vortideck_core::VortiDeckCore,
    package_id: Option<&str>,
    contribution_id: Option<&str>,
) -> Result<Option<Value>, String> {
    Ok(match contribution_for(core, package_id, contribution_id).await? {
        Some(Contribution::Widget { config_schema, .. }) => config_schema,
        _ => None,
    })
}

pub async fn contribution_for(
    core: &vortideck_core::VortiDeckCore,
    package_id: Option<&str>,
    contribution_id: Option<&str>,
) -> Result<Option<Contribution>, String> {
    let Some(package_id) = package_id else {
        return Ok(None);
    };
    let contributions = if vortideck_core::plugins::canonical_package_id(package_id)
        == "builtin.vortideck-widgets"
    {
        vortideck_core::plugins::contributions::builtin_overlay_package().contributions
    } else {
        // A removed package can retain its saved layout/configuration. It has no
        // rendering authority; do not discard a project merely because it is absent.
        let Some(manifest) = core.plugin_manager().package_manifest(package_id).await else {
            return Ok(None);
        };
        manifest.contributions
    };
    let widgets: Vec<_> = contributions
        .iter()
        .filter(|c| matches!(c, Contribution::Widget { .. }))
        .collect();
    let selected = widgets
        .iter()
        .enumerate()
        .find(|(index, widget)| {
            let Contribution::Widget { id, .. } = widget else {
                return false;
            };
            contribution_id.map_or(widgets.len() == 1, |requested| {
                requested == id.as_deref().unwrap_or(&format!("widget-{index}"))
            })
        })
        .map(|(_, widget)| *widget);
    match selected {
        Some(widget) => Ok(Some(widget.clone())),
        _ => Err(format!(
            "Select an existing widget contribution in package '{package_id}'"
        )),
    }
}

pub async fn validate_element(
    core: &vortideck_core::VortiDeckCore,
    element: &mut OverlayElement,
) -> Result<(), String> {
    let schema = schema_for(
        core,
        element.package_id.as_deref(),
        element.contribution_id.as_deref(),
    )
    .await?;
    element.config = validate_widget_config(schema.as_ref(), &element.config)?;
    Ok(())
}

pub async fn inspect(app: &AppHandle, element_id: &str) -> Result<Value, String> {
    let state = app.state::<Arc<RwLock<OverlayState>>>();
    let element = state
        .read()
        .await
        .get_element(element_id)
        .cloned()
        .ok_or("Overlay element not found")?;
    let core = app.state::<vortideck_core::VortiDeckCore>();
    let contribution = contribution_for(
        core.inner(),
        element.package_id.as_deref(),
        element.contribution_id.as_deref(),
    )
    .await?;
    let (schema, commands) = match contribution {
        Some(Contribution::Widget { config_schema, commands, .. }) => (config_schema, commands),
        _ => (None, None),
    };
    Ok(
        json!({"elementId": element.id, "packageId": element.package_id,
        "contributionId": element.contribution_id, "configSchema": schema, "commands": commands,
        "config": element.config, "configRevision": element.config_revision}),
    )
}

pub async fn configure(
    app: &AppHandle,
    element_id: &str,
    config: Value,
    expected_revision: u64,
) -> Result<Value, String> {
    let state = app.state::<Arc<RwLock<OverlayState>>>();
    let original = state
        .read()
        .await
        .get_element(element_id)
        .cloned()
        .ok_or("Overlay element not found")?;
    let core = app.state::<vortideck_core::VortiDeckCore>();
    let schema = schema_for(
        core.inner(),
        original.package_id.as_deref(),
        original.contribution_id.as_deref(),
    )
    .await?;
    let config = validate_widget_config(schema.as_ref(), &config)?;
    let message = {
        let mut guard = state.write().await;
        let current = guard
            .get_element(element_id)
            .ok_or("Overlay element not found")?;
        if current.package_id != original.package_id
            || current.contribution_id != original.contribution_id
        {
            return Err(
                "Widget settings changed. Reload the current settings before applying your edit."
                    .into(),
            );
        }
        let candidate =
            prepare_config_update(&guard, element_id, config.clone(), expected_revision)?;
        let revision = candidate
            .get_element(element_id)
            .ok_or("Overlay element not found")?
            .config_revision;
        // Publish only a snapshot that the host successfully persisted.
        candidate.persist(app)?;
        *guard = candidate;
        json!({"type": "WIDGET_CONFIG_UPDATED", "version": 1, "elementId": element_id,
            "config": config, "configRevision": revision})
    };
    if let Some(clients) = app.try_state::<Arc<crate::client_registry::ClientRegistry>>() {
        clients.send_to_overlays(&message.to_string()).await;
    }
    // A delivery failure must not report the durable write as failed. Editor
    // inspection and OBS reconnect both recover from the native snapshot.
    if let Err(error) = app.emit("overlay-widget-config-updated", &message) {
        log::warn!("Widget settings saved, but editor notification failed: {error}");
    }
    Ok(message)
}

fn prepare_config_update(
    state: &OverlayState,
    element_id: &str,
    config: Value,
    expected_revision: u64,
) -> Result<OverlayState, String> {
    let current = state
        .get_element(element_id)
        .ok_or("Overlay element not found")?;
    if current.config_revision != expected_revision {
        return Err(
            "Widget settings changed. Reload the current settings before applying your edit."
                .into(),
        );
    }
    let revision = if current.config == config {
        current.config_revision
    } else {
        current.config_revision.saturating_add(1)
    };
    let mut candidate = state.clone();
    candidate.update_element(element_id, |element| {
        element.config = config;
        element.config_revision = revision;
    });
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_stage_without_mutating_runtime_and_reject_stale_edits() {
        let element: OverlayElement = serde_json::from_value(json!({
            "id":"goal","element_type":"widget","content":"","position":{"x":20,"y":30},
            "size":{"width":300,"height":100},"visible":true,
            "data":{"current":42},"config":{"goal":100},"config_revision":2,
            "render_html":"<iframe srcdoc='unchanged'></iframe>"
        }))
        .unwrap();
        let mut state = OverlayState::new();
        state.add_element(element);
        let candidate = prepare_config_update(&state, "goal", json!({"goal":200}), 2).unwrap();
        let changed = candidate.get_element("goal").unwrap();
        assert_eq!(changed.config_revision, 3);
        assert_eq!(changed.data, Some(json!({"current":42})));
        assert_eq!(
            changed.render_html.as_deref(),
            Some("<iframe srcdoc='unchanged'></iframe>")
        );
        assert_eq!(changed.position.x, 20.0);
        assert_eq!(
            state.get_element("goal").unwrap().config,
            json!({"goal":100})
        );
        assert!(prepare_config_update(&candidate, "goal", json!({"goal":300}), 2).is_err());
        assert_eq!(
            prepare_config_update(&candidate, "goal", json!({"goal":200}), 3)
                .unwrap()
                .get_element("goal")
                .unwrap()
                .config_revision,
            3
        );
        let saved = serde_json::to_value(&candidate.current_design).unwrap();
        let restored: super::super::OverlayDesign = serde_json::from_value(saved).unwrap();
        assert_eq!(restored.elements[0].config, json!({"goal":200}));
        assert_eq!(restored.elements[0].config_revision, 3);
        assert!(candidate.generated_html.contains("data-widget-config="));
    }
}

#[tauri::command]
pub async fn get_overlay_widget_settings(
    app: AppHandle,
    window: tauri::WebviewWindow,
    element_id: String,
) -> Result<Value, String> {
    super::super::commands::overlay_commands::ensure_trusted_overlay_window(window.label())?;
    inspect(&app, &element_id).await
}

#[tauri::command]
pub async fn configure_overlay_widget(
    app: AppHandle,
    window: tauri::WebviewWindow,
    element_id: String,
    config: Value,
    expected_revision: u64,
) -> Result<Value, String> {
    super::super::commands::overlay_commands::ensure_trusted_overlay_window(window.label())?;
    configure(&app, &element_id, config, expected_revision).await
}
