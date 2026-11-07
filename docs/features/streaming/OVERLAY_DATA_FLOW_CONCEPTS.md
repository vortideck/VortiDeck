# Overlay Data Flow: Direct vs. Flow-Mediated Event Routing

This document defines the architectural concept for routing events from ingestion points to both overlay widgets (on the Canvas and in OBS Browser Sources) and the Flow Engine.

---

## Architecture Overview

```
                      [ Stream Events (Twitch/YT/Kick) ]
                      [ Generic Webhooks (Ko-fi/Stripe) ]
                                      │
                                      ▼
                             [ Rust EventBus ]
                                      │
           ┌──────────────────────────┴──────────────────────────┐
           │ Direct Binding Path                                 │ Flow-Mediated Path
           ▼                                                     ▼
   [ Rust Warp Server ]                                    [ Flow Engine ]
   (Direct WS Broadcast)                                (Filter/Throttle/AI/Logic)
           │                                                     │
           │                                                     ▼
           │                                             [ Flow Output Node ]
           │                                                     │
           │                                                     ▼
           ▼                                            [ EventBus Emitted ]
    [ OBS Overlay ]                                              │
    (WebSocket Client) ◄─────────────────────────────────────────┘
```

---

## 1. Direct Binding vs. Flow-Mediated Routing

VortiDeck supports two distinct pathways for rendering events on overlays:

### Direct Binding
* **How it works**: An overlay element (e.g., a "Chat Widget" or "Subscriber Alert Box") binds directly to an event type on the EventBus (e.g., `StreamEventPayload::ChatMessage` or `EventPayload::Generic` with source `"ko-fi"`).
* **When to use**: For simple, real-time widgets that do not require processing, calculations, or external logic.
* **Benefits**: Ultra-low latency, bypasses execution engine completely.

### Flow-Mediated Routing
* **How it works**: 
  1. An event triggers a **Flow** via a Trigger Node (e.g., `On Twitch Chat`).
  2. The Flow runs standard nodes (e.g., filtering toxic words, aggregating a donation goal, calling an AI moderator, delaying the action).
  3. The Flow terminates at a **Universal Output Node** (or an overlay action node like `ShowOverlayElement`).
  4. The output node publishes a `FlowCompleted` event payload containing the final computed data.
  5. The overlay element, configured to listen to that specific `flow_id`, receives the payload and updates its DOM.
* **When to use**: When you need filtering, moderation, variable accumulation, delays, or complex conditional logic before showing something on stream.

> [!NOTE]
> **Independent Flow Execution**:
> Flows run completely independently of whether any overlay element is currently listening to them. If a flow triggers and runs to completion, its outputs are published. If no overlay element is subscribed to that `flow_id`, the payload is simply discarded. This ensures decoupled development and modular design.

---

## 2. OBS Direct WebSocket Bypass

To ensure high-frequency events (like active Twitch chat or rapid webhook notifications) do not stutter or cause UI lag, we avoid routing messages through the main Tauri renderer:

> [!IMPORTANT]
> **Tauri UI Thread Bypass**:
> Routing events through `Rust -> Tauri IPC -> Vue App -> WebSocket -> OBS` blocks the single-threaded Vue renderer and introduces latency.
> Instead, the Rust Warp server subscribes directly to the `EventBus` in a background task and broadcasts events over WebSocket directly to OBS clients connected at `/ws/overlay`.

---

## 3. Data Schema and Binding Resolution

Overlays process template variables using a simple double-curly-braces interpolation scheme:

```html
<div class="alert">
  Thanks {{user}} for the {{amount}} donation!
</div>
```

### Direct Event Binding Resolver
For direct events, variables are mapped to JSON paths in the incoming event:
* `user` $\rightarrow$ `payload.user_name` (from `StreamEventPayload`)
* `amount` $\rightarrow$ `payload.amount` (from `GenericEventPayload`)

### Flow Output Binding Resolver
For flow-mediated events, variables are resolved from the `result` map of the `FlowCompleted` event:
* `user` $\rightarrow$ `result.moderated_username`
* `amount` $\rightarrow$ `result.formatted_amount`
