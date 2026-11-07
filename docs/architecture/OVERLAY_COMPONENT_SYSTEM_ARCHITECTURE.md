# Overlay Component System Architecture

> Comprehensive design for the schema-based overlay component system, scripting/data layer integration, and how it connects to VortiDeck's core engine.

## Table of Contents

1. [Problem Statement](#problem-statement)
2. [Current State](#current-state)
3. [Architecture Overview](#architecture-overview)
4. [Phase 1: Schema-Based Component System](#phase-1-schema-based-component-system)
5. [Phase 2: Streaming Node Provider + EventBus Bridge](#phase-2-streaming-node-provider--eventbus-bridge)
6. [Phase 3: Overlay Node Provider + Component Runtime](#phase-3-overlay-node-provider--component-runtime)
7. [Phase 4: Component Creator UI](#phase-4-component-creator-ui)
8. [Data Flow Architecture](#data-flow-architecture)
9. [How Custom Logic Works (No Scripting Language Needed)](#how-custom-logic-works)
10. [Element-Level CSS Variable Bridge](#element-level-css-variable-bridge)
11. [Key Files Reference](#key-files-reference)

---

## Problem Statement

The overlay system needs to support user-created components that can:

- **Look different** — custom HTML layout, colors, fonts, borders (theming)
- **Display live data** — viewer count, now-playing track, follower count
- **React to events** — animate on new follower, show alert on donation
- **Use complex logic** — compute donation goal progress, AI-generated chat summaries

All while maintaining parity between the **canvas editor** (Vue, live CSS vars) and the **OBS browser source** (static HTML, resolved values).

### What CSS Variables Alone Cannot Do

The `--el-*` CSS variable bridge (implemented) handles theming and per-element style overrides. But CSS variables only control **appearance**, not **behavior**:

| Cannot Do | Why |
|-----------|-----|
| Live data binding (follower count) | Static HTML template, no data source |
| Event reactions (animate on follow) | No event system, no JS execution |
| Conditional rendering (show during raid) | No `if`/`else` in static HTML |
| Loops (render chat message list) | No `for` / dynamic lists |
| API calls (Spotify, Twitch) | No JS runtime in component |
| Custom animations (typing, counters) | No JS, no keyframe generation |

**Solution**: Components don't need their own scripting — they consume data and events from VortiDeck's existing engine.

---

## Current State

### What Exists

| System | Status | Location |
|--------|--------|----------|
| **UniversalExecutionEngine** | Production | `vortideck-core/src/services/execution/mod.rs` |
| **FlowExecutor + FlowNodeRegistry** | Production | `vortideck-core/src/services/flow/` |
| **EventBus** (tokio broadcast) | Production | `vortideck-core/src/events/` |
| **Event Bridge** (EventBus → Tauri IPC) | Production | `src-tauri/src/adapters/platform_adapters.rs` |
| **Plugin System** (WASM + Service) | Production | `vortideck-core/src/plugins/` |
| **Streaming WebSocket** (cloud relay) | Production | `vortideck-core/src/services/streaming/` |
| **Overlay WebSocket** (OBS broadcast) | Production | `src-tauri/src/overlay/` + unified server |
| **Element CSS Variable Bridge** (`--el-*`) | Implemented | `layers/desktop-streaming/composables/overlay/elementVars.ts` |
| **Element Registry** (built-in elements) | Implemented | `layers/desktop-streaming/composables/overlay/elementRegistry.ts` |

### What's Missing

| Gap | Impact |
|-----|--------|
| Streaming events don't reach EventBus | Flows can't trigger on follow/sub/raid |
| Overlay has no EventBus subscription | Components can't react to events |
| Components have no schema | Properties panel is hardcoded |
| No `StreamingNodeProvider` | Stream data/triggers not in flow palette |
| No `OverlayNodeProvider` | Can't control overlay from flows |
| No component data bindings | Can't display live data in templates |

---

## Architecture Overview

### The Engine (Already Exists)

```
┌─────────────────────────────────────────────────────┐
│  TRIGGER LAYER (UniversalAction)                    │
│  Button, Context Menu, WebSocket, Schedule, etc.    │
└──────────────────────┬──────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────┐
│  UniversalExecutionEngine                           │
│  Dispatches to the right system                     │
└──────────────────────┬──────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────┐
│  ORCHESTRATION LAYER (FlowExecutor)                 │
│  DAG of nodes, data passing, retry, error handling  │
└──────────────────────┬──────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────┐
│  NODE LAYER (FlowNodeRegistry)                      │
│  Every capability = a FlowNode                      │
│                                                     │
│  StandardNodeProvider  → 20+ built-in nodes         │
│  WasmFlowNodeProvider  → plugin nodes               │
│  McpToolProvider       → MCP tool nodes             │
│  SystemActionProvider  → system action nodes         │
│  StreamingProvider     → stream trigger/data nodes  │  ← NEW
│  OverlayProvider       → overlay action nodes       │  ← NEW
└─────────────────────────────────────────────────────┘
```

### The Pattern

Every module follows the same recipe:

1. **Implement `FlowNodeProvider`** → register nodes with `FlowNodeRegistry`
2. **Emit events to `EventBus`** → for reactivity across the system
3. **Optionally register as `Integration`** → for button/action-level execution

This is how plugins, MCP tools, and system actions already work. Streaming and overlay just need to follow the same pattern.

### Execution Hierarchy

```
UniversalAction  (trigger: "what to run")
  → Flow          (orchestration: "in what order")
    → FlowNode    (computation: "the actual work")
```

- **Actions** = how triggers (buttons, menus, webhooks) say "do this"
- **Flows** = how users chain multiple steps together (visual DAG editor)
- **Nodes** = the atomic computation units (everything is a node)
- **Plugins** create **nodes**. Streaming creates **nodes**. Overlay creates **nodes**.

---

## Phase 1: Schema-Based Component System

> Refactor the `--el-*` system into a per-component schema. Built-in elements become pre-registered components.

### Component Schema

```typescript
interface ComponentSchema {
  id: string                    // Unique identifier (e.g., 'donation-goal')
  name: string                  // Display name
  category: string              // 'alert' | 'widget' | 'display' | 'chat' | 'custom'

  // Visual template (HTML with var() and {{placeholder}} references)
  template: string

  // CSS variables this component uses, with defaults and metadata
  cssVars: Record<string, {
    default: string             // Default value (usually a var(--theme-*) reference)
    label: string               // Display label in Properties panel
    type: 'color' | 'size' | 'font' | 'string'
  }>

  // Properties panel fields — auto-generated from this array
  editableProps: Array<{
    key: string                 // Internal key
    label: string               // Display label
    type: 'color' | 'range' | 'select' | 'text' | 'number'
    cssVar: string              // Which CSS var it controls
    options?: string[]          // For select type
    min?: number                // For range/number
    max?: number
  }>

  // Default inline styles for the wrapper
  defaultStyle: Record<string, string>

  // Default dimensions
  width: number
  height: number
}
```

### Example: Built-in "New Follower" Alert as Schema

```typescript
const followerAlert: ComponentSchema = {
  id: 'follower',
  name: 'New Follower',
  category: 'alert',
  template: `
    <div style="display:flex; align-items:center; gap:16px; padding:16px;
                background:var(--c-bg); border-radius:var(--c-radius);
                border:var(--c-border);">
      <div style="font-size:36px;">❤️</div>
      <div>
        <div style="font-weight:700; color:var(--c-text);">New Follower!</div>
        <div style="color:var(--c-text); opacity:0.7;">Username just followed</div>
      </div>
    </div>
  `,
  cssVars: {
    '--c-bg':     { default: 'linear-gradient(135deg, var(--theme-accent), var(--theme-primary))', label: 'Background', type: 'color' },
    '--c-text':   { default: 'var(--theme-text)',           label: 'Text Color',   type: 'color' },
    '--c-radius': { default: 'var(--theme-border-radius)',  label: 'Radius',       type: 'size' },
    '--c-border': { default: 'var(--theme-border-thin)',    label: 'Border',       type: 'string' },
  },
  editableProps: [
    { key: 'bg',     label: 'Background',    type: 'color', cssVar: '--c-bg' },
    { key: 'text',   label: 'Text Color',    type: 'color', cssVar: '--c-text' },
    { key: 'radius', label: 'Border Radius', type: 'range', cssVar: '--c-radius', min: 0, max: 50 },
  ],
  defaultStyle: {
    borderRadius: 'var(--c-radius)',
    border: 'var(--c-border)',
    boxShadow: 'var(--c-shadow)',
  },
  width: 300,
  height: 100,
}
```

### What Changes

| Component | Change |
|-----------|--------|
| `elementRegistry.ts` | Returns `ComponentSchema` objects instead of raw element objects |
| `elementVars.ts` | `buildElementVars` → `buildComponentVars(schema, elementStyle)` — reads from component's `cssVars` instead of global `ELEMENT_VAR_DEFAULTS` |
| `UniversalElement.vue` | Injects component-specific CSS vars on wrapper (from schema) |
| `useOverlayV3Sync.ts` | Resolves component-specific vars (from schema) for OBS |
| `OverlayRightPanel.vue` | Dynamically renders controls from `editableProps` instead of hardcoded fields |

### CSS Variable Resolution (Two Rendering Paths)

**Canvas (Vue, browser resolves CSS vars natively):**
```
Component wrapper div gets:
  --c-bg: linear-gradient(135deg, var(--theme-accent), var(--theme-primary))
  --c-text: var(--theme-text)
  --c-radius: var(--theme-border-radius)

Template children use var(--c-*) → browser cascades from wrapper → resolves --theme-* from canvas container
```

**OBS (static HTML, all vars resolved to concrete values):**
```
resolveAllVars("var(--c-bg)")
  → "linear-gradient(135deg, var(--theme-accent), var(--theme-primary))"  (resolve --c-*)
  → "linear-gradient(135deg, #ff1493, #ff00ff)"                          (resolve --theme-*)
```

---

## Phase 2: Streaming Node Provider + EventBus Bridge

> Connect the streaming system to the engine. Stream events become flow triggers. Stream data becomes flow nodes.

### New EventPayload Variant

```rust
// vortideck-core/src/events/types.rs

pub enum EventPayload {
    Flow(FlowEventPayload),
    System(SystemEventPayload),
    Webhook(WebhookEventPayload),
    Relay(RelayEventPayload),
    Stream(StreamEventPayload),           // ← NEW
}

pub enum StreamEventPayload {
    Follow { account_id: u64, platform: Platform, user_name: String, user_display_name: String },
    Subscription { account_id: u64, platform: Platform, user_name: String, tier: String, message: Option<String> },
    Raid { account_id: u64, platform: Platform, raider: String, viewer_count: u32 },
    Bits { account_id: u64, platform: Platform, user_name: String, amount: u32, message: Option<String> },
    ChatMessage { account_id: u64, platform: Platform, message: ChatMessage },
    StreamOnline { account_id: u64, platform: Platform, data: StreamData },
    StreamOffline { account_id: u64, platform: Platform },
    ViewerCountUpdated { account_id: u64, platform: Platform, count: u32 },
    SessionStarted { session_id: String },
    SessionEnded { session_id: String },
}
```

### Emit Stream Events to EventBus

```rust
// In StreamingWebSocket handler, after parsing CloudEvent:
match cloud_event {
    CloudEvent::Follow { account_id, platform, data } => {
        // Existing: emit to frontend via Tauri IPC
        app_handle.emit("streaming:event", &event_json);

        // NEW: also emit to EventBus (for flows + overlay)
        event_bus.emit(EventPayload::Stream(StreamEventPayload::Follow {
            account_id,
            platform,
            user_name: data.user_name,
            user_display_name: data.user_display_name,
        }));
    }
    CloudEvent::Subscription { .. } => { /* same pattern */ }
    CloudEvent::Raid { .. } => { /* same pattern */ }
    CloudEvent::Bits { .. } => { /* same pattern */ }
    CloudEvent::ChatMessage { .. } => { /* same pattern */ }
    // ...
}
```

### StreamingNodeProvider

```rust
// vortideck-core/src/services/streaming/flow_adapter.rs (NEW)

pub struct StreamingNodeProvider {
    streaming_state: Arc<RwLock<StreamingState>>,
}

impl FlowNodeProvider for StreamingNodeProvider {
    fn get_flow_nodes(&self) -> Vec<Box<dyn FlowNode>> {
        vec![
            // TRIGGER NODES (FlowTriggerNode — start a flow on event)
            Box::new(OnFollowNode::new()),            // stream.on-follow
            Box::new(OnSubscriptionNode::new()),      // stream.on-subscription
            Box::new(OnRaidNode::new()),              // stream.on-raid
            Box::new(OnBitsNode::new()),              // stream.on-bits
            Box::new(OnChatMessageNode::new()),       // stream.on-chat-message
            Box::new(OnStreamOnlineNode::new()),      // stream.on-stream-online

            // DATA NODES (read stream state mid-flow)
            Box::new(GetViewerCountNode::new(self.streaming_state.clone())),
            Box::new(GetSessionStatsNode::new(self.streaming_state.clone())),
            Box::new(GetRecentChatNode::new(self.streaming_state.clone())),

            // ACTION NODES (do things to the stream mid-flow)
            Box::new(SendChatMessageNode::new(self.streaming_state.clone())),
            Box::new(UpdateStreamTitleNode::new(self.streaming_state.clone())),
        ]
    }
    fn provider_name(&self) -> &'static str { "StreamingProvider" }
}
```

### Event Bridge Update

```rust
// platform_adapters.rs — extend the event bridge
match event {
    EventPayload::Stream(stream_event) => {
        app_handle.emit("stream_event", serde_json::to_value(&stream_event).unwrap());
    }
    // ... existing Flow, System, Webhook, Relay handlers
}
```

### What This Unlocks

- **Flows trigger on stream events**: "When someone follows → run AI greeting → send chat message → show overlay alert"
- **Stream data available in flows**: "Every 30s → get viewer count → if > 100 → update overlay goal bar"
- **Buttons trigger stream actions**: Button → `UniversalAction::Flow` → flow with `SendChatMessageNode`

---

## Phase 3: Overlay Node Provider + Component Runtime

> Components can react to events and display live data. Overlay actions are available as flow nodes.

### OverlayNodeProvider

```rust
// vortideck-core/src/overlay/flow_adapter.rs (NEW)

pub struct OverlayNodeProvider {
    overlay_state: Arc<RwLock<OverlayState>>,
}

impl FlowNodeProvider for OverlayNodeProvider {
    fn get_flow_nodes(&self) -> Vec<Box<dyn FlowNode>> {
        vec![
            Box::new(ShowElementNode::new(self.overlay_state.clone())),
            Box::new(HideElementNode::new(self.overlay_state.clone())),
            Box::new(AnimateElementNode::new(self.overlay_state.clone())),
            Box::new(UpdateElementDataNode::new(self.overlay_state.clone())),
            Box::new(SetElementStyleNode::new(self.overlay_state.clone())),
        ]
    }
    fn provider_name(&self) -> &'static str { "OverlayProvider" }
}
```

### Component Data Bindings

Components declare what data they consume:

```typescript
interface ComponentSchema {
  // ... (cssVars, editableProps from Phase 1)

  // NEW: Bind template placeholders to flow outputs
  dataBindings?: Record<string, {
    flowId: string              // Which flow provides the data
    nodeOutput: string          // Dot-path into node output (e.g., 'track.name')
  }>

  // NEW: React to EventBus events
  eventHandlers?: Array<{
    event: string               // Event type ('stream:follow', 'flow:node_completed', etc.)
    filter?: Record<string, any> // Optional filter (e.g., { flowId: 'spotify-poll' })
    action: string              // Built-in action ('show', 'hide', 'animate', 'updateBindings')
    params?: Record<string, any> // Action parameters
  }>
}
```

### Component Runtime (Frontend)

Small reactive layer that manages per-component-instance state:

```typescript
// composables/overlay/useComponentRuntime.ts (NEW)

interface ComponentInstance {
  schema: ComponentSchema
  element: OverlayElement
  data: Record<string, any>         // Current bound data
  visible: boolean
  animationClass: string | null
}

export const useComponentRuntime = () => {
  const instances = ref<Map<string, ComponentInstance>>(new Map())

  // Listen to EventBus events via Tauri IPC
  const setupEventListeners = () => {
    // Stream events
    listen('stream_event', (event) => {
      for (const [id, instance] of instances.value) {
        for (const handler of instance.schema.eventHandlers || []) {
          if (matchesEvent(handler, event)) {
            executeAction(instance, handler.action, handler.params)
          }
        }
      }
    })

    // Flow events (for data binding updates)
    listen('flow_event', (event) => {
      if (event.payload.type === 'NodeCompleted') {
        updateDataBindings(event.payload)
      }
    })
  }

  // Render template with current data
  const renderTemplate = (instance: ComponentInstance): string => {
    let html = instance.schema.template
    for (const [key, value] of Object.entries(instance.data)) {
      html = html.replaceAll(`{{${key}}}`, String(value))
    }
    return html
  }

  // Built-in actions
  const executeAction = (instance: ComponentInstance, action: string, params?: any) => {
    switch (action) {
      case 'show': instance.visible = true; break
      case 'hide': instance.visible = false; break
      case 'animate': instance.animationClass = params?.class; break
      case 'updateBindings': /* re-render with fresh data */ break
    }
  }
}
```

### Three Data Paths Into Components

| Source | Examples | How It Arrives | Flow Engine Needed? |
|--------|----------|----------------|---------------------|
| **Stream events** | Follow, sub, raid, bits, chat | `EventPayload::Stream` → EventBus → Tauri IPC → component `eventHandlers` | No |
| **Flow outputs** | Spotify track, computed stats, AI summaries | Flow runs → `NodeCompleted` event → component `dataBindings` | Yes |
| **Session state** | Viewer count, stream duration, message count | `StreamEventPayload::ViewerCountUpdated` → component `eventHandlers` | No |

---

## Phase 4: Component Creator UI

> Users visually create and register custom components without writing code.

### Creator Workflow

1. **Design visual** — HTML template editor with live preview, CSS var declarations
2. **Pick data sources** — dropdown of available flows + their outputs, binds to `{{placeholders}}`
3. **Pick event reactions** — dropdown of EventBus event types + built-in actions
4. **Configure properties** — define what appears in the Properties panel
5. **Save & register** — component becomes available in the element palette

### User Complexity Levels

| User Level | What They Do | Plugin Required? |
|------------|-------------|-----------------|
| **Basic** | Custom HTML + CSS vars, no logic | No |
| **Intermediate** | Bind to existing flows for data, react to stream events | No |
| **Advanced** | Build custom flows (chain existing nodes) for novel logic | No |
| **Developer** | Write WASM plugin for truly custom computation | Yes |

Most users never write a plugin. They build flows using the node palette and bind components to flow outputs.

### Example: "Spotify Now Playing" — No Plugin

1. User creates a flow in flow editor:
   - `HTTP Request node` (polls Spotify API every 10s)
   - `JSON Parse node`
   - Outputs: `{ track: { name, artist, albumArt } }`

2. User creates a component in component creator:
   ```
   Template: <div><img src="{{albumArt}}"/><span>{{track}} - {{artist}}</span></div>
   CSS vars: --c-bg, --c-text (themed defaults)
   Data bindings: track → spotify-flow.track.name
   Event: flow:node_completed (spotify-flow) → updateBindings
   ```

3. Done. Flow handles API. Component handles display.

---

## Data Flow Architecture

### Complete System Diagram

```
┌──────────────────────────────────────────────────────────────────┐
│                       DATA SOURCES                               │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Cloud Relay ─────── Follow, Sub, Raid, Bits, Chat, Stream      │
│  (wss://ws.vortideck.com)                                       │
│                                                                  │
│  Flow Engine ─────── Computed data, API results, AI outputs     │
│  (FlowExecutor)                                                  │
│                                                                  │
│  Session Manager ──── Viewer count, duration, message count     │
│                                                                  │
│  Plugins ──────────── Custom data from WASM/Service plugins     │
│                                                                  │
└──────────────────────────┬───────────────────────────────────────┘
                           │
                    ALL emit to:
                           │
                           ▼
               ┌───────────────────────┐
               │      EventBus         │
               │  (tokio broadcast)    │
               │                       │
               │  EventPayload::       │
               │    Flow(...)          │
               │    Stream(...)  ← NEW │
               │    System(...)        │
               │    Webhook(...)       │
               │    Relay(...)         │
               └───────────┬───────────┘
                           │
                    Event Bridge
                  (platform_adapters.rs)
                           │
                           ▼
               ┌───────────────────────┐
               │   Tauri IPC Events    │
               │                       │
               │  "flow_event"         │
               │  "stream_event" ← NEW │
               │  "volume_changed"     │
               │  "webhook_event"      │
               │  "relay_event"        │
               └───────────┬───────────┘
                           │
                           ▼
               ┌───────────────────────┐
               │  Component Runtime    │
               │  (frontend)           │
               │                       │
               │  Matches events to:   │
               │  - eventHandlers      │
               │  - dataBindings       │
               │                       │
               │  Re-renders template  │
               │  with fresh data      │
               └───────────┬───────────┘
                           │
              ┌────────────┼────────────┐
              ▼                         ▼
    ┌──────────────┐          ┌──────────────┐
    │ Canvas (Vue) │          │  OBS Sync    │
    │              │          │              │
    │ CSS vars     │          │ resolveVars  │
    │ cascade      │          │ → concrete   │
    │ natively     │          │ HTML output  │
    └──────────────┘          └──────────────┘
```

---

## How Custom Logic Works

### No Scripting Language Needed

Components are **views**. Logic lives in **flows**. The existing engine handles everything:

```
"Show donation goal with progress bar"

Component template:
  <div>Goal: {{progress}}% (${{total}} / ${{goal}})</div>
  <div style="width:{{progress}}%; background:var(--c-fill);"></div>

Flow (user builds visually):
  [On Bits Trigger] → [Get Session Stats] → [Math: total/goal*100] → [Output: {progress, total, goal}]

Component dataBindings:
  progress → donation-tracker-flow.percentage
  total    → donation-tracker-flow.total
  goal     → donation-tracker-flow.goal
```

The component never runs logic. The flow computes. The component displays.

### When A Plugin IS Needed

Only when no existing node can do the computation:

| Scenario | Existing nodes sufficient? | Plugin? |
|----------|--------------------------|---------|
| Fetch API data | Yes (HTTP Request node) | No |
| Transform JSON | Yes (JSON Parse node) | No |
| AI text generation | Yes (AI Text Generator node) | No |
| Play sound on event | Yes (Audio Play node) | No |
| Custom game integration | No existing node | Yes (WASM plugin) |
| Proprietary data format | No existing node | Yes (WASM plugin) |

Plugins implement `FlowNode` → register via `WasmFlowNodeProvider` → appear in flow palette → users drag into flows → components bind to output. Same pattern as everything else.

---

## Element-Level CSS Variable Bridge

> Already implemented. This section documents the current system for reference.

### How It Works

Each component declares CSS variables with theme-aware defaults. The variable resolution chain:

```
Component var (--c-bg)
  → defaults to theme var (var(--theme-card-bg))
    → resolves to concrete value (#111111)
```

### Two Rendering Paths

**Canvas (browser resolves natively):**
1. `buildComponentVars(schema, element.style)` produces `--c-*` map
2. Wrapper div gets `--c-*` as inline CSS custom properties
3. Template HTML uses `var(--c-*)` — browser cascades from wrapper
4. Theme vars (`--theme-*`) set on `.canvas-container` — browser resolves the chain

**OBS (all vars resolved to concrete values):**
1. `buildComponentVars(schema, element.style)` produces `--c-*` map
2. `resolveAllVars()` does two passes:
   - Pass 1: `var(--c-bg)` → `linear-gradient(135deg, var(--theme-accent), var(--theme-primary))`
   - Pass 2: `var(--theme-accent)` → `#ff1493`, `var(--theme-primary)` → `#ff00ff`
3. Result: `linear-gradient(135deg, #ff1493, #ff00ff)` — fully concrete, no var() references

### Properties Panel Integration

Color pickers resolve `var()` chains to display actual hex values:

```typescript
resolveStyleValue(selectedElement.style?.backgroundColor, '#ffffff')
// Input:  "var(--c-bg)"
// Step 1: → "var(--theme-card-bg)" (from component vars)
// Step 2: → "#111111" (from theme vars)
// Output: "#111111" (shown in color picker)
```

When user picks a new color, it overrides the `--c-bg` var for that element instance.

### Key Files

| File | Role |
|------|------|
| `composables/overlay/elementVars.ts` | Variable definitions, `buildElementVars()` |
| `composables/overlay/elementRegistry.ts` | Element type definitions with `var(--el-*)` in customHTML |
| `composables/overlay/useOverlayTheme.ts` | Theme state, `resolveVar()` for OBS |
| `composables/overlay/useOverlayV3Sync.ts` | OBS HTML generation with two-pass var resolution |
| `components/UniversalElement.vue` | Canvas rendering with CSS var injection on wrapper |
| `components/overlay/OverlayRightPanel.vue` | Properties panel with var-aware color pickers |

---

## Key Files Reference

### Core Engine (Rust)

| File | Purpose |
|------|---------|
| `vortideck-core/src/services/execution/mod.rs` | UniversalExecutionEngine, UniversalAction enum |
| `vortideck-core/src/services/flow/executor.rs` | FlowExecutor — DAG execution |
| `vortideck-core/src/services/flow/traits.rs` | FlowNode trait, FlowNodeProvider trait, FlowTriggerNode |
| `vortideck-core/src/services/flow/registry.rs` | FlowNodeRegistry — central node catalog |
| `vortideck-core/src/services/flow/standard_nodes.rs` | 20+ built-in nodes |
| `vortideck-core/src/events/types.rs` | EventPayload enum, all event types |
| `vortideck-core/src/events/bus.rs` | EventBus (tokio broadcast) |
| `vortideck-core/src/plugins/flow_integration.rs` | WasmFlowNodeProvider — plugins as nodes |
| `vortideck-core/src/services/mcp/flow_adapter.rs` | McpToolProvider — MCP tools as nodes |
| `vortideck-core/src/services/streaming/websocket.rs` | StreamingWebSocket — cloud relay connection |
| `vortideck-core/src/services/streaming/models.rs` | Platform, ChatMessage, StreamData, CloudEvent |
| `src-tauri/src/adapters/platform_adapters.rs` | Event Bridge (EventBus → Tauri IPC) |
| `src-tauri/src/overlay/state.rs` | OverlayState, OverlayElement, OverlayDesign |
| `src-tauri/src/commands/overlay_commands.rs` | Overlay Tauri commands + WebSocket broadcast |

### Frontend (Vue/TypeScript)

| File | Purpose |
|------|---------|
| `layers/desktop-streaming/composables/overlay/elementVars.ts` | CSS variable bridge definitions |
| `layers/desktop-streaming/composables/overlay/elementRegistry.ts` | Element type registry |
| `layers/desktop-streaming/composables/overlay/useOverlayTheme.ts` | Theme state + resolveVar |
| `layers/desktop-streaming/composables/overlay/useOverlayV3Sync.ts` | OBS sync + HTML generation |
| `layers/base/components/UniversalElement.vue` | Canvas element renderer |
| `layers/desktop-streaming/components/overlay/OverlayRightPanel.vue` | Properties panel |
| `layers/desktop-streaming/pages/streaming/overlays/index.vue` | Overlay editor page |

---

## Implementation Roadmap

| Phase | Scope | Depends On |
|-------|-------|------------|
| **Phase 1** | Schema-based components, dynamic Properties panel | Nothing (start here) |
| **Phase 2** | `StreamingNodeProvider` + `EventPayload::Stream` + EventBus bridge | Phase 1 |
| **Phase 3** | `OverlayNodeProvider` + component runtime (dataBindings, eventHandlers) | Phase 2 |
| **Phase 4** | Component Creator UI (visual template editor, data source picker) | Phase 3 |

Each phase is independently shippable. Phase 1 improves the current system. Phase 2 connects streaming to the engine. Phase 3 makes components reactive. Phase 4 opens it to users.

---

## Phase 2 Implementation Detail: Streaming → EventBus Bridge

> Exact changes needed, what stays untouched, performance analysis, and chat-to-overlay data flow.

### Current Streaming Event Path (Before)

```
Cloud relay (wss://ws.vortideck.com)
  ↓ CloudEvent JSON
StreamingWebSocket::handle_message()
  ├── ChatMessage → MessageStore.store_message() (SQLite)
  ├── ChatMessage → check session commands → UniversalExecutionEngine
  ├── StreamOnline → SessionManager.start_session() (auto-activate)
  ├── StreamOffline → SessionManager.end_session() (auto-end)
  └── ALL events → event_callback(event)
                      ↓
                 streaming_connection.rs:
                   app_handle.emit("streaming:event", &event)
                      ↓
                 Frontend: chat page, session UI
```

EventBus is a **completely separate system** — only used by FlowExecutor, SystemControl, Webhooks, and Relay. Streaming doesn't touch it.

### New Streaming Event Path (After)

```
Cloud relay (wss://ws.vortideck.com)
  ↓ CloudEvent JSON
StreamingWebSocket::handle_message()
  ├── ChatMessage → MessageStore.store_message() (SQLite)          ← UNCHANGED
  ├── ChatMessage → check session commands → ExecutionEngine       ← UNCHANGED
  ├── StreamOnline → SessionManager.start_session()                ← UNCHANGED
  ├── StreamOffline → SessionManager.end_session()                 ← UNCHANGED
  ├── ALL events → event_callback(event)                           ← UNCHANGED
  │                   ↓
  │              app_handle.emit("streaming:event", &event)        ← UNCHANGED
  │                   ↓
  │              Frontend: chat page, session UI                   ← UNCHANGED
  │
  └── ALL events → EventBus.emit(Stream(payload))                  ← NEW (additive)
                      ↓
                 spawn_event_bridge (platform_adapters.rs)
                      ↓
                 app_handle.emit("stream_event", payload)           ← NEW
                      ↓
                 Frontend: overlay components, flow triggers         ← NEW
```

**Both paths coexist.** The existing `streaming:event` carries the full `CloudEvent` (raw, all fields). The new `stream_event` carries a structured `StreamEventPayload` (typed, clean). Different consumers, different needs. Zero breaking changes.

### Exact Rust Changes (4 Files, ~60 Lines Total)

#### Change 1: `vortideck-core/src/events/types.rs`

Add one variant to `EventPayload` + define `StreamEventPayload`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "category", content = "payload", rename_all = "camelCase")]
pub enum EventPayload {
    Flow(FlowEventPayload),
    System(SystemEventPayload),
    Webhook(WebhookEventPayload),
    Relay(RelayEventPayload),
    Stream(StreamEventPayload),  // ← NEW: one variant added
}

/// Stream event types from platform relay (Twitch, YouTube, Kick, TikTok)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StreamEventPayload {
    Follow {
        #[serde(rename = "accountId")]
        account_id: u64,
        platform: String,
        #[serde(rename = "userName")]
        user_name: String,
        #[serde(rename = "userDisplayName")]
        user_display_name: String,
    },
    Subscription {
        #[serde(rename = "accountId")]
        account_id: u64,
        platform: String,
        #[serde(rename = "userName")]
        user_name: String,
        tier: Option<String>,
        message: Option<String>,
    },
    Raid {
        #[serde(rename = "accountId")]
        account_id: u64,
        platform: String,
        raider: String,
        #[serde(rename = "viewerCount")]
        viewer_count: Option<u32>,
    },
    Bits {
        #[serde(rename = "accountId")]
        account_id: u64,
        platform: String,
        #[serde(rename = "userName")]
        user_name: String,
        amount: u32,
        message: Option<String>,
    },
    ChatMessage {
        #[serde(rename = "accountId")]
        account_id: u64,
        platform: String,
        #[serde(rename = "userName")]
        user_name: String,
        #[serde(rename = "userDisplayName")]
        user_display_name: String,
        message: String,
        #[serde(rename = "isModerator")]
        is_moderator: bool,
        #[serde(rename = "isSubscriber")]
        is_subscriber: bool,
    },
    StreamOnline {
        #[serde(rename = "accountId")]
        account_id: u64,
        platform: String,
        title: Option<String>,
    },
    StreamOffline {
        #[serde(rename = "accountId")]
        account_id: u64,
        platform: String,
    },
    ViewerCountUpdated {
        #[serde(rename = "accountId")]
        account_id: u64,
        platform: String,
        count: u32,
    },
}
```

#### Change 2: `vortideck-core/src/services/streaming/websocket.rs`

Add `event_bus` field + builder + emit after existing callback:

```rust
pub struct StreamingWebSocket {
    // ... all existing fields unchanged ...
    event_bus: Option<Arc<crate::events::EventBus>>,  // ← NEW field
}

impl StreamingWebSocket {
    pub fn new(/* existing params */) -> Self {
        Self {
            // ... all existing field inits unchanged ...
            event_bus: None,  // ← NEW
        }
    }

    /// Set the event bus for broadcasting stream events to the engine
    pub fn with_event_bus(mut self, event_bus: Option<Arc<crate::events::EventBus>>) -> Self {
        self.event_bus = event_bus;
        self
    }
}
```

In `handle_message()`, after the existing `event_callback` invocation (line ~836):

```rust
// EXISTING (unchanged):
if let Some(callback) = event_callback {
    callback(event.clone());
}

// NEW: Broadcast to EventBus for flows + overlay
if let Some(bus) = event_bus {
    if let Some(stream_payload) = Self::cloud_event_to_stream_payload(&event) {
        let _ = bus.emit(crate::events::EventPayload::Stream(stream_payload));
    }
}
```

Converter function (simple match on CloudEvent variants):

```rust
fn cloud_event_to_stream_payload(event: &CloudEvent) -> Option<StreamEventPayload> {
    match event {
        CloudEvent::Follow { account_id, platform, data } => Some(StreamEventPayload::Follow {
            account_id: *account_id,
            platform: platform.to_string(),
            user_name: data.get("user_name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            user_display_name: data.get("user_display_name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        }),
        CloudEvent::ChatMessage { account_id, platform, data } => Some(StreamEventPayload::ChatMessage {
            account_id: *account_id,
            platform: platform.to_string(),
            user_name: data.user_name.clone(),
            user_display_name: data.user_display_name.clone(),
            message: data.message.clone(),
            is_moderator: data.is_moderator,
            is_subscriber: data.is_subscriber,
        }),
        CloudEvent::Subscription { account_id, platform, data } => Some(StreamEventPayload::Subscription {
            account_id: *account_id,
            platform: platform.to_string(),
            user_name: data.get("user_name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            tier: data.get("tier").and_then(|v| v.as_str()).map(String::from),
            message: data.get("message").and_then(|v| v.as_str()).map(String::from),
        }),
        CloudEvent::Raid { account_id, platform, data } => Some(StreamEventPayload::Raid {
            account_id: *account_id,
            platform: platform.to_string(),
            raider: data.get("from_broadcaster_user_name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            viewer_count: data.get("viewers").and_then(|v| v.as_u64()).map(|v| v as u32),
        }),
        CloudEvent::Bits { account_id, platform, data } => Some(StreamEventPayload::Bits {
            account_id: *account_id,
            platform: platform.to_string(),
            user_name: data.get("user_name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            amount: data.get("bits_used").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            message: data.get("message").and_then(|v| v.as_str()).map(String::from),
        }),
        CloudEvent::StreamOnline { account_id, platform, data } => Some(StreamEventPayload::StreamOnline {
            account_id: *account_id,
            platform: platform.to_string(),
            title: Some(data.title.clone()),
        }),
        CloudEvent::StreamOffline { account_id, platform } => Some(StreamEventPayload::StreamOffline {
            account_id: *account_id,
            platform: platform.to_string(),
        }),
        CloudEvent::ChannelUpdate { account_id, platform, data } => {
            data.viewer_count.map(|count| StreamEventPayload::ViewerCountUpdated {
                account_id: *account_id,
                platform: platform.to_string(),
                count,
            })
        },
        // Ping, Error, PlatformStatus, SendChatResult — no overlay relevance
        _ => None,
    }
}
```

#### Change 3: `src-tauri/src/streaming_connection.rs`

Pass EventBus when creating WebSocket (one line):

```rust
// After getting execution_engine (line ~97):
let event_bus = app_handle.try_state::<Arc<vortideck_core::events::EventBus>>()
    .map(|bus| bus.inner().clone());

let mut ws = StreamingWebSocket::new(
    streaming_state.message_store.clone(),
    streaming_state.session_manager.clone(),
    streaming_ws_url.clone(),
    access_token.clone(),
)
.with_database(core_db)
.with_execution_engine(execution_engine)
.with_event_bus(event_bus);  // ← one line added
```

#### Change 4: `src-tauri/src/adapters/platform_adapters.rs`

Add one match arm in `spawn_event_bridge()` (after line ~503):

```rust
// Stream events → "stream_event" IPC
EventPayload::Stream(stream_event) => {
    if let Ok(json_data) = serde_json::to_value(&stream_event) {
        if let Err(e) = app_handle.emit("stream_event", json_data) {
            log::warn!("Failed to forward stream event to frontend: {}", e);
        }
    }
}
```

### What Does NOT Change (Zero Refactoring)

| Component | File | Changes? | Why |
|-----------|------|----------|-----|
| Chat message storage | `websocket.rs` (line 586-729) | **No** | EventBus emit is after all storage logic |
| Session command execution | `websocket.rs` (line 604-723) | **No** | Command checks happen before EventBus emit |
| Permission checking | `websocket.rs` (line 24-75) | **No** | Not touched at all |
| Cooldown tracking | `websocket.rs` (line 635-650) | **No** | Not touched at all |
| Session auto-activate | `websocket.rs` (line 732-803) | **No** | StreamOnline handler unchanged |
| Session auto-end | `websocket.rs` (line 807-833) | **No** | StreamOffline handler unchanged |
| `event_callback` path | `websocket.rs` (line 836-842) | **No** | Still invoked, still emits `streaming:event` |
| Chat page frontend | `pages/streaming/chat/index.vue` | **No** | Still listens to `streaming:event` |
| Session manager UI | `pages/streaming/index.vue` | **No** | Still listens to `streaming:event` |
| WebSocket reconnection | `websocket.rs` (line 338-488) | **No** | Same exponential backoff |
| Token refresh on auth error | `websocket.rs` (line 430-474) | **No** | Same OAuth flow |
| `streaming_connection.rs` setup | Lines 1-115 | **+2 lines** | Just pass EventBus, rest unchanged |

### Performance Analysis

| Metric | Impact |
|--------|--------|
| **Chat message rate** | Busy streams: 100+ msgs/sec |
| **EventBus implementation** | `tokio::broadcast` — lock-free, O(1) send |
| **Buffer size** | 10,000 events default (100 seconds at peak rate) |
| **Clone cost per event** | `CloudEvent` already cloned for `event_callback`. `StreamEventPayload` is smaller (only relevant fields, no raw `Value`) |
| **Serialization** | `serde_json::to_value` in event bridge — same as all other event types |
| **Memory** | `StreamEventPayload::ChatMessage` is ~200 bytes. At 100/sec × 10K buffer = ~2MB worst case |
| **Subscribers** | Overlay components only process events matching their `eventHandlers`. No work for irrelevant events |
| **OBS sync throttle** | Component runtime can batch chat updates (e.g., re-render every 100ms, not per-message) |

### Chat Messages → Overlay Component (End-to-End)

```
1. Cloud relay sends ChatMessage
   ↓
2. StreamingWebSocket::handle_message()
   ├── Stores in SQLite (existing)
   ├── Checks session commands (existing)
   ├── event_callback → "streaming:event" (existing, chat page)
   └── EventBus.emit(Stream::ChatMessage { user, message, ... })  ← NEW
       ↓
3. spawn_event_bridge receives it
   ↓
4. app_handle.emit("stream_event", { type: "chatMessage", userName: "...", message: "..." })
   ↓
5. Component Runtime (Phase 3, frontend) receives Tauri IPC event
   ↓
6. Matches against component's eventHandlers:
   eventHandlers: [
     { event: "stream:chat_message", action: "appendToBuffer", params: { maxItems: 30 } }
   ]
   ↓
7. appendToBuffer action:
   instance.buffer.push({
     username: event.userDisplayName,
     message: event.message,
     platform: event.platform,
     isMod: event.isModerator,
   })
   if (instance.buffer.length > 30) instance.buffer.shift()  // Ring buffer
   ↓
8. Re-render template with buffer data:
   Template: <div class="chat-container">
     {{#each messages}}
       <div style="color:var(--c-text);">
         <span style="color:var(--c-username); font-weight:bold;">{{username}}</span>
         <span>{{message}}</span>
       </div>
     {{/each}}
   </div>
   ↓
9. Canvas: Vue reactivity updates v-html, CSS vars cascade
   OBS: useOverlayV3Sync resolves vars → sends HTML via WebSocket
```

**Note on template loops:** The `{{#each}}` syntax requires the component runtime to support iteration in template rendering. This is Phase 3 work — a simple template engine that handles `{{#each items}}...{{/each}}` and `{{#if condition}}...{{/if}}` blocks. Not a full scripting language — just two control structures.

### Dual IPC Events: Migration Path

During transition, two IPC events carry overlapping stream data:

| IPC Event | Payload | Consumer | Status |
|-----------|---------|----------|--------|
| `streaming:event` | Full `CloudEvent` (raw, all fields) | Chat page, session UI | Existing, keep |
| `stream_event` | Structured `StreamEventPayload` (typed, clean) | Overlay components, flow triggers | New |

**Long-term cleanup (optional, not blocking):**
- Migrate chat page to listen on `stream_event` instead of `streaming:event`
- Remove `event_callback` from `StreamingWebSocket`
- Remove `streaming:event` IPC entirely
- All consumers use EventBus path

This is a future cleanup, not a requirement. Both paths can coexist indefinitely with negligible overhead.

---

## Direct vs. Flow-Mediated Event Paths

VortiDeck supports two pipelines for routing events to overlays, giving users both simple defaults and advanced scripting power:

### 1. Direct Event Path (Zero-Setup, Low-Latency)
For simple elements that do not require processing (e.g., displaying raw chat messages, or a simple "New Follower" name alert):
* **Flow**: The element's schema maps a template placeholder (e.g. `{{user_name}}`) directly to a platform event payload key (e.g. `Twitch Follower Event`).
* **Under the Hood**: The local WebSocket server catches `EventPayload::Stream` from the EventBus, serializes it, and sends it directly to the OBS client. The overlay runtime processes the DOM update immediately without starting any Flow engine execution.

### 2. Flow-Mediated Event Path (Advanced Logic & Filtering)
For elements requiring translation, calculations, moderation, or conditional logic (e.g. AI-based chat filters, donation goal calculation, or coordinate formatting):
* **Flow**: The platform event triggers a specific Flow (e.g. starting with `OnChatMessageNode` or `OnFollowNode`). The event data travels through intermediate nodes (like an `AI Moderation` node or a `Math` node).
* **Under the Hood**: The Flow runs, calculates the result, and publishes a `FlowEventPayload::FlowCompleted` event to the EventBus. The overlay client listens for this completion and renders the clean, calculated output.

### UX/UI Integration
In the Overlay Editor's Properties Panel, under the **Data Tab**, users are presented with a simple **Data Source Selector** dropdown:
* `Raw Event Streams (Direct)`: E.g., `Twitch Chat (Raw)`, `Kick Subscriptions (Raw)`. Default option requiring zero setup.
* `Flow Outputs (Mediated)`: E.g., `Flow: Clean Chat Stream`, `Flow: Spotify Tracker`. Select the Flow and the corresponding output key (e.g. `clean_message`). 
* If a Flow output is selected, the editor offers a shortcut `[ Create Interceptor Flow + ]` button to auto-scaffold a flow template starting with that platform's trigger and ending with a Flow Output Node.

---

## Universal Flow Output Bindings vs. Specialized Nodes

When routing flow computations to overlays, we balance loose coupling with explicit commands:

| Dimension | Option A: Universal Flow Output Bindings (Recommended for Data) | Option B: Specialized Overlay Nodes (Recommended for Actions) |
|---|---|---|
| **Concept** | Flow does not know about the overlay. It completes and publishes its result payload to the EventBus. The overlay component binds to the flow output. | Flow contains explicit nodes like `ShowOverlayElement` or `HideOverlayElement` pointing to specific overlay IDs. |
| **Pros** | - **Decoupled**: Flow is highly reusable (same output can update Stream Deck, file log, and overlay).<br>- **Canvas/OBS Parity**: Vue editor canvas and OBS websocket client both consume the same `FlowCompleted` events.<br>- **No Clogging**: Keeps the flow diagram clean from "Update text" clutter. | - **Intuitive**: Very easy to visualize commands like "Wait 5s then hide alert".<br>- **Direct Control**: Flow can trigger complex visual timelines on the overlay. |
| **Cons** | Requires the user to declare data mappings inside the element creator/editor. | Clogs the flow diagram if used for simple text/data updates. Hard to reuse the same data on other hardware. |

### Architectural Decision: Hybrid Approach
VortiDeck implements a **hybrid approach**:
1. **For Data (Text, Numbers, Song Titles, Goals)**: Use **Universal Flow Output Bindings**. The overlay component listens to `FlowCompleted` events and extracts the data.
2. **For Actions (Animations, Transitions, Show/Hide, Audio Triggers)**: Use **Specialized Flow Nodes** (e.g., `ShowElementNode`, `AnimateElementNode`). These act as active commands that push side-effects down to the overlay from the flow pipeline.

