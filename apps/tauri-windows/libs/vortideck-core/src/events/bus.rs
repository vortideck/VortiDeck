use super::types::EventPayload;
use tokio::sync::broadcast;

/// Event bus for broadcasting all VortiDeck events using tokio broadcast channels
///
/// This provides a platform-agnostic event system that works in:
/// - Tauri desktop applications
/// - Standalone servers
/// - Unit tests
///
/// Uses tokio's broadcast channel which is:
/// - Multi-producer, multi-consumer
/// - Lock-free and efficient
/// - Already in the dependency tree (no extra deps)
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<EventPayload>,
}

impl EventBus {
    /// Create a new event bus with the specified buffer size
    ///
    /// Buffer size determines how many events can be queued before
    /// slow subscribers start missing events (they'll get RecvError::Lagged)
    ///
    /// Recommended: 1000 for desktop, 10000 for servers
    pub fn new(buffer_size: usize) -> Self {
        let (tx, _rx) = broadcast::channel(buffer_size);
        Self { tx }
    }

    /// Create a new event bus with default buffer size (10000)
    ///
    /// Increased from 1000 to handle:
    /// - System events (volume slider can generate 100+ events/sec)
    /// - Stress testing scenarios
    /// - Multiple event types (Flow, System, Webhook, Relay)
    ///
    /// Memory cost: ~2-5 MB (negligible on desktop)
    /// Provides 20 seconds of buffer at peak rate (500 events/sec)
    pub fn new_default() -> Self {
        Self::new(10000)
    }

    /// Subscribe to events
    ///
    /// Returns a receiver that will receive all future events.
    /// Each subscriber gets their own independent copy of events.
    pub fn subscribe(&self) -> broadcast::Receiver<EventPayload> {
        self.tx.subscribe()
    }

    /// Emit an event to all subscribers
    ///
    /// If there are no subscribers, the event is silently dropped.
    /// Returns Ok(subscriber_count) or Err if the event couldn't be sent.
    pub fn emit(
        &self,
        event: EventPayload,
    ) -> Result<usize, broadcast::error::SendError<EventPayload>> {
        match self.tx.send(event) {
            Ok(receiver_count) => Ok(receiver_count),
            // Tokio reports "no active receivers" as SendError. For this
            // live-notification bus that is an expected no-op, not a runtime
            // failure: durable workflow inputs are persisted separately.
            Err(_) => Ok(0),
        }
    }

    /// Get the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{EventPayload, FlowEventPayload};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_event_bus_basic() {
        let bus = EventBus::new_default();

        // Subscribe before emitting
        let mut rx = bus.subscribe();

        // Emit event
        let result = bus.emit(EventPayload::Flow(FlowEventPayload::FlowStarted {
            flow_id: "test-flow".to_string(),
            run_id: "test-run".to_string(),
            flow_name: "Test Flow".to_string(),
        }));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1); // 1 subscriber

        // Receive event
        let event = rx.recv().await.unwrap();
        match event {
            EventPayload::Flow(FlowEventPayload::FlowStarted {
                flow_id, flow_name, ..
            }) => {
                assert_eq!(flow_id, "test-flow");
                assert_eq!(flow_name, "Test Flow");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_event_bus_multiple_subscribers() {
        let bus = EventBus::new_default();

        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        assert_eq!(bus.subscriber_count(), 2);

        // Emit event
        bus.emit(EventPayload::Flow(FlowEventPayload::FlowCompleted {
            flow_id: "test-flow".to_string(),
            run_id: "test-run".to_string(),
            result: HashMap::new(),
        }))
        .unwrap();

        // Both subscribers should receive it
        let event1 = rx1.recv().await.unwrap();
        let event2 = rx2.recv().await.unwrap();

        matches!(
            event1,
            EventPayload::Flow(FlowEventPayload::FlowCompleted { .. })
        );
        matches!(
            event2,
            EventPayload::Flow(FlowEventPayload::FlowCompleted { .. })
        );
    }

    #[tokio::test]
    async fn test_event_bus_no_subscribers() {
        let bus = EventBus::new_default();

        // Emit with no subscribers should not error
        let result = bus.emit(EventPayload::Flow(FlowEventPayload::FlowStarted {
            flow_id: "test-flow".to_string(),
            run_id: "test-run".to_string(),
            flow_name: "Test Flow".to_string(),
        }));

        // broadcast::send returns Ok(0) when there are no receivers
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }
}
