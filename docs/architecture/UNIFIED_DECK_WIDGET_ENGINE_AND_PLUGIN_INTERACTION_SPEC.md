# Unified Deck Widget Engine & Plugin Interaction Architecture Specification

**Status:** Approved Architectural Blueprint  
**Target:** VortiDeck OS Core Deck Engine (Desktop, Mobile & Web Companion)  
**Version:** 2.2.0

---

## 1. Executive Summary

This document specifies the architectural convergence of VortiDeck's Deck Simulator V1 (hardened, IPC-connected, SQLite-persisted) and the Deck Simulator V2 Prototype (multi-span 2D grid, multi-directional resize, interactive widgets). 

Rather than maintaining separate simulator pages, VortiDeck is adopting a **Unified Deck Engine** with a standardized **Plugin Widget SDK**. This architecture supports diverse interaction models—from simple 1×1 macro buttons to complex multi-zone media controllers, audio faders, and passive telemetry feeds.

### 1.1 Implementation ledger (2026-08-30)

The production V1 simulator is now the migration base. `/deck/simulator-v2` redirects to it so the prototype cannot become a second production engine.

Implemented:

* canonical widget definitions, instances, ports, policies, intents, layouts, runtime state, snapshots, and deltas;
* `WidgetHost`, renderer registry, built-in widgets, sandbox bridge, and V1 action/rotary compatibility renderers;
* deterministic multi-span placement, collision checks, history, transactional persistence, optimistic revisions, rollback, and V1-to-widget upgrade;
* a schema-driven widget inspector with action search and per-port policy configuration;
* one persisted-binding execution boundary for desktop, direct mobile, and relay mobile—clients send intents, never executable actions;
* host-owned run-once, repeat lifecycle, toggle branches, queue dispatch, continuous throttling, value transforms, replay rejection, plan-limit validation, package grant/re-consent validation, and intent acknowledgements;
* desktop/mobile widget snapshots, self-describing definitions, document deltas, missed-revision snapshot recovery, ordered authoritative state patches, and accepted/rejected intent acknowledgements;
* live runtime publishers for `system.volume`, integration status, and system telemetry, with execution status/error/duration reconciliation for every bound action;
* bounded repeat, queue-depth enforcement, cancellation on release, app background, disconnect, item removal, and package revocation;
* persisted toggle state, explicit on/off branches, continuous input ranges/curves, repeat limits, queue limits, binding removal, and schema/default validation in the inspector;
* direct grid move/resize previews with constraint and collision validation followed by one transactional history/save commit;
* package revocation now removes the grant, revokes the live permission checker, unloads the runtime, and cancels active widget work;
* direct and relay mobile transport now fail closed: canonical HMAC signatures and nonces are required after the local AEAD-authenticated pairing bootstrap, replay/tampering is rejected in both directions, direct broadcasts are signed per paired client, and plaintext mobile authentication/relay selection is no longer sent; relay application payloads remain readable by the gateway until protocol-v3 encryption is implemented;
* V1 decks remain unchanged until explicitly upgraded; migrated items retain `legacyButtonId`, the mature renderers, and a reversible legacy fallback.
* portable deck-view contracts, bounded Rust/TypeScript validators, and the shared trusted declarative renderer now support safe plugin companion views without copying plugin code to mobile;
* package deck surfaces can declare `portableView`, `publicConfigPaths`, and `publicStatePaths`; community packages remain limited to sandboxed desktop content and/or the portable renderer;
* widget definitions now carry canonical content hashes, snapshot and document-delta paths transfer plugin definitions, and mobile verifies definitions before storing them in a per-paired-desktop persistent bounded cache;
* remote snapshots/deltas use a public instance projection with `availablePorts`; executable bindings/actions are retained in local editor documents but removed from companion traffic;
* plugin configuration and runtime state sent to companions are reduced to explicit manifest allowlists, while built-in definitions retain their reviewed public contracts; remote error details are sanitized.
* package grant/re-consent broadcasts hashed definition upserts; revocation broadcasts definition removal and purges the paired mobile cache, so an unsupported placeholder replaces revoked presentation without waiting for reconnect;
* desktop and mobile now select the matching landscape/portrait placement locally instead of using the inverted desktop selection or relying solely on the snapshot's temporary default.
* the desktop editor supports a persisted deck-level `Adaptive Layout` policy plus explicit independent layouts: Adaptive edits derive and validate the opposite orientation, Independent edits preserve it, and `Sync current → other` performs a one-shot validated overwrite without changing the policy;
* legacy button image fit is an explicit per-button `cover | contain` database field, defaults to `cover`, and is honored by both the desktop adapter and the mobile renderer;
* protocol-v2 cache inventory and immutable object messages now transfer only missing definition/asset hashes over the existing signed, replay-protected direct/relay transport;
* every v2 object response is preceded by an authenticated active-deck manifest; mobile binds definition hashes to definition IDs and asset hashes to exact declarations before accepting bytes;
* desktop resolves portable assets only inside the owning installed package, then verifies the manifest hash, byte length, media type, and raster signature before transfer;
* mobile stores definitions and raster assets in a bounded per-paired-desktop cache, re-verifies bytes before commit/use, garbage-collects unreachable assets on definition update/revoke, and resolves `{ asset: id }` only through the authenticated definition manifest;
* executable/vector asset ambiguity is fail-closed: PNG, JPEG, and WebP are supported; SVG remains rejected until desktop rasterization or a production sanitizer is implemented.

Deliberately retained compatibility paths:

* `buttons`, `Dynamic/Button.vue`, `Buttons/Rotary.vue`, and the old mobile button handlers remain the source for decks that have not been upgraded;
* the compatibility renderers wrap those mature controls after upgrade while normalized intents replace their transport/scheduling ownership;
* V2 prototype components and page source remain temporarily for visual reference, but route middleware redirects `/deck/simulator-v2` to the unified production simulator.

Remaining rollout work—not architectural gaps in the host contract—is listed in Phase 9. A real OS/media-session provider is still required before `media.playback` can publish authoritative now-playing state; none exists in the current backend. Compatibility data/renderers must remain until device E2E, migrated-deck recovery, and one-release rollback gates pass. They are migration infrastructure, not a competing engine.

Important remaining boundary: the portable schema, renderer, definition hashes, redacted projection, definition/asset caches, authenticated manifests, missing-hash inventory negotiation, lifecycle refresh, and local orientation selection now exist. Atomic manifest-before-snapshot activation and negotiated retirement of redundant v1 definition payloads, shared cross-language conformance fixtures/fuzzing, optional desktop SVG rasterization, and device-level orientation/reconnect E2E still remain. Unsupported or invalid definitions continue to reach the safe unavailable-widget placeholder.

Verification completed for this implementation pass:

* the full desktop Rust host compiles with `cargo check --locked`;
* all 52 repository frontend tests pass, including portable-schema/asset validation and integrity, widget contracts, and mobile encryption/signature/replay policy;
* all 98 core unit tests, all 21 previously verified core integration tests, the compiled core doctest, all 33 Tauri library tests, and all 25 desktop-host binary tests pass;
* the mobile Nuxt production build succeeds;
* the core crate and Tauri library compile; Rust coverage includes manifest portable-view validation, remote action/config redaction, and public runtime-state projection (the Windows `aws-lc-sys` build required a short `CARGO_TARGET_DIR` to avoid the host path-length failure);
* `git diff --check` passes;
* whole-workspace desktop typecheck remains blocked by the repository's pre-existing TypeScript backlog outside the unified widget paths;
* mobile typecheck currently scans generated Android `src-tauri/target/**` binary/codegen assets and fails there before producing a meaningful source-only result.

---

## 2. V1 vs. V2 Architectural Comparison & Gap Analysis

