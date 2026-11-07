//! One validation adapter for live delivery and reconnect of persisted commands.
use super::OverlayState;
use serde_json::{json, Value};
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tokio::sync::RwLock;
use vortideck_core::events::{OverlayCommandPayload, OverlayTarget};
use vortideck_core::plugins::{widget_commands::accepts_command, Contribution};

pub async fn delivery_message(
    app: &AppHandle,
    command: &OverlayCommandPayload,
    replay: bool,
) -> Result<Value, String> {
    if command.schema_version != 1
        || command.command_id.is_empty()
        || command.delivery_revision == 0
    {
        return Err("Invalid overlay command envelope".into());
    }
    if command
        .expires_at
        .is_some_and(|expiry| expiry <= chrono::Utc::now())
    {
        return Err("Overlay command expired".into());
    }
    let state = app.state::<Arc<RwLock<OverlayState>>>();
    let elements = {
        let guard = state.read().await;
        guard
            .current_design
            .elements
            .iter()
            .filter(|element| guard.is_element_renderable(element))
            .cloned()
            .collect::<Vec<_>>()
    };
    let core = app.state::<vortideck_core::VortiDeckCore>();
    let mut eligible = Vec::new();
    for element in &elements {
        if let OverlayTarget::Widget { widget_id } = &command.target
            && element.id != *widget_id
        {
            continue;
        }
        let contribution = super::settings::contribution_for(
            core.inner(),
            element.package_id.as_deref(),
            element.contribution_id.as_deref(),
        )
        .await?;
        let accepted = match contribution {
            Some(Contribution::Widget { commands, .. }) => {
                accepts_command(commands.as_ref(), &command.command, &command.payload)?
            }
            None if element.package_id.is_some() => false,
            _ => accepts_command(None, &command.command, &command.payload)?,
        };
        if accepted {
            eligible.push(element.id.clone());
        }
    }
    // Existing host playback uses the durable integration queue, not widget
    // code. It intentionally works even when the composition has no widgets.
    let host_playback = matches!(command.target, OverlayTarget::All)
        && matches!(
            command.command.as_str(),
            "overlay.alert.play" | "overlay.media.play"
        );
    if eligible.is_empty() && !host_playback {
        return Err(
            "No visible, authorized widget instance accepts this command; use a placed instance ID"
                .into(),
        );
    }
    Ok(json!({"category":"overlay","payload":command,"eligibleWidgetIds":eligible,"replay":replay}))
}

pub async fn reject_delivery(app: &AppHandle, command: &OverlayCommandPayload, error: String) {
    use vortideck_core::events::{
        OverlayCommandAckStatus, OverlayCommandAcknowledgement, OverlayCommandReceiptPhase,
    };
    let receipt = OverlayCommandAcknowledgement {
        command_id: command.command_id.clone(),
        status: OverlayCommandAckStatus::Rejected,
        receipt_phase: OverlayCommandReceiptPhase::HostAccepted,
        instance_id: None,
        applied_revision: Some(command.delivery_revision),
        error: Some(error),
    };
    if let Some(overlay) =
        app.try_state::<Arc<vortideck_core::integrations::builtin::overlay::OverlayIntegration>>()
    {
        let _ = overlay.acknowledge(receipt.clone()).await;
    }
    if let Some(state) =
        app.try_state::<vortideck_core::services::flow::commands::FlowExecutorState>()
        && let Some(executor) = state.executor.lock().await.clone()
    {
        executor.acknowledge_overlay_command(receipt).await;
    }
}
