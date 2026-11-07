# Overlay Engine Architecture

> Historical proposal, not the current runtime contract. The parallel frontend
> template/action managers described here were removed on 3 October 2026.
> The editor and debug widgets now use `useOverlayV3Sync` and native
> `OverlayState`; OBS canvas behavior and remaining acceptance are tracked in
> F11. Do not rebuild the old managers
> from this proposal.

## What Already Exists

| System | Location | Status |
|--------|----------|--------|
| **Plugin Registry** | `composables/plugins/usePluginSystem.ts` | Working — `registerTheme()`, `registerElement()`, `registerEffect()`, `registerAction()`, `registerDataSource()` hooks |
| **Theme Composable** | `composables/overlay/useOverlayTheme.ts` | Working — single source of truth, CSS vars, `resolveVar()` for OBS |
| **Theme Registry** | `composables/overlay/themeRegistry.ts` | Working — catalog of 9 themes across 5 categories |
| **Element Registry** | `composables/overlay/elementRegistry.ts` | Working — 22 element types, all themed with `var(--theme-*)` |
| **OBS Sync** | `composables/overlay/useOverlayV3Sync.ts` | Working — batch sync, position broadcast, HTML generation with resolved vars |
| **Flow Executor** | `vortideck-core/src/services/flow/executor.rs` | Working — Rust-side flow execution with type safety and error handling |
| **Event Bus** | `composables/useEventBus.ts` | Working — pub/sub for cross-component communication |
| **Widget Registry** | Plugin system registers widgets via `registerElement()` | Partial — elements exist, but no live-data binding |
| **Overlay Actions** | Plugin system registers via `registerAction()` | Partial — actions registered but no overlay-specific dispatcher |
| **Dynamic Effects** | Plugin system registers via `registerEffect()` | Partial — effects registered but no overlay effect renderer |
| **Data Subscriptions** | Plugin system registers via `registerDataSource()` | Partial — sources registered but no overlay subscription loop |
| **Modular Elements** | `elementRegistry.ts` modular type with template + bindings | Working — template interpolation with mock data |

## Architecture Diagram

```
┌─────────────────────────────────────────────────┐
│                  Plugin System                   │
│  registerTheme() · registerElement()             │
│  registerEffect() · registerAction()             │
│  registerDataSource()                            │
└──────────────────────┬──────────────────────────┘
                       │ registers into
                       ▼
┌─────────────────────────────────────────────────┐
│              Overlay Engine (missing)             │
│                                                  │
│  ┌──────────┐ ┌──────────┐ ┌──────────────────┐ │
│  │  Theme    │ │ Element  │ │ Data Subscription│ │
│  │  Manager  │ │ Manager  │ │ Manager          │ │
│  └────┬─────┘ └────┬─────┘ └────────┬─────────┘ │
│       │            │                │            │
│  ┌────┴─────┐ ┌────┴─────┐ ┌───────┴──────────┐ │
│  │  Effect  │ │  Action  │ │ Event            │ │
│  │  Renderer│ │ Dispatch │ │ Router           │ │
│  └──────────┘ └──────────┘ └──────────────────┘ │
└──────────────────────┬──────────────────────────┘
                       │ feeds
                       ▼
┌─────────────────────────────────────────────────┐
│                   Consumers                      │
│                                                  │
│  Canvas Editor  ·  OBS Sync  ·  Preview          │
│  Flow Nodes     ·  Marketplace                   │
└─────────────────────────────────────────────────┘
```

## What's Actually Missing

The individual registries and composables all exist. What's missing is **one unified composable** that ties them together:

```
useOverlayEngine()
├── Connects plugin-registered data sources → element bindings
├── Dispatches plugin-registered actions → element state changes
├── Triggers plugin-registered effects → CSS/animation injection
├── Routes events (EventBus) → overlay reactions
└── Manages element lifecycle (add/remove/update → OBS sync)
```

Currently, `index.vue` manually wires everything together. The overlay engine would replace that manual wiring with a declarative system where plugins just register capabilities and elements just declare what they need.

## How Plugins Fit

A plugin that adds a "Twitch Chat" overlay widget would:

```ts
export default defineVortiDeckPlugin({
  name: 'twitch-chat-overlay',

  setup({ registerElement, registerDataSource, registerEffect }) {
    // 1. Register a data source that polls Twitch chat
    registerDataSource('twitch-chat', {
      type: 'websocket',
      connect: (config) => twitchChatStream(config.channel),
      schema: { messages: 'ChatMessage[]' },
    })

    // 2. Register the overlay element
    registerElement({
      id: 'twitch-chat-widget',
      name: 'Twitch Chat',
      category: 'widgets',
      defaultSize: { width: 400, height: 600 },
      template: `<div class="chat-container">...</div>`,
      bindings: { chat: 'twitch-chat' }, // connects to data source above
    })

    // 3. Register effects triggered by chat events
    registerEffect('chat-highlight', {
      trigger: 'twitch-chat:new-message',
      condition: (msg) => msg.isHighlighted,
      animation: 'pulse',
      duration: 2000,
    })
  },
})
```

The overlay engine would automatically:
- Start the `twitch-chat` data source when the element is on canvas
- Pipe data into the element's template bindings
- Fire effects when conditions match
- Sync all HTML changes to OBS

## Phased Implementation Plan

### Phase 1: Data Binding (connects data sources → elements)
- Create `useOverlayEngine.ts` composable
- Subscribe to registered data sources when elements are added to canvas
- Replace mock data in modular elements with live data pipe
- Auto-sync updated HTML to OBS when data changes

### Phase 2: Action Dispatch (overlay-specific actions)
- Route actions from flow executor / hotkeys / events to overlay elements
- Actions like: show/hide element, trigger animation, swap theme, update text
- Connect to existing `useOverlayV3Sync` for OBS updates

### Phase 3: Effect System (visual effects on triggers)
- Render effects registered by plugins (confetti, explosions, alerts)
- CSS animation injection into OBS HTML
- Duration management and cleanup
- Event-triggered (new sub, raid, donation, etc.)

### Phase 4: Marketplace Integration
- Download themes/elements/effects from backend
- Install as local plugins
- Theme preview in ThemeBrowserModal
- Version management and updates

## File Structure (Proposed)

```
composables/overlay/
├── useOverlayTheme.ts        ← exists (theme state + CSS vars)
├── useOverlayV3Sync.ts       ← exists (OBS sync)
├── useOverlayEngine.ts       ← NEW (orchestrator)
├── useOverlayDataBinding.ts  ← NEW (data source → element pipe)
├── useOverlayEffects.ts      ← NEW (effect renderer)
├── useOverlayActions.ts      ← NEW (action dispatcher)
├── themeRegistry.ts           ← exists (theme catalog)
└── elementRegistry.ts         ← exists (element catalog)
```

## Key Principle

**The overlay engine is a thin orchestration layer, not a rewrite.** Everything it needs already exists — it just connects the dots between plugin registrations and the canvas/OBS consumers.