```
┌────────────────────────────────────────────────────────────────────────┐
│                          CURRENT STATE                                 │
├───────────────────────────────────┬────────────────────────────────────┤
│   Simulator V1 (Production Core)  │     Simulator V2 (Demo Sandbox)    │
├───────────────────────────────────┼────────────────────────────────────┤
│ ✔ Deep Rust/Tauri IPC Integration │ ✖ In-Memory Mock (`saveDeck` logs) │
│ ✔ SQLite Database Persistence     │ ✖ Disconnected from SQLite/Backend │
│ ✔ State-Managed (`useDeckStore`)  │ ✖ Isolated Local Reactive State    │
│ ✔ Theme-Compliant HSL Tokens      │ ✖ Hardcoded Tailwind Slate Classes │
│ ✔ Boundary-Aware Context Menu     │ ✖ Hardcoded Fixed Context Menu     │
│ ✔ Live WebSocket Companion Sync   │ ✖ No IPC / Mobile Streaming Sync   │
│ ✖ Rigid 1×1 Grid Slot Restriction │ ✔ Multi-Span 2D Grid (e.g. 2×2,1×2)│
│ ✖ No Direct Widget Corner Resize  │ ✔ 4-Corner Multi-Directional Resize│
└───────────────────────────────────┴────────────────────────────────────┘
```

---

## 3. The 4 Core Plugin & Widget Interaction Archetypes

Deck elements are categorized into four distinct interaction archetypes. The Deck Engine handles user input (clicks, drags, holds, and context menus) based on the widget's declared archetype:

```
┌─────────────────────────┐  ┌─────────────────────────┐
│ 1. Atomic Button (1×1)  │  │ 2. Compound Multi-Zone  │
│ ┌─────────────────────┐ │  │ ┌──────┐ ┌──────┐ ┌───┐ │
│ │   Entire Tile is    │ │  │ │ Prev │ │ Play │ │Next││
│ │   One Big Trigger   │ │  │ └──────┘ └──────┘ └───┘ │
│ └─────────────────────┘ │  │     Spotify Bar (2×1)   │
└─────────────────────────┘  └─────────────────────────┘
┌─────────────────────────┐  ┌─────────────────────────┐
│ 3. Continuous / Fader   │  │ 4. Telemetry / Passive  │
│ ┌─────────────────────┐ │  │ ┌─────────────────────┐ │
│ │  ▲ Vertical Drag    │ │  │ │  CPU 42% | RAM 68%  │ │
│ │  █ Value Scrubbing  │ │  │ │  Live Twitch Chat   │ │
│ └─────────────────────┘ │  │ └─────────────────────┘ │
└─────────────────────────┘  └─────────────────────────┘
```

### Archetype 1: Atomic Clickable Action Widgets
* **Target Elements**: Macros, OBS Scene Switchers, App Launchers, Flow Triggers, Hotkeys.
* **Surface Model**: The entire widget bounding box is a single interactive touch target.
* **Supported Trigger Modes**:
  - `run_once`: Executes the action pipeline upon mouse down / tap.
  - `switch` (Toggle): Maintains a binary ON/OFF state with independent icons, labels, and active colors.
  - `repeat_while_pressing`: Rapidly fires the action at a configurable interval while held down.
  - `queue`: Appends triggers to a synchronized execution queue.
  - `long_press`: Secondary alternate trigger (e.g., tap to mute, hold 500ms to open audio settings).

### Archetype 2: Compound / Multi-Zone Interactive Widgets
* **Target Elements**: Spotify Media Player (2×1), OBS 4-Scene Matrix (2×2), Twitch Streamer Polls (2×2), Soundboard Mini-Folder (2×2).
* **Surface Model**: A single multi-span widget container enclosing multiple independently clickable sub-zones.
* **Input Routing Rules**:
  - **Edit Mode**: Clicking anywhere on the widget bounding box selects and drags the parent widget as a whole.
  - **Interactive Mode**: Mouse/touch events pass directly through to internal interactive elements (`@click.stop`), allowing buttons, switches, and tabs inside the widget to function independently.

### Archetype 3: Continuous / Value-Scrubbing Widgets
* **Target Elements**: Vertical Audio Faders (1×2), Horizontal Sliders (2×1), Rotary Encoders (1×1 / 2×2), Color Temperature Pickers.
* **Surface Model**: Continuous drag-tracking across a single or radial axis.
* **Input Routing Rules**:
  - Emits real-time throttling events (`onInput(val)`) for UI feedback and debounced commitments (`onChange(val)`) over Tauri IPC to OS audio APIs, OBS Websocket volume, or lighting hardware.
  - Tap-to-Mute: Tapping the track header or track gutter toggles mute state without shifting the fader position.

### Archetype 4: Telemetry & Live Streaming Display Widgets
* **Target Elements**: CPU/GPU Performance Gauges, Real-Time Twitch Chat, OBS Live Program Video Feed, Discord Voice Activity Indicators, Crypto/Stock Tickers.
* **Surface Model**: High-performance real-time rendering surface fed by backend event channels.
* **Input Routing Rules**:
  - Primarily passive display.
  - Context action on single click (e.g. click OBS video feed to toggle Studio mode; click chat to pause scrolling).

---

## 4. Plugin Manifest & Widget SDK Concept Summary

Every built-in widget and third-party plugin defines metadata that informs the Deck Engine of its layout capabilities and context-menu options. The following is the compact conceptual form. Section 9 contains the canonical production contracts, including input ports, mobile support, state schemas, bindings, and intents.

```typescript
export type WidgetInteractionType = 
  | 'atomic_button'
  | 'compound_zones'
  | 'continuous_fader'
  | 'rotary_dial'
  | 'telemetry_display';

export type ButtonExecutionMode = 
  | 'run_once'
  | 'repeat_while_pressing'
  | 'switch'
  | 'queue';

export interface WidgetGridSpan {
  cols: number;
  rows: number;
}

export interface WidgetManifest {
  /** Unique identifier of the widget type (e.g. 'vorti_spotify_player') */
  type: string;
  /** Human-readable display name */
  name: string;
  /** Category in the Widget Palette Modal */
  category: 'streaming' | 'audio' | 'utility' | 'system' | 'integrations';
  /** Primary interaction archetype */
  interactionType: WidgetInteractionType;
  /** Allowed grid size bounds */
  minSpan: WidgetGridSpan;
  maxSpan: WidgetGridSpan;
  defaultSpan: WidgetGridSpan;
  /** Execution modes allowed in the context menu (for atomic buttons) */
  supportedModes?: ButtonExecutionMode[];
  /** Default configuration payload stored in SQLite */
  defaultConfig: Record<string, any>;
  /** Preview icon component */
  icon: any;
}
```

---

## 5. Persistence Model and Backward Compatibility

### 5.1 Transitional option

The smallest possible migration is to extend `buttons` with layout and widget fields:

```sql
ALTER TABLE buttons ADD COLUMN col_span INTEGER NOT NULL DEFAULT 1;
ALTER TABLE buttons ADD COLUMN row_span INTEGER NOT NULL DEFAULT 1;
ALTER TABLE buttons ADD COLUMN widget_type TEXT NOT NULL DEFAULT 'action_button';
ALTER TABLE buttons ADD COLUMN widget_config TEXT DEFAULT '{}';
ALTER TABLE buttons ADD COLUMN custom_css TEXT DEFAULT '';
```

This is acceptable only as a short-lived compatibility step. It allows the existing V1 button rows to render as 1x1 action widgets without an immediate destructive migration.

It is not the preferred final model. A chat feed, media controller, telemetry panel, or plugin surface is not semantically a button. Extending `buttons` indefinitely would mix action scheduling, visual layout, plugin identity, live state, and arbitrary widget configuration in one table.

### 5.2 Recommended production model

The mature engine introduces first-class deck-item tables while retaining `buttons` during migration:

