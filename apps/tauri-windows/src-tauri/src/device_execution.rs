//! OBS activation and repeatable physical-button action execution.
use super::*;

pub(super) async fn activate_obs_plugin(
    tx: broadcast::Sender<Message>,
    state_manager: Arc<StateManager>,
) -> (String, String) {
    // Set up OBS plugin sender in the global StateManager
    let sender = tx.clone();
    state_manager.set_obs_plugin_sender(sender).await;

    // Use the OBS integration directly for activation
    let obs_integration =
        vortideck_core::integrations::builtin::obs::ObsIntegration::new(state_manager.clone());

    // Activate the OBS integration directly
    let result = match obs_integration.activate().await {
        Ok((integration_guid, instance_guid)) => {
            println!("✅ OBS plugin activated successfully via integration");
            println!("Generated integration GUID: {}", integration_guid);
            println!("Generated instance GUID: {}", instance_guid);
            (integration_guid, instance_guid)
        }
        Err(e) => {
            println!("❌ Failed to activate OBS plugin via integration: {:?}", e);
            // Fallback to generated GUIDs
            let fallback_integration_guid = format!("obs_plugin_{}", uuid::Uuid::new_v4());
            let fallback_instance_guid = format!("obs_plugin_{}", uuid::Uuid::new_v4());
            (fallback_integration_guid, fallback_instance_guid)
        }
    };

    // 🎯 Start automatic banner rotation when OBS successfully connects
    tokio::spawn(async move {
        // Give OBS a moment to fully initialize
        tokio::time::sleep(Duration::from_millis(500)).await;

        match banners::start_automatic_banner_rotation(state_manager, None).await {
            Ok(()) => println!("✅ Banner rotation system started for OBS"),
            Err(e) => println!("❌ Failed to start banner rotation: {}", e),
        }
    });

    result
}

// Button repeat management functions
pub(super) async fn start_button_repeat(
    button_id: i32,
    db: Arc<Database>,
    tx: broadcast::Sender<Message>,
    button_repeat_map: ButtonRepeatMap,
    app_handle: AppHandle,
) {
    // Stop any existing repeat for this button
    stop_button_repeat(button_id, button_repeat_map.clone()).await;

    // Get button configuration
    let button = match db.get_button_by_id(button_id).await {
        Ok(Some(button)) => button,
        _ => {
            println!("❌ Button {} not found", button_id);
            return;
        }
    };

    // Parse execution config
    let execution_config = button.get_execution_config().unwrap_or(None);
    let (hold_threshold_ms, repeat_interval_ms) = parse_execution_config(execution_config);

    println!(
        "🔧 Button {} config: hold_threshold={}ms, repeat_interval={}ms",
        button_id, hold_threshold_ms, repeat_interval_ms
    );

    // Create stop signal channel
    let (stop_sender, mut stop_receiver) = broadcast::channel(1);

    // Clone variables for the repeat task
    let db_clone = db.clone();
    let tx_clone = tx.clone();
    let app_handle_clone = app_handle.clone();

    // Start the repeat task
    let repeat_handle = tokio::spawn(async move {
        // Wait for hold threshold
        println!("⏰ Waiting {}ms for hold threshold...", hold_threshold_ms);
        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(hold_threshold_ms)) => {
                println!("✅ Hold threshold reached, starting repeat");

                // Notify frontend that repeat has started
                let repeat_start_msg = serde_json::json!({
                    "action": "button_repeat_started",
                    "buttonId": button_id
                });

                if let Err(e) = tx_clone.send(Message::Text(repeat_start_msg.to_string().into())) {
                    println!("❌ Failed to send repeat start notification: {}", e);
                }

                // Execute immediately when hold starts
                if let Some(action_str) = &button.action {
                    execute_button_action(button_id, action_str, &db_clone, &tx_clone, &app_handle_clone).await;
                }

                // Start repeating
                let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(repeat_interval_ms));
                interval.tick().await; // Skip first tick (immediate execution already happened)

                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            if let Some(action_str) = &button.action {
                                execute_button_action(button_id, action_str, &db_clone, &tx_clone, &app_handle_clone).await;
                            }
                        }
                        _ = stop_receiver.recv() => {
                            println!("🛑 Repeat stopped for button {}", button_id);
                            break;
                        }
                    }
                }
            }
            _ = stop_receiver.recv() => {
                println!("🛑 Repeat cancelled before threshold for button {}", button_id);
            }
        }
    });

    // Store the repeat state
    let repeat_state = ButtonRepeatState {
        button_id,
        repeat_handle: Some(repeat_handle),
        stop_sender: Some(stop_sender),
    };

    let mut map = button_repeat_map.write().await;
    map.insert(button_id, repeat_state);
    println!("📋 Repeat state stored for button {}", button_id);
}

pub(super) async fn stop_button_repeat(button_id: i32, button_repeat_map: ButtonRepeatMap) {
    let mut map = button_repeat_map.write().await;

    if let Some(mut repeat_state) = map.remove(&button_id) {
        println!("🔄 Stopping repeat for button {}", button_id);

        // Send stop signal
        if let Some(stop_sender) = repeat_state.stop_sender.take() {
            let _ = stop_sender.send(());
        }

        // Abort the task if it's still running
        if let Some(repeat_handle) = repeat_state.repeat_handle.take() {
            repeat_handle.abort();
        }

        println!("✅ Repeat stopped for button {}", button_id);
    }
}

fn parse_execution_config(config: Option<serde_json::Value>) -> (u64, u64) {
    let default_hold_threshold = 300;
    let default_repeat_interval = 100;

    if let Some(config) = config {
        let hold_threshold = config
            .get("hold_threshold_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(default_hold_threshold);

        let repeat_interval = config
            .get("repeat_interval_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(default_repeat_interval);

        (hold_threshold, repeat_interval)
    } else {
        (default_hold_threshold, default_repeat_interval)
    }
}

async fn execute_button_action(
    button_id: i32,
    action_str: &str,
    db: &Arc<Database>,
    tx: &broadcast::Sender<Message>,
    app_handle: &AppHandle,
) {
    // Parse action
    match serde_json::from_str::<Action>(action_str) {
        Ok(action) => {
            // Execute the action
            execute_action_universal(
                action,
                None, // No value for repeat actions
                Some(button_id.to_string()),
                db.clone(),
                tx.clone(),
                None,
            )
            .await;
        }
        Err(e) => {
            println!("❌ Failed to parse action for button {}: {}", button_id, e);
        }
    }
}
