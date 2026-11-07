// Direct state manager creation for Tauri (no factory pattern)
// Simple, direct implementation that creates StateManager with Tauri event bridge

use serde_json::Value;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::broadcast::{self, error::RecvError};
use vortideck_core::core::StateManager;

const EVENT_BRIDGE_CAPACITY: usize = 1000;
pub const STATE_BRIDGE_RESYNC_REQUIRED: &str = "state_bridge_resync_required";

/// Direct state manager creation function
/// No factory pattern - just create StateManager with simple event bridge
pub fn create_state_manager(app_handle: AppHandle) -> Arc<StateManager> {
    log::info!("Creating StateManager with Tauri event bridge");

    // Create event bridge for StateManager to Tauri communication
    let event_bridge = create_event_bridge(app_handle.clone());

    // Create StateManager with event sender
    let state_manager = Arc::new(StateManager::new().with_event_sender(event_bridge));

    log::info!("StateManager created successfully");
    state_manager
}

/// Create a simple event bridge between StateManager and Tauri
/// No factory pattern - just a direct bridge function
fn create_event_bridge(app_handle: AppHandle) -> broadcast::Sender<(String, Value)> {
    let (event_sender, event_receiver) =
        broadcast::channel::<(String, Value)>(EVENT_BRIDGE_CAPACITY);

    tokio::spawn(async move {
        forward_event_bridge(event_receiver, move |event_name, event_data| {
            app_handle
                .emit(event_name, event_data)
                .map_err(|error| error.to_string())
        })
        .await;
    });

    event_sender
}

async fn forward_event_bridge<Emit>(
    mut receiver: broadcast::Receiver<(String, Value)>,
    mut emit: Emit,
) where
    Emit: FnMut(&str, &Value) -> Result<(), String>,
{
    loop {
        match receiver.recv().await {
            Ok((event_name, event_data)) => {
                if let Err(error) = emit(&event_name, &event_data) {
                    log::error!("Failed to emit event '{event_name}' to Tauri: {error}");
                }
            }
            Err(RecvError::Lagged(dropped_events)) => {
                log::warn!(
                    "State-to-UI event bridge lagged by {dropped_events} event(s); requesting an authoritative snapshot refresh"
                );
                let payload = serde_json::json!({ "droppedEvents": dropped_events });
                if let Err(error) = emit(STATE_BRIDGE_RESYNC_REQUIRED, &payload) {
                    log::error!("Failed to request a state-to-UI snapshot refresh: {error}");
                }
            }
            Err(RecvError::Closed) => {
                log::debug!("State-to-UI event bridge closed");
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[tokio::test]
    async fn bridge_requests_resync_after_lag_then_forwards_later_updates() {
        let (sender, receiver) = broadcast::channel(2);
        sender.send(("first".into(), serde_json::json!(1))).unwrap();
        sender
            .send(("second".into(), serde_json::json!(2)))
            .unwrap();
        sender.send(("third".into(), serde_json::json!(3))).unwrap();

        let emitted = Arc::new(Mutex::new(Vec::<(String, Value)>::new()));
        let emitted_for_bridge = emitted.clone();
        let bridge = tokio::spawn(async move {
            forward_event_bridge(receiver, move |event_name, event_data| {
                emitted_for_bridge
                    .lock()
                    .unwrap()
                    .push((event_name.to_owned(), event_data.clone()));
                Ok(())
            })
            .await;
        });

        tokio::task::yield_now().await;
        sender.send(("later".into(), serde_json::json!(4))).unwrap();
        drop(sender);

        tokio::time::timeout(Duration::from_secs(1), bridge)
            .await
            .expect("closed bridge should stop")
            .expect("bridge task should not panic");

        let emitted = emitted.lock().unwrap();
        assert_eq!(emitted[0].0, STATE_BRIDGE_RESYNC_REQUIRED);
        assert_eq!(emitted[0].1["droppedEvents"], 1);
        assert!(emitted
            .iter()
            .any(|(name, data)| name == "later" && data == 4));
    }

    #[tokio::test]
    async fn bridge_exits_normally_when_all_senders_close() {
        let (sender, receiver) = broadcast::channel(2);
        let bridge = tokio::spawn(forward_event_bridge(receiver, |_, _| Ok(())));
        drop(sender);

        tokio::time::timeout(Duration::from_secs(1), bridge)
            .await
            .expect("closed bridge should stop")
            .expect("bridge task should not panic");
    }
}