```sql
CREATE TABLE deck_items (
  id TEXT PRIMARY KEY NOT NULL,
  deck_id INTEGER NOT NULL,
  definition_id TEXT NOT NULL,
  package_id TEXT,
  contribution_id TEXT,
  config_json TEXT NOT NULL DEFAULT '{}',
  config_version INTEGER NOT NULL DEFAULT 1,
  appearance_json TEXT NOT NULL DEFAULT '{}',
  legacy_button_id INTEGER,
  enabled BOOLEAN NOT NULL DEFAULT TRUE,
  FOREIGN KEY(deck_id) REFERENCES decks(id),
  FOREIGN KEY(legacy_button_id) REFERENCES buttons(id)
);

CREATE TABLE deck_item_layouts (
  item_id TEXT NOT NULL,
  layout_key TEXT NOT NULL,       -- landscape, portrait, or future breakpoint
  x INTEGER NOT NULL,
  y INTEGER NOT NULL,
  col_span INTEGER NOT NULL,
  row_span INTEGER NOT NULL,
  PRIMARY KEY(item_id, layout_key),
  FOREIGN KEY(item_id) REFERENCES deck_items(id) ON DELETE CASCADE
);

CREATE TABLE deck_item_bindings (
  id TEXT PRIMARY KEY NOT NULL,
  item_id TEXT NOT NULL,
  port_id TEXT NOT NULL,
  action_json TEXT NOT NULL,
  policy_json TEXT NOT NULL,
  transform_json TEXT,
  FOREIGN KEY(item_id) REFERENCES deck_items(id) ON DELETE CASCADE
);
```

Persistent configuration and transient runtime state are deliberately separate. Current volume, viewer count, active OBS scene, playback position, pressed state, and connection health must not generate continuous SQLite writes. They travel through the runtime state protocol described later in this document.

### 5.3 Backward compatibility guarantee

* Every existing V1 button is exposed as a built-in `vortideck.action-button` or `vortideck.rotary` deck item.
* Existing button IDs remain valid for action execution, icons, limits, analytics, and saved user data.
* An adapter may synthesize deck items from button rows until a user deck is explicitly upgraded.
* Upgrading is transactional and reversible until the new document has validated and persisted successfully.
* Existing decks continue to open on the old path during the compatibility period.
* No widget migration may alter the meaning of a stored V1 action or execution mode.

---

## 6. Universal Context Menu Architecture

The global, theme-compliant context menu (`useGlobalContextMenu`) adapts its options dynamically based on the target widget's `interactionType`:

```
┌─────────────────────────────────────────────────────────────┐
│              Context Menu Dynamic Adaptation                │
├──────────────────────────────┬──────────────────────────────┤
│ Widget Type                  │ Context Menu Actions Shown   │
├──────────────────────────────┼──────────────────────────────┤
│ Atomic Action Button         │ • Choose Icon / Badge        │
│                              │ • Change Mode (Click/Rotary) │
│                              │ • Behavior (Run Once/Toggle/ │
│                              │   Repeat/Queue)              │
│                              │ • Repeat Interval Settings   │
│                              │ • Reset Button (Danger)      │
├──────────────────────────────┼──────────────────────────────┤
│ Audio Fader / Rotary         │ • Assign Audio Source        │
│                              │ • Scale Range (0dB to +6dB)  │
│                              │ • Curve (Linear / Log)       │
│                              │ • Reset Default Level        │
├──────────────────────────────┼──────────────────────────────┤
│ Compound Widget (e.g. Media) │ • Select Player Service      │
│                              │ • Album Art Quality          │
│                              │ • Configure Internal Actions │
├──────────────────────────────┼──────────────────────────────┤
│ Telemetry / Video Feed       │ • Data Refresh Interval      │
│                              │ • Video Feed Source (OBS)    │
│                              │ • Low-Latency Stream Mode    │
└──────────────────────────────┴──────────────────────────────┘
```

---

## 7. Current V1 Runtime Paths

Understanding the existing paths is essential: the migration preserves their behavior while replacing their coupling and rigid layout assumptions.

### 7.1 Stored button data

The current TypeScript and Rust button models contain:

```typescript
interface LegacyButton {
  id: number;
  deck_id: number;
  label: string;
  icon: string | null;
  background: string | null;
  action: string | null;
  position: number;
  mode: string | null;
  execution_mode: string | null;
  execution_config: string | null;
  icon_updated_at: number | null;
}
```

Two fields describe different concerns and must not be collapsed:

* `mode` chooses the visual/input renderer: currently `click/press` or `rotary`.
* `execution_mode` chooses backend scheduling semantics: `run_once`, `repeat_while_pressing`, `switch`, or `queue`.

### 7.2 Atomic click-button path

```text
SQLite Button
  -> Rust DeckWithNestedButtons payload
  -> useDeckStore.activeDeck
  -> simulator sortedButtons
  -> Deck/Dynamic/Button.vue
  -> simulator click/press/release handler
  -> useSendCommand()
  -> direct or E2EE-signed WebSocket command
  -> command_handler::handle_command_action
  -> button-limit and execution-policy lookup
  -> UniversalExecutionEngine
  -> macro / OBS / program / app / flow / HTTP / focus-window / system action
```

`Dynamic/Button.vue` is primarily a renderer. It displays the icon, label/position modifier views, theme CSS, padding, context-menu bounds, and haptic feedback. The simulator page currently owns run-once, switch, repeat, selection, lock, and editing behavior.

For `repeat_while_pressing`, the client does not continuously execute the action. It sends `start_button_repeat` and `stop_button_repeat`. The Rust host loads the stored repeat configuration, waits for the hold threshold, executes on an interval, and stops or aborts the task on release. The backend sends `button_repeat_started` so renderers can display an active-repeat state.

### 7.3 Rotary path

```text
SQLite Button with mode=rotary
  -> useDeckStore.activeDeck
  -> Deck/Buttons/Rotary.vue
  -> angle and hold gesture calculation
  -> throttled valueChange(number)
  -> useSendCommand(action, value, buttonId)
  -> command_handler::handle_command_action
  -> convert_action_to_universal_action
  -> UniversalExecutionEngine
  -> value-aware system/integration action
  -> system_volume_changed / volume_changed
  -> useDeckStore.systemVolume
  -> rotary visual state update
```

The rotary renderer legitimately owns specialized interaction mechanics:

* radial angle calculation;
* free, blocked, and continuous rotation behavior;
* hold-to-engage logic;
* range conversion;
* drag lifecycle and endpoint behavior;
* throttled preview values;
* rotary-specific visual feedback.

These mechanics must remain in a rotary renderer or rotary interaction controller. They do not belong in `WidgetHost`.

The existing synchronization is system-volume-specific. The backend currently injects volume into the legacy `background` JSON and broadcasts volume events. The new architecture replaces that overload with generic widget runtime state.

### 7.4 Desktop and mobile differences to eliminate

Desktop and mobile use the same stored buttons and base renderers, but the pages duplicate input scheduling, gesture suppression, repeat cleanup, haptics, feedback, and connection checks.

Some mobile calls send the action without a `buttonId`, forcing the backend to attempt source resolution by matching the serialized action string. That is ambiguous when multiple items share an action and weakens limit enforcement, state attribution, switch identity, and analytics.

The new protocol always includes stable `instanceId`, `portId`, and legacy `buttonId` when applicable. The host never guesses the source from the action payload.

### 7.5 Current implementation source map

The principal implementation paths traced for this specification are:

