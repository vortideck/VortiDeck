//! Event Bus Demonstration
//! Shows how to use the high-performance Tokio broadcast EventBus in VortiDeck.
//!
//! Run with: cargo run --example event_emitter_demo

use chrono::Utc;
use serde_json::json;
use vortideck_core::events::{
    EventBus, EventPayload, StreamEventMetadata, StreamEventPayload, WebhookEventPayload,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 VortiDeck Event Bus Demo\n");

    let event_bus = EventBus::new_default();
    let mut subscriber1 = event_bus.subscribe();
    let mut subscriber2 = event_bus.subscribe();

    // Spawn subscriber tasks
    let task1 = tokio::spawn(async move {
        while let Ok(event) = subscriber1.recv().await {
            match event {
                EventPayload::Stream(stream_event) => {
                    println!(
                        "  [Sub 1 - Stream] Received: {:?}",
                        stream_event.event_type()
                    );
                }
                EventPayload::Webhook(webhook_event) => {
                    println!(
                        "  [Sub 1 - Webhook] Received webhook event: {:?}",
                        webhook_event
                    );
                }
                _ => {}
            }
        }
    });

    let task2 = tokio::spawn(async move {
        while let Ok(event) = subscriber2.recv().await {
            match event {
                EventPayload::Webhook(WebhookEventPayload::WebhookReceived {
                    webhook_id, ..
                }) => {
                    println!(
                        "  [Sub 2 - Webhook Monitor] Ingested webhook ID: {}",
                        webhook_id
                    );
                }
                _ => {}
            }
        }
    });

    // Emit stream event
    println!("📍 Emitting Stream Event (Twitch Follow)...");
    event_bus.emit(EventPayload::Stream(StreamEventPayload::Follow {
        metadata: StreamEventMetadata::new(1001, "twitch"),
        user_name: "johndoe".to_string(),
        user_display_name: "JohnDoe".to_string(),
    }))?;

    // Emit webhook event
    println!("📍 Emitting Webhook Event (Stripe Payment)...");
    event_bus.emit(EventPayload::Webhook(
        WebhookEventPayload::WebhookReceived {
            webhook_id: "wh_stripe_123".to_string(),
            timestamp: Utc::now().timestamp_millis(),
            event_id: Some("evt_stripe_999".to_string()),
            service: Some("stripe".to_string()),
            flow_id: None,
            payload: Some(json!({ "type": "payment_intent.succeeded", "amount": 2500 })),
            is_test: false,
        },
    ))?;

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    drop(event_bus);

    let _ = tokio::join!(task1, task2);

    println!("\n✅ All demos completed successfully!");
    Ok(())
}
