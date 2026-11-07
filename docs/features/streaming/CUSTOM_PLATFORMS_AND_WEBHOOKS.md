# Extensibility: Custom Platforms, Webhooks, and Third-Party Sockets

This document details the architectural design for connecting custom streaming platforms, local/cloud webhooks, and third-party WebSocket feeds to VortiDeck's unified `EventBus` and overlays without modifying the Rust core.

---

## The Core Design: Schemaless Generic Events

To prevent hardcoding the application to a fixed list of platforms (Twitch, YouTube, Kick), the `EventPayload` incorporates a schemaless `Generic` variant:

```rust
pub enum EventPayload {
    // ... standard categories ...
    Stream(StreamEventPayload), // Strict platform events (Twitch, YT, Kick defaults)
    
    /// Schemaless variant for custom platforms, webhooks, and third-party integrations
    Generic {
        source: String,             // e.g., "ko-fi", "custom-webhook", "trovo"
        event_type: String,         // e.g., "donation", "chat_message", "sensor_trigger"
        payload: serde_json::Value, // Dynamic JSON structure
    }
}
```

This allows any external script, custom server, or plugin to publish unstructured JSON payloads to the EventBus. The **Overlay Elements** and **Flow Nodes** can map to these keys dynamically using dot-notation path selectors (e.g. `payload.data.username` or `payload.amount`) which are resolved at runtime.

---

## Integration Pathways

```
               [ Platform Outgoing Webhooks ]
                             │
                             ├───────► Behind NAT? ──► [ Relay Backend (Cloud) ]
                             │                                │ (E2EE WS Relay)
                             │                                ▼
                             └───────► Direct HTTP ──► [ Local Warp Webhook API ]
                                                              │
                                                              ▼
[ Third-Party WebSocket API ] ────────► Plugin ──────► [ Local EventBus ]
                                                              │
                                                              ▼
                                                     [ OBS / Canvas Overlays ]
```

### 1. HTTP Webhooks (Ko-fi, Stripe, custom API callbacks)
Webhooks are fully compatible with this architecture:
* **Local Network**: The custom service fires an HTTP POST request directly to the local Warp route `http://localhost:<port>/webhook/:id` ([webhook_routes.rs](../../../apps/tauri-windows/src-tauri/src/adapters/webhook_routes.rs)).
* **External Webhooks (NAT/Firewall Bypass)**: For services located on the internet, the callback hits a dedicated endpoint on the **VortiDeck Relay Backend** (e.g., `https://relay.vortideck.com/webhook/:id`). The stateless relay backend forwards the raw JSON payload down the active E2EE WebSocket connection.
* **Ingestion**: The local application receives the payload, wraps it in the `EventPayload::Generic` enum, and publishes it directly to the local `EventBus` for immediate processing by Flows and Overlays.

### 2. Custom WebSockets (Trovo, local socket servers)
* **The Constraint**: The built-in `StreamingWebSocket` client ([websocket.rs](../../../apps/tauri-windows/libs/vortideck-core/src/services/streaming/websocket.rs)) parses the unified `CloudEvent` struct ([models.rs](../../../apps/tauri-windows/libs/vortideck-core/src/services/streaming/models.rs)). Pointing it at an arbitrary socket server will fail.
* **The Solution**: 
  1. The developer registers a local **VortiDeck Plugin**.
  2. The plugin opens its own custom TCP/WebSocket connection directly to the target platform.
  3. When it receives a message, it invokes the host API to emit the data to the central EventBus using the `EventPayload::Generic` variant.
  4. The overlays and flow engine process the event identically.

---

## Overlay Data Bindings

When a custom service sends a generic event payload, the overlay dynamic update works as follows:

1. **Schema Definition**: The custom element defines template placeholders matching expected JSON paths:
   ```html
   <div class="custom-widget">
     New Alert: {{payload.user}} has sent {{payload.val}}!
   </div>
   ```
2. **Component Runtime Update**: The overlay's WebSocket listener receives the generic event payload and maps the variables dynamically:
   ```typescript
   // useOverlayEngine.ts
   if (msg.type === "Generic" && msg.source === element.binding.sourceId) {
     element.data.user = getNestedValue(msg.payload, "user");
     element.data.val = getNestedValue(msg.payload, "val");
   }
   ```
3. **Parity**: Since the EventBus broadcasts to the unified Warp WebSocket channel, both the **Canvas Editor** (Tauri) and the **OBS Browser Source** (WebSocket Client) parse the event and render it in sync.