| Concern | Current source |
|---|---|
| Shared deck/button TypeScript models | `layers/base/types/deck.ts` |
| Pinia deck, theme, modifier, and system-volume state | `layers/base/stores/Deck.ts` |
| Desktop simulator orchestration | `layers/desktop/pages/deck/simulator.vue` |
| V2 layout/widget prototype | `layers/desktop/pages/deck/simulator-v2.vue` |
| Legacy click renderer | `layers/base/components/Deck/Dynamic/Button.vue` |
| Legacy rotary renderer and gesture logic | `layers/base/components/Deck/Buttons/Rotary.vue` |
| Shared command transport | `layers/base/composables/useSendCommand.ts` |
| Action assignment adapter | `layers/base/composables/useSetButtonAction.ts` |
| Mobile deck renderer and gesture handling | `layers/mobile/pages/deck.vue` |
| Mobile message/state handlers | `layers/mobile/composables/useWebSocketHandlers.ts` and `useConnectWs.ts` |
| Rust button/deck database models | `apps/tauri-windows/libs/vortideck-core/src/db/models.rs` |
| Rust Diesel schema | `apps/tauri-windows/libs/vortideck-core/src/db/schema.rs` |
| WebSocket command parsing and validation | `apps/tauri-windows/src-tauri/src/command_handler.rs` |
| Action conversion and universal dispatch | `apps/tauri-windows/src-tauri/src/action_execution.rs` and `vortideck-core/src/services/execution/` |
| Backend repeat lifecycle | `apps/tauri-windows/src-tauri/src/device_execution.rs` |
| Package/widget contribution model | `apps/tauri-windows/libs/vortideck-core/src/plugins/contributions.rs` |

---

## 8. Target Component Architecture

```text
DeckCanvas
  +-- GridLayoutEngine
  +-- WidgetHost (one per instance)
        +-- ActionButtonRenderer
        +-- RotaryRenderer
        +-- FaderRenderer
        +-- MediaRenderer
        +-- CompoundRenderer
        +-- TelemetryRenderer
        +-- SandboxedPluginRenderer
                  |
                  v
             WidgetIntent
                  |
                  v
       WidgetInteractionRouter
                  |
                  v
       Existing Execution Engine
                  |
                  v
       WidgetRuntimeState patches
```

The system consolidates contracts, lifecycle, security, input routing, state delivery, and shared chrome. It does not consolidate all visual and gesture behavior into one monolithic Vue component.

### 8.1 `DeckCanvas` responsibilities

`DeckCanvas` owns layout-level behavior:

* grid dimensions, gap, padding, margins, and orientation;
* item coordinates and spans;
* collision validation and rearrangement previews;
* drag, resize, keyboard movement, and multi-selection;
* portrait and landscape layout selection;
* layout transactions, undo/redo, autosave, and recovery;
* viewport scaling and safe-area handling.

The collision solver is not a `WidgetHost` concern.

### 8.2 `WidgetHost` responsibilities

`WidgetHost` is the universal instance boundary. It owns:

* resolving a widget definition and renderer from the registry;
* providing normalized configuration, theme tokens, density, runtime state, and host context;
* edit versus interactive mode gating;
* selection outline, common focus semantics, and resize handles supplied by the canvas;
* the common context-menu entry point;
* subscription/plan lock overlays;
* package capability and permission state;
* loading, disconnected, missing-package, revoked-permission, and renderer-error fallbacks;
* sandbox selection for untrusted plugin renderers;
* routing renderer intents to `WidgetInteractionRouter`;
* common accessibility metadata and input-source reporting;
* host-level pressed/executing/error indicators when requested by the renderer;
* cleanup of subscriptions and in-flight interactions on unmount or mode change.

`WidgetHost` must not:

* calculate rotary angles or fader geometry;
* understand Spotify, OBS, Twitch, audio-device, or plugin domain logic;
* call `useSendCommand`, Tauri `invoke`, WebSocket APIs, or integrations directly;
* read the global deck store from inside a renderer;
* implement collision resolution;
* decide arbitrary execution semantics from a renderer type;
* persist high-frequency runtime values to SQLite.

### 8.3 Renderer responsibilities

A renderer owns presentation and specialized input interpretation only. It receives immutable instance configuration plus runtime state, then emits normalized intents.

Examples:

* `ActionButtonRenderer`: icon, label, badge, pressed visuals, primary surface.
* `RotaryRenderer`: angle/value conversion and rotary gesture mechanics.
* `FaderRenderer`: vertical/horizontal coordinate-to-value conversion and mute sub-zone.
* `MediaRenderer`: track presentation and independently addressable playback zones.
* `TelemetryRenderer`: efficient passive rendering with optional contextual actions.
* `SandboxedPluginRenderer`: iframe lifecycle, postMessage validation, CSP, and capability-scoped bridge.

Renderers never execute actions or own external truth. Optimistic visuals are allowed, but authoritative state returns through `WidgetRuntimeState`.

---

## 9. Universal Widget Contracts

### 9.1 Definitions and input ports

One coarse `interactionType` is useful for categorization, but it is insufficient for compound widgets. The executable contract is a list of typed input ports:

```typescript
export interface WidgetDefinition {
  id: string;
  name: string;
  category: string;
  rendererId: string;
  interactionType:
    | 'atomic_button'
    | 'compound_zones'
    | 'continuous_value'
    | 'telemetry_display';

  defaultSpan: WidgetGridSpan;
  minSpan: WidgetGridSpan;
  maxSpan: WidgetGridSpan;

  configSchema: JsonSchema;
  stateSchema?: JsonSchema;
  inputs: WidgetInputPort[];
  stateSubscriptions?: WidgetStateSubscription[];
  mobileSupport: 'host_schema' | 'synced_web_bundle' | 'fallback_only';
}

export type WidgetInputPort =
  | {
      id: string;
      kind: 'trigger';
      supportedPolicies: TriggerExecutionPolicyType[];
    }
  | {
      id: string;
      kind: 'value';
      valueType: 'number';
      min: number;
      max: number;
      supportedDelivery: Array<'live' | 'commit' | 'live_and_commit'>;
    }
  | {
      id: string;
      kind: 'command';
      payloadSchema?: JsonSchema;
    };
```

Examples:

| Widget | Ports |
|---|---|
| Action button | `primary: trigger` |
| Rotary | `value: value`, optionally `press: trigger` |
| Fader | `value: value`, `mute: trigger` |
| Media bar | `previous`, `playPause`, `next`, `seek` |
| OBS scene matrix | one trigger port per scene |
| Telemetry gauge | no ports, or one contextual trigger |
| Compound plugin | any manifest-declared combination |

The host supports the port protocol universally. Individual widgets expose only the ports they actually implement.

### 9.2 Deck instances

```typescript
export interface DeckWidgetInstance {
  id: string;
  definitionId: string;
  packageId?: string;
  contributionId?: string;
  configVersion: number;
  config: Record<string, unknown>;
  appearance?: Record<string, unknown>;
  layouts: Record<string, WidgetPlacement>;
  bindings: WidgetBinding[];
  legacyButtonId?: number;
  enabled: boolean;
}
```

The definition describes what a widget type can do. The instance describes one configured placement on one user deck.

### 9.3 Bindings and execution policies

```typescript
export interface WidgetBinding {
  id: string;
  portId: string;
  action: ActionReference;
  policy: WidgetExecutionPolicy;
  transform?: ValueTransform;
}

export type WidgetExecutionPolicy =
  | { type: 'run_once' }
  | { type: 'repeat'; holdMs: number; intervalMs: number }
  | {
      type: 'toggle';
      onAction?: ActionReference;
      offAction?: ActionReference;
      persistState?: boolean;
    }
  | { type: 'queue'; maxDepth?: number }
  | {
      type: 'continuous';
      throttleMs: number;
      commitOnRelease: boolean;
    };
```

Execution policy belongs to a binding/port, not to the visual renderer. A rotary can expose a continuously delivered value and a separate press trigger. A compound media controller can assign a different action to every zone.

### 9.4 Universal intents

```typescript
export interface WidgetIntent {
  instanceId: string;
  portId: string;
  legacyButtonId?: number;
  kind: 'trigger' | 'value' | 'command';
  phase: 'start' | 'preview' | 'commit' | 'end' | 'cancel';
  value?: unknown;
  sequence: number;
  timestamp: number;
  inputSource: 'mouse' | 'touch' | 'pen' | 'keyboard' | 'remote';
}
```

Representative intents:

```typescript
// Atomic tap
{ instanceId, portId: 'primary', kind: 'trigger', phase: 'commit' }

// Hold-to-repeat lifecycle
{ instanceId, portId: 'primary', kind: 'trigger', phase: 'start' }
{ instanceId, portId: 'primary', kind: 'trigger', phase: 'end' }

// Rotary/fader feedback and commitment
{ instanceId, portId: 'value', kind: 'value', phase: 'preview', value: 63 }
{ instanceId, portId: 'value', kind: 'value', phase: 'commit', value: 63 }

// Compound widget zone
{ instanceId, portId: 'playPause', kind: 'command', phase: 'commit' }
```

The router validates definition, port, binding, plan limits, capability grants, sequence, and value schema before dispatching to the backend.

### 9.5 Runtime state

```typescript
export interface WidgetRuntimeState {
  instanceId: string;
  revision: number;
  status: 'loading' | 'ready' | 'executing' | 'disconnected' | 'error';
  values: Record<string, unknown>;
  error?: { code: string; message: string };
}
```

Examples include `{ value: 63, muted: false }`, `{ activeScene: 'Game' }`, `{ playing: true, progress: 0.42 }`, and `{ cpu: 42, gpuTemp: 61 }`.

Runtime updates are revisioned so stale WebSocket or plugin responses cannot overwrite newer state. The host applies snapshots and patches; renderers display them.

---

## 10. Input Routing and Execution

`WidgetInteractionRouter` is the convergence point for desktop, mobile, keyboard, plugin, and future hardware input.

Its responsibilities are:

1. Validate the instance and port.
2. Reject interaction in edit, locked, disabled, missing-package, or revoked-permission states.
3. Resolve the binding and action reference.
4. Apply value transforms and schema validation.
5. Apply execution policy: run once, repeat lifecycle, toggle, queue, or continuous throttling/commit.
6. Attach stable source identity and correlation IDs.
7. Dispatch through the existing Universal Execution Engine.
8. Publish acknowledged state, execution status, and errors.
9. Cancel held or continuous interactions on disconnect, deck change, app backgrounding, or renderer unmount.

The existing action enum and Universal Execution Engine remain the initial execution backend. Widget work must not fork a second macro/flow/OBS/system executor.

For compatibility, legacy button bindings translate into the current serialized `Action` enum and button execution configuration. The backend eventually accepts widget instance and binding IDs directly, while retaining `legacyButtonId` during migration.

---

## 11. Theme, Appearance, and Settings

All current V1 deck settings remain supported:

* grid size;
* button/widget spacing;
* grid margin and padding;
* maximize-space behavior;
* side controls;
* active theme;
* button padding;
* label/icon modifier views;
* subscription limits and lock presentation. `ui_buttons` is a physical row-major deck-cell allowance across built-in and plugin widgets: multi-span widgets consume every covered cell, crossing the allowance locks the whole widget, and vacant restricted cells retain a stable lock affordance;

The theme contract evolves from button-specific CSS strings toward host tokens:

```typescript
interface WidgetThemeContext {
  surface: string;
  surfaceHover: string;
  text: string;
  textMuted: string;
  primary: string;
  secondary: string;
  danger: string;
  border: string;
  radius: string;
  shadow: string;
  spacingScale: number;
}
```

Legacy `clickButton` and `rotaryButton` theme CSS is adapted into these tokens during compatibility. Specialized renderers may define additional namespaced tokens, but a plugin cannot inject unrestricted CSS into the parent editor.

---

## 12. Plugin and Package Integration

The existing `Contribution::Widget` contract currently describes overlay widgets. Deck-control widgets have different input, sizing, latency, mobile, and authority requirements. The plugin manifest must therefore add either:

* a distinct `deckWidget` contribution; or
* an explicit `surfaces: ['deck', 'overlay']` declaration plus deck-specific fields.

Recommended contribution shape:

```json
{
  "type": "widget",
  "id": "media-controller",
  "name": "Media Controller",
  "surfaces": ["deck"],
  "renderer": { "type": "hostSchema", "schemaId": "media-controller-v1" },
  "interactionType": "compound_zones",
  "defaultSpan": { "cols": 2, "rows": 1 },
  "minSpan": { "cols": 2, "rows": 1 },
  "maxSpan": { "cols": 4, "rows": 2 },
  "inputs": [
    { "id": "previous", "kind": "trigger" },
    { "id": "playPause", "kind": "trigger" },
    { "id": "next", "kind": "trigger" },
    { "id": "seek", "kind": "value", "min": 0, "max": 1 }
  ],
  "capabilities": {
    "subscribe": ["media.playback"],
    "actions": ["media.control"]
  }
}
```

Built-in and third-party widgets use the same definition and instance contracts. Trust determines renderer mode:

* built-in host renderer: reviewed Vue/host schema;
* verified/community declarative renderer: host-rendered schema with capability-scoped data;
* rich web renderer: sandboxed iframe, strict CSP, validated postMessage bridge;
* plugin runtime: remains desktop-authoritative and runs through existing permission enforcement.

Removing, disabling, or failing a plugin never silently deletes deck configuration. `WidgetHost` displays a recoverable missing-package/error placeholder with reinstall, replace, remove, and inspect actions.

### 12.1 Deck view schema versus overlay component schema

The deck/mobile declarative renderer must be a **separate schema** from the existing overlay `ComponentSchema`.

The overlay schema in `layers/desktop-streaming/composables/overlay/componentSchema.ts` contains an HTML `template`, unrestricted CSS-variable strings, and in several built-ins inline `<style>` and `<script>` blocks. Its output is intended for browser/OBS rendering and `UniversalElement.vue` selects `v-html` or a sandboxed iframe. That is appropriate for a broadcast surface, but it is not a safe portable mobile control contract.

The two systems should share only stable lower-level concepts:

* package and contribution identity;
* design tokens such as colors, radii, typography, spacing, and semantic status colors;
* typed state paths and event-topic vocabulary;
* asset references and content hashes;
* editor metadata for safe properties.

They must not share executable templates, DOM assumptions, CSS strings, scripts, or browser APIs. An overlay is viewer-facing output on a large freeform canvas. A deck widget is latency-sensitive authenticated input on a constrained grid. A package may contribute both surfaces, but it supplies an explicit view for each:

```json
{
  "type": "widget",
  "id": "spotify-controller",
  "surfaces": {
    "overlay": { "renderer": "sandboxedHtml", "entry": "overlay.html" },
    "deck": {
      "desktopRenderer": { "type": "sandboxedHtml", "entry": "deck.html" },
      "portableView": { "type": "hostSchema", "schema": { "...": "..." } }
    }
  }
}
```

### 12.2 Portable deck view schema

The portable schema is an immutable, versioned render tree interpreted by trusted `WidgetHost` code. It describes presentation and maps gestures to already-declared input ports. It never contains executable logic or an action implementation.

```typescript
interface PortableWidgetView {
  schemaVersion: 1;
  root: WidgetViewNode;
  assets?: WidgetAssetRef[];
  accessibility?: {
    label?: WidgetValue;
    hint?: WidgetValue;
  };
}

type WidgetViewNode =
  | LayoutNode       // stack, row, column, grid, spacer, divider
  | SurfaceNode      // panel, shape
  | ContentNode      // text, icon, image, badge
  | DisplayNode      // progress, meter, status, list
  | ControlNode;     // press-zone, toggle, slider, rotary

type WidgetValue =
  | { literal: string | number | boolean | null }
  | { config: string; fallback?: unknown }
  | { state: string; fallback?: unknown }
  | { select: { when: WidgetCondition; then: WidgetValue; else: WidgetValue } };

interface WidgetGestureBinding {
  gesture: "press" | "hold" | "release" | "rotate" | "drag";
  portId: string;
  delivery?: "start_end" | "preview_commit" | "commit";
}
```

The initial primitive set should deliberately remain small: `stack`, `row`, `column`, `grid`, `panel`, `text`, `icon`, `image`, `badge`, `progress`, `meter`, `press-zone`, `toggle`, `slider`, and `rotary`. Conditions permit only bounded operations such as equality, range comparison, boolean composition, and selection. Styling uses typed tokens and bounded numeric values rather than arbitrary CSS.

The schema must reject:

* HTML, JavaScript, WASM, C++, native libraries, and `eval`-like expressions;
* scriptable URLs, remote stylesheets, arbitrary fonts, and direct network access;
* filesystem, clipboard, microphone, camera, integration-token, or mobile-device API access;
* undeclared state paths or input ports;
* unbounded recursion, node counts, strings, lists, images, animations, or update rates.

### 12.3 Renderer tiers and exact visual parity

There are three production renderer tiers:

1. **Shared built-in renderer.** VortiDeck button, rotary, fader, toggle, media, and telemetry components are compiled into desktop and mobile. When platform-safe, they use the same base Vue renderer and therefore retain exact V1 visual/gesture parity.
2. **Portable plugin renderer.** A plugin supplies `PortableWidgetView`. The same trusted schema renderer runs on desktop and mobile, producing close and normally identical visuals from the same tree and theme tokens.
3. **Desktop-rich renderer.** Arbitrary plugin HTML/JS runs only in the desktop iframe sandbox. The plugin must additionally provide a portable companion view for mobile. The companion can reproduce the control surface but need not reproduce browser-only video, WebGL, complex animation, or editing behavior.

The desktop plugin runtime may be WASM, service code, or another supported host runtime. It owns integration logic under package permissions. It is not copied to the phone. The phone never becomes a second plugin host.

Downloaded executable mobile renderers are excluded from the initial architecture. Supporting them later would require publisher signatures, platform review, a separate sandbox/runtime, revocation, vulnerability response, and an app-store-compliant code-loading policy. Declarative views avoid this security and lifecycle burden.

### 12.4 Authority boundary

The portable view may emit only `{ instanceId, portId, phase, value, sequence }`. The authoritative desktop resolves that intent against the current persisted item, definition, grant, plan limits, binding, execution policy, and action. Mobile must never receive or choose the executable binding action.

Consequently the mobile wire projection of an instance is intentionally narrower than the persisted desktop instance:

```typescript
interface RemoteWidgetInstance {
  id: string;
  definitionId: string;
  packageId?: string;
  contributionId?: string;
  configVersion: number;
  publicConfig: Record<string, unknown>;
  appearance: Record<string, unknown>;
  layouts: Record<string, WidgetPlacement>;
  enabled: boolean;
  availablePorts: string[];
}
```

`bindings[].action`, plugin secrets, credentials, private configuration, filesystem paths, permission internals, and execution configuration that is not needed for presentation are excluded. The current snapshot serializes full `DeckWidgetInstance.bindings`; that must be replaced by this public projection before the protocol is considered final.

### 12.5 Assets

Portable views reference declared assets by ID, never arbitrary local paths:

```typescript
interface WidgetAssetRef {
  id: string;
  sha256: string;
  mediaType: "image/png" | "image/jpeg" | "image/webp";
  byteLength: number;
  purpose: "icon" | "artwork" | "background";
}
```

Static package assets are content-addressed and transferred once. Dynamic artwork is a separate expiring state asset with strict byte, dimension, frequency, and cache limits. SVG is currently rejected; a future desktop rasterization stage may convert it to a declared raster object, but active SVG content is never accepted. Mobile resolves only assets listed in the authenticated manifest.

### 12.6 Desktop plugin sandbox

The existing `SandboxedPluginAdapter.vue` is the desktop rich-renderer boundary: `sandbox="allow-scripts"`, no same-origin privilege, `connect-src 'none'`, data/blob-only media, and a validated `postMessage` intent bridge. This remains a desktop option, but it requires further production hardening:

* size and parse limits before constructing `srcdoc`;
* explicit origin/source and per-frame capability token checks in bridge messages;
* schema validation for config, state, intent value, and phase/port compatibility;
* lifecycle cancellation and rate limits;
* removal/revocation handling without deleting the stored instance;
* security tests for CSP escape, navigation, popups, downloads, oversized messages, and intent floods.

### 12.7 Package manifest target

`DeckSurfaceContribution` currently stores `rendererId`, schemas, ports, subscriptions, and `mobileSupport`; it has no portable view. The final manifest replaces the ambiguous string flag with explicit renderer declarations:

```typescript
interface DeckSurfaceContributionV2 {
  definitionId: string;
  desktopRenderer:
    | { type: "builtin"; rendererId: string }
    | { type: "hostSchema"; view: PortableWidgetView }
    | { type: "sandboxedHtml"; entry: string };
  portableView:
    | { type: "hostSchema"; view: PortableWidgetView }
    | { type: "builtin"; rendererId: string }
    | { type: "unsupported"; reason: string };
  configSchema: Record<string, unknown>;
  publicConfigPaths: string[];
  stateSchema: Record<string, unknown>;
  publicStatePaths: string[];
  inputs: WidgetInputPort[];
  stateSubscriptions: string[];
  defaultSpan: WidgetGridSpan;
  minSpan: WidgetGridSpan;
  maxSpan: WidgetGridSpan;
}
```

Third-party packages cannot select a privileged built-in renderer ID. Package IDs namespace definition IDs, and updates that change permissions or public data exposure require re-consent.

---

## 13. Mobile Companion Protocol

Mobile is a renderer and input client. It does not install the desktop package, load its runtime, hold its integration credentials, or execute its action code. “Installing a plugin widget on mobile” means caching an authenticated portable definition and safe assets, not installing the plugin.

### 13.1 Final distribution flow

```text
Package installed/granted on desktop
       │
       ├── runtime + permissions + credentials ──> desktop only
       ├── rich HTML/JS renderer ────────────────> desktop sandbox only
       └── portable view + public state/assets
                          │
                  paired E2EE channel
                          ▼
             mobile trusted WidgetHost
                          │
                 typed signed intent
                          ▼
       desktop validation + Universal Execution Engine
```

The lifecycle is:

1. During pairing/reconnect, both peers negotiate protocol and portable-schema versions.
2. Mobile sends its cache inventory as content hashes, not whole cached documents.
3. Desktop sends an authenticated manifest for definitions/assets used by the active deck.
4. Mobile requests only missing hashes, validates every object, stores accepted immutable objects, and atomically activates the snapshot.
5. On deck change, desktop sends the new manifest plus missing definitions/assets and then the deck snapshot.
6. On plugin update/grant/revocation, desktop sends a definition-manifest delta. A revoked or invalid definition becomes a recoverable placeholder immediately.
7. Runtime changes use small ordered state patches. Definitions are not resent with every state update.
8. Mobile gestures become signed intents. Desktop validates and executes them, then returns acknowledgement and authoritative state.

Definitions may be proactively sent with a deck snapshot or fetched on demand by hash. The preferred behavior is one round-trip manifest synchronization before activating a deck, with lazy transfer only for large non-critical artwork. This avoids blank widgets during normal deck switching while keeping reconnections small.

### 13.2 Protocol objects

```typescript
interface DeckWidgetManifestMessage {
  action: "deck_widget_manifest";
  protocolVersion: 2;
  schemaVersions: number[];
  deckId: number;
  deckRevision: number;
  layoutKey: "portrait" | "landscape";
  definitions: Array<{
    definitionId: string;
    packageId?: string;
    version: string;
    contentHash: string;
  }>;
  assets: WidgetAssetRef[];
}

interface DeckWidgetSnapshotMessageV2 {
  action: "deck_widget_snapshot";
  protocolVersion: 2;
  deckId: number;
  revision: number;
  layoutKey: "portrait" | "landscape";
  instances: RemoteWidgetInstance[];
  state: WidgetStateSnapshot[];
}

interface WidgetDefinitionObject {
  definition: WidgetDefinition;
  portableView: PortableWidgetView;
  packageId?: string;
  packageVersion?: string;
  contentHash: string;
}
```

Definition and asset objects are immutable by hash. Deck snapshots reference them. Definition deltas are separate from document deltas so an installed plugin update can replace presentation without rewriting every deck instance.

### 13.3 Cache model

The final mobile cache is persistent and partitioned by paired desktop identity:

```text
pairedDesktopId /
  definitions / sha256
  assets / sha256
  manifests / deckId + revision
```

Cache rules:

* verify authenticated transport, declared hash, schema, size, and media type before commit;
* write to a temporary entry and atomically rename only after complete validation;
* impose per-object, per-package, per-desktop, and total LRU quotas;
* never cache bindings/actions, credentials, tokens, private config/state, local paths, or permission grants;
* last-known runtime state may be cached only for offline appearance and is visibly stale/non-authoritative;
* remove unreachable assets through LRU/manifest garbage collection;
* purge the paired namespace on unpair, with an explicit optional “keep deck appearance” product decision only if no sensitive state is retained;
* invalidate a definition immediately on package revocation, failed re-consent, unsupported schema version, or hash mismatch.

The existing `useWidgetRegistry` map is process memory, not this cache. It registers snapshot definitions only when an ID is absent, so it also cannot correctly activate a changed definition with the same ID. The target registry resolves `{ definitionId, contentHash }` and has separate built-in, installed-desktop, and remote-cache scopes.

### 13.4 Transport authentication and signing

All post-pairing direct and relay messages use canonical HMAC authentication, nonces, and replay rejection. This authenticates portable definitions and assets as coming from the paired desktop. TLS protects each relay network hop; payload confidentiality from the relay itself requires the planned encrypted protocol-v3 envelope.

A separate publisher signature is not required for non-executable declarative objects. It would be required if VortiDeck later allowed downloaded executable renderer bundles. Package provenance/signatures may still be added for marketplace trust and desktop installation, independently of the phone protocol.

Transport authentication does not replace content validation. A compromised or malicious paired desktop can send hostile data, so the mobile parser still enforces schema allowlists, resource ceilings, sanitized assets, and zero executable content.

### 13.5 Failure behavior

Mobile never falls back to executing unknown content. It shows a typed placeholder for:

* missing definition or asset;
* unsupported schema/protocol version;
* invalid hash or schema;
* plugin removed, revoked, or awaiting re-consent;
* desktop-only widget;
* disconnected/stale authoritative state.

The placeholder preserves placement and instance identity and can explain the recovery action. It is a security state, not a legacy execution path.

### 13.6 Current implementation versus target

| Area | Current repository | Final target |
|---|---|---|
| Built-in mobile rendering | Shared base Vue renderers are compiled into mobile | Retain; formalize built-in renderer compatibility/versioning |
| Plugin desktop rendering | Package HTML runs in `SandboxedPluginAdapter` | Retain and harden |
| Plugin mobile rendering | Portable host-schema renderer is implemented; unsupported widgets use a placeholder | Expand primitive/accessibility coverage and device E2E |
| Portable schema field | `portableView` exists on deck-surface contributions | Stabilize/version through the plugin SDK |
| Definition validation | Bounded Rust and TypeScript allowlist validators | Add shared conformance fixtures/fuzzing |
| Definition updates | Snapshots/document deltas carry hashes; grant/re-consent/revoke broadcasts refresh entries; v2 inventory requests only missing immutable objects | Add explicit manifest-first atomic activation and remove redundant v1 definition payloads after negotiation |
| Cache | Persistent, bounded, per-paired-desktop definition and raster-asset cache with inventory and revocation GC | Add persisted deck manifests and broader quota/telemetry controls |
| Assets | Authenticated content-addressed PNG/JPEG/WebP transfer, integrity/type checks, bounded cache, renderer resolution | Add desktop SVG rasterization and dynamic expiring artwork channel |
| Mobile instance projection | Actions are redacted; plugin config/state use public allowlists | Freeze protocol-v2 public projection fixtures |
| Layout negotiation | Clients select portrait/landscape locally; snapshot still defaults to `portrait` | Protocol-level orientation negotiation and device E2E |
| `synced_web_bundle` | Enum value only; no transfer/runtime | Remove from v1 protocol; defer executable bundles |
| Security | Signed/replay-protected direct and relay transport | Retain plus schema/asset validation and cache isolation |

Mobile requirements remain:

* identical coordinates/spans for the selected layout;
* pointer/touch cancellation on swipe, pinch, multitouch, app background, deck change, and disconnect;
* authenticated direct and relay intents;
* stable `instanceId`, `portId`, sequence, and correlation ID;
* snapshot recovery when document or state revisions are missed;
* a safe fallback renderer for unsupported definitions;
* no integration credentials, executable bindings, or unrestricted plugin authority on the phone.

---

## 14. Layout Solver Requirements

V2's occupancy-matrix concept is valuable, but the production solver must be deterministic and non-destructive.

Rules:

* Enforce definition min/max spans on add, resize, paste, template load, and migration.
* Never silently shrink an unrelated widget.
* Preview every affected move before commit.
* Reject an operation when no valid arrangement exists, or offer an explicit user-approved compaction.
* Keep all changes in a transaction and roll back atomically on failure.
* Revalidate after grid-size or orientation changes.
* Persist separate portrait and landscape layouts behind an explicit deck policy. New decks default to Adaptive Layout; existing decks migrate conservatively to Independent so custom arrangements are not rewritten. Adaptive mirrors successful edits into the other orientation, Independent changes only the visible orientation, and a one-shot current-to-other command is always available. A failed background mirror keeps the user's current edit and the previous opposite layout; a new widget is rejected when it cannot be placed in both orientations.
* Place new widgets in the first valid region rather than fixed coordinates.
* Support keyboard movement/resizing and accessible announcements.
* Record layout operations for undo/redo.
* Autosave only validated documents, with a recoverable previous revision.

---

## 15. Legacy Adapters and Migration

### 15.1 Action-button adapter

```typescript
function legacyButtonToWidget(button: LegacyButton): DeckWidgetInstance {
  return {
    id: `button:${button.id}`,
    definitionId:
      button.mode === 'rotary'
        ? 'vortideck.rotary'
        : 'vortideck.action-button',
    configVersion: 1,
    config: {
      label: button.label,
      iconRef: button.id,
    },
    layouts: {
      default: legacyPositionToPlacement(button.position),
    },
    bindings: [legacyButtonBinding(button)],
    legacyButtonId: button.id,
    enabled: true,
  };
}
```

The first adapter continues rendering the existing `Dynamic/Button.vue`. The host supplies theme, padding, icon, edit/interactive mode, and runtime status. Raw input is translated to `WidgetIntent`; the legacy renderer stops calling stores or transports directly.

### 15.2 Rotary adapter

The rotary adapter continues using the existing angle and drag mechanics, but receives:

* normalized range and behavior config;
* current value from `WidgetRuntimeState`;
* theme tokens from host context;
* interaction availability from the host.

It emits preview/commit value intents. System-volume-specific store watching and `background` JSON parsing are removed after parity is proven.

### 15.3 Migration principle

Wrap first, verify parity, then refactor. Do not rewrite the mature renderers and execution engine simultaneously.

---

## 16. Implementation Roadmap

### Phase 0: Contracts and regression baseline

* Freeze TypeScript/Rust wire contracts for definitions, instances, placements, bindings, intents, and runtime state.
* Add characterization tests for current V1 click, rotary, repeat, switch, queue, limits, themes, icon handling, mobile, and relay behavior.
* Add contract fixtures shared by Rust and TypeScript tests.

### Phase 1: Host shell with legacy data

* Introduce `DeckCanvas`, `WidgetHost`, widget registry, and legacy adapters.
* Render current 1x1 buttons and rotaries through the host without changing SQLite.
* Preserve V1 layout, settings, themes, drag/drop assignment, and context menus.

### Phase 2: Intent router

* Introduce `WidgetIntent` internally.
* Move desktop/mobile duplicated input scheduling into `WidgetInteractionRouter`.
* Preserve the existing backend command and repeat paths through compatibility translation.
* Require source identity on every command.

### Phase 3: First-class persistence

* Add `deck_items`, `deck_item_layouts`, and `deck_item_bindings` migrations and Rust models.
* Add transactional document save/load APIs and revisioning.
* Keep legacy button linkage and rollback support.

### Phase 4: Multi-span layout

* Extract and harden the V2 solver.
* Add constraints, non-destructive collision behavior, undo/redo, autosave, portrait/landscape layouts, and recovery.
* Add property-based solver tests.

### Phase 5: Generic runtime state

* Replace rotary/system-volume special cases with versioned state snapshots and patches.
* Add execution acknowledgements, optimistic state reconciliation, error state, and cancellation.

### Phase 6: Mobile parity

* Add versioned deck snapshots and deltas to direct and relay WebSocket paths.
* Render the same widget instances through the mobile host.
* Validate reconnect, revision gaps, orientation, gestures, and E2EE intent routing.
* Introduce the redacted `RemoteWidgetInstance`; never serialize executable actions or private config/state to mobile.
* Negotiate portrait/landscape layout instead of hard-coding the snapshot layout key.

### Phase 7: Built-in advanced widgets

* Ship real fader, toggle, media, OBS, and telemetry definitions one at a time.
* Connect each to real integrations through bindings and runtime state.
* Do not promote simulated V2 data into production widgets.

### Phase 8: Plugin widgets

* Extend package contributions with deck surface metadata, ports, schemas, span constraints, and mobile support.
* Connect the widget palette to installed/granted packages.
* Enforce capability grants, CSP, sandboxing, state filtering, and graceful failure.
* Add `PortableWidgetView` types plus strict Rust and TypeScript validators with depth/count/size limits.
* Add the shared declarative renderer and primitive-level component/accessibility tests.
* Replace `mobileSupport`/`synced_web_bundle` hints with explicit desktop and portable renderer declarations.
* Add definition/asset manifests, hashes, definition deltas, cache inventory negotiation, and per-device persistent cache isolation.
* Add package update, re-consent, revoke, missing asset, corrupt schema, stale cache, and unsupported-version E2E tests.

### Phase 9: Retirement

* Make the unified engine the default only after V1 parity gates pass.
* Keep a rollback feature flag for at least one release cycle.
* Remove `/deck/simulator-v2` and V1-only rendering code after migration telemetry and recovery tests are satisfactory.

---

## 17. Testing Strategy

### Unit tests

* layout placement, collision, rollback, orientation, and span constraints;
* intent validation, value transforms, throttling, debouncing, and cancellation;
* execution-policy state machines;
* definition/config/state schema validation;
* legacy adapter round trips;
* runtime-state revision handling.

### Component tests

* `WidgetHost` edit/interactive gating;
* locked, loading, disconnected, missing-package, revoked, and error states;
* button press/release/cancel;
* rotary preview/commit and external-state reconciliation;
* compound zone isolation;
* context menu and keyboard accessibility.

### Rust contract and integration tests

* SQLite migrations and rollback;
* deck document serialization compatibility;
* intent-to-action resolution;
* plan/capability enforcement;
* repeat lifecycle cleanup;
* switch/queue source identity;
* state broadcast ordering and revision recovery.

### End-to-end tests

* open an existing V1 deck, execute every supported action type, edit, save, restart, and compare behavior;
* operate the same deck from desktop simulator and Android/iOS companion;
* direct and relay/E2EE paths;
* rotate mobile during held/continuous input;
* disconnect/reconnect during repeat or fader interaction;
* install, grant, use, revoke, break, reinstall, and remove a plugin widget;
* migration failure and recovery from the previous deck revision.

---

## 18. Production Acceptance Criteria

The unified engine is ready to replace V1 only when:

1. Every existing V1 deck opens without user intervention and retains its actions, positions, labels, icons, themes, settings, and execution policies.
2. Click, rotary, run-once, repeat, switch, queue, modifier views, limits, and context-menu operations match V1 behavior.
3. Saving and restarting produces an identical validated deck document.
4. Desktop and mobile render the same active layout and converge on authoritative state after reconnect.
5. Direct and relay/E2EE input use the same intent semantics and always preserve source identity.
6. Multi-span add, move, resize, grid change, and rotation never overlap, lose, or silently shrink unrelated widgets.
7. Undo/redo and crash recovery restore the previous valid revision.
8. Plugin widgets cannot exceed granted events, actions, UI, storage, or network capabilities.
9. Missing or broken packages degrade to recoverable placeholders without deleting user configuration.
10. High-frequency values do not flood SQLite, WebSocket, or the execution engine beyond configured delivery policy.
11. Automated parity, contract, solver, mobile, migration, and plugin-security suites pass.
12. Mobile snapshots contain no executable action bindings, credentials, private config/state, or unrestricted local paths.
13. Declarative definitions and assets are hash-verified, bounded, cached per paired desktop, and invalidated on revoke/re-consent failure.
14. The same portable schema fixture renders and emits equivalent typed intents on desktop and mobile.

---

## 19. Final Architectural Decision

VortiDeck will evolve the V1 production engine rather than rebuild from the V2 demo.

The existing click button and rotary become widget renderers through compatibility adapters. They retain their specialized visuals and gesture mechanics. `WidgetHost` becomes the universal lifecycle, state, security, theming, editing, and intent boundary. `DeckCanvas` owns layout. `WidgetInteractionRouter` owns input policy and dispatch. The existing Universal Execution Engine remains authoritative for actions and integrations.

The plugin distribution decision is equally strict: packages and arbitrary code are installed and executed on desktop; mobile receives only a validated portable view, safe public state, bounded content-addressed assets, and declared input ports. Built-ins retain shared compiled renderers. Rich desktop HTML/JS receives a declarative mobile companion instead of being copied to the phone. The overlay component system and deck view system share tokens, identities, assets, and typed data vocabulary, but not their render documents or runtimes.

The final ownership model is:

| Layer | Owns |
|---|---|
| `DeckCanvas` | responsive grid, orientation layouts, move/resize, selection |
| `WidgetHost` | lifecycle, modes, theme, lock/error states, renderer boundary |
| Built-in/portable/rich renderer | presentation and specialized gesture interpretation |
| `WidgetInteractionRouter` | intent normalization, cancellation, delivery policy |
| Desktop widget service | definition resolution, public projection, manifests, assets, state publication |
| Universal Execution Engine | bindings, permissions, policy execution, integrations, authoritative result |
| Mobile sync/cache | authenticated portable objects and transient public state only |
| Plugin runtime | desktop-only integration/business logic within granted capabilities |

The result is one deck engine that is as reliable as V1, as flexible as the V2 concept, consistent across desktop and mobile, and extensible through the existing package and permission system without making mobile an arbitrary-code plugin host.
