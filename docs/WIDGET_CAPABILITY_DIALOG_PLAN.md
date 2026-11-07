# Implementation Plan — Widget Capability System + Permission Dialog

**Status:** Ready to implement
**Design ref:** `WIDGET_CAPABILITY_ARCHITECTURE.md`
**Goal:** Make overlay widgets first-class, **safe** engine participants — a manifest-driven capability model, a per-widget sandbox+CSP, a capability-filtered bridge, and the **Capability Dialog** window the user approves at install. Built on the existing `PermissionChecker` / `PluginPermission`.

This plan is layered — each phase is independently shippable. **Phase 4 (the dialog) is the user-facing centerpiece** and is specified in full at the bottom.

---

## Phase 0 — Package manifest: contributions list (extends existing PluginManifest)

Extend the existing `PluginManifest` (`bundle.rs`) with a typed `contributions` list — generalizing today's `runtime`+`web` into N typed entries, and adding `widget` as a contribution type. This is a generalization of what exists, not a new system (see architecture doc §3.5).

**Files:**
- `apps/tauri-windows/libs/vortideck-core/src/plugins/bundle.rs` — add `contributions: Vec<Contribution>` (optional, backward-compatible) to `PluginManifest`.
- `apps/tauri-windows/libs/vortideck-core/src/plugins/contributions.rs` (new) — `Contribution` enum + per-type config:

```rust
pub enum Contribution {
    Code { runtime: PluginRuntime, entry: EntryConfig },   // today's `runtime`+`entry`
    FlowNode { node: NodeSpec },
    Widget { capabilities: WidgetCapabilities, content: String },
    UiPanel { panel: String },
    Macro { macro_spec: MacroSpec },
}

pub struct WidgetCapabilities {
    pub subscribe: Vec<String>,          // "follow", "chatMessage", ...
    pub actions:   Vec<String>,          // "triggerFlow:<id>", "openUi", ...
    pub ui: bool,
    pub network:   NetworkCapability,    // None | Whitelist(Vec<String>) | Full
}
pub enum NetworkCapability { None, Whitelist(Vec<String>), Full }
```

- `layers/desktop-streaming/composables/overlay/widgetManifest.ts` (new) — TS mirror + validators (contribution list + widget capabilities).

**Backward compat:** `contributions` is `#[serde(default)]`; legacy manifests with `runtime`/`web` still parse (treated as a single `Code` + `UiPanel`/`Widget` contribution).

**Done when:** a package manifest (single-widget OR multi-contribution) parses/validates in Rust + TS; legacy manifests still load; `cargo check` + TS clean.

---

## Phase 1 — Widget = plugin (permission integration)

Wire widgets into the existing permission system.

**Files:**
- `apps/tauri-windows/libs/vortideck-core/src/plugins/permissions.rs` — extend `PluginPermission` with widget-specific entries if needed (`SubscribeEvent(String)`, `TriggerAction(String)`, `OpenUi`, `Network` already exists).
- `apps/tauri-windows/src-tauri/src/commands/widget_commands.rs` (new) — `install_widget(manifest)`, `revoke_widget(id)`, `get_widget_permissions(id)`.
- `main.rs` — register commands.

**Behavior:**
- `install_widget` validates manifest → calls `PermissionChecker.grant_permissions(widget_id, manifest.permissions)` → stores manifest + grants (DB or store) → returns the widget for the editor to add.
- Grants keyed by widget id, just like flow plugins.

**Done when:** a widget's grants persist and `has_permission`/`enforce_permission` work for widget ids.

---

## Phase 2 — Per-widget sandbox + CSP

Render untrusted widgets with a stricter environment; drive CSP from the `network` capability.

**Files:**
- `layers/desktop-streaming/composables/overlay/useOverlayV3Sync.ts` — in `generateElementHTML`, select sandbox flags + CSP per widget trust:
  - `trust: builtin` → `sandbox="allow-scripts allow-same-origin"` (current behavior).
  - `trust: verified | community` → `sandbox="allow-scripts"` (no `allow-same-origin`) **+** inject a `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data: https:; connect-src <from network cap>">` into the srcdoc `<head>`.

**CSP from capability:** `network: None → connect-src 'none'`; `Whitelist(urls) → connect-src <urls>`; `Full → connect-src *`.

**Done when:** a community widget cannot reach `window.parent` (opaque origin) and cannot `fetch()` an external URL when `network: None`; a builtin widget is unaffected.

---

## Phase 2.5 — Widget registry + dev-folder hot-reload

Widgets aren't registered today (they're hardcoded schemas in `componentSchema.ts`). Register them as packages (`widget` contribution) and enable a dev authoring loop reusing the existing `PluginWatcher` infra.

**Files:**
- `apps/tauri-windows/libs/vortideck-core/src/widgets/registry.rs` (new) — scans a widgets folder, parses package manifests, registers `widget` contributions (mirrors the plugin loader).
- `apps/tauri-windows/libs/vortideck-core/src/plugins/watcher.rs` — extend `is_plugin_file` + handlers to recognize widget packages and route to the widget registry (reuse the same `notify` plumbing; don't duplicate it). On change: re-read manifest → regenerate the widget's `render_html` → `syncElementsToBackend` → overlay diff swaps the iframe `srcdoc` → widget reloads.
- `main.rs` — point a widgets dir (+ an optional `widgets-dev` dir, auto-trusted only when dev mode is on) at the watcher; push updated widget defs to the editor on registry change.

**Dev loop (the payoff):** edit a widget package in the dev folder → watcher fires → widget reloads **live in editor + OBS**. This *is* the real-time preview for code-first authoring — no separate preview engine; the runtime is the preview.

**Reuse vs new:** reuses `PluginWatcher` (notify + folder watch + events), `syncElementsToBackend`, and the fetch/diff engine. New code = widget manifest loader + a "widget changed → regenerate render_html" handler.

**Done when:** dropping/editing a widget package in the dev folder reloads it live (editor + OBS) without an app restart; production widgets load from the standard widgets folder.

---

## Phase 3 — Capability-filtered bridge (inbound + redaction)

Stop broadcasting everything to every widget; deliver only granted events, redacted.

**Files:**
- `apps/tauri-windows/src-tauri/src/main.rs` (overlay bridge task) — when forwarding an event, look up each overlay client's widget grants; skip widgets that didn't `subscribe` to that event type; redact sensitive fields per a small redaction map (e.g., strip `message`/`email` from donation payloads unless granted).
- `layers/desktop-streaming/composables/overlay/useOverlayV3Sync.ts` — attach the widget's `subscribe` list to its sync payload so the backend knows each widget's grants.

**Done when:** a widget that didn't request `chatMessage` never receives it; a follow-only widget doesn't get raid/bits.

---

## Phase 4 — Capability Dialog window ⭐ (the centerpiece)

The modal the user sees when adding a widget. **Detailed spec at the bottom of this file.**

**Files:**
- `layers/desktop/components/Modal/CapabilityDialog.vue` (new) — the dialog component.
- `layers/desktop-streaming/pages/streaming/overlays/index.vue` — open the dialog from the "Add widget" flow; on grant, call `install_widget`.
- `layers/desktop-streaming/components/overlay/OverlayRightPanel.vue` (or wherever the widget library lives) — trigger the dialog on add.

**Done when:** adding a marketplace widget shows the dialog; approving calls `install_widget` and adds the widget; canceling aborts.

---

## Phase 5 — Permission management UI + runtime prompts

- `layers/desktop/components/Modal/WidgetPermissions.vue` (new) — list each installed widget + its grants with revoke toggles (browser-extension style).
- Global entry: a "Widgets / Extensions" page or dashboard card.
- Runtime prompt: a lightweight modal when a widget first attempts an un-granted capability (Allow once / Always / Deny), surfaced from the gated action handler.

---

## Phase 6 — Two-way outbound gating

Let widgets call out, checked by `PermissionChecker`.

**Files:**
- `useOverlayV3Sync.ts` (iframe bridge script) — accept an outbound message type `WIDGET_ACTION` from the iframe.
- `main.rs` / `widget_commands.rs` — on `WIDGET_ACTION`, `PermissionChecker.enforce_permission(widget_id, action)`; execute if granted, else drop (and optionally trigger the runtime prompt from Phase 5).

---

## Phase 7 — Widen the bridge (behind capabilities)

With gating in place, forward more categories so widgets can react to the whole engine:
- `Flow` (all variants, not just `FlowCompleted`)
- `System` (VolumeChanged, SystemInfo)
- `Webhook` (WebhookReceived)
- `Relay` (mobile status/commands)

All filtered through Phase 3's capability check.

**Files:** `main.rs` overlay bridge match arms — replace the `_ => {}` drops with capability-filtered forwarding.

---

## Phase 8 — Flow "Emit Event" node + macros on bus

- `apps/tauri-windows/libs/vortideck-core/src/services/flow/nodes/emit_event.rs` (new) — a flow node that publishes `EventPayload::Generic { source: "flow:<id>", eventType, payload }` (or a typed event) to the bus.
- `services/execution/*` — emit `MacroStarted`/`MacroCompleted` lifecycle events (id + name, **never inputs**) to the bus.

This closes the engine loop (flows/macros become producers).

---

## Phase 9 — SDK / contract extension

- `packages/plugin-sdk` — add a `widget` module: typed event types, `widget.subscribe()`, `widget.trigger()`, theming var reference, manifest authoring helpers.
- `docs/WIDGET_SDK.md` (new) — the creator-facing contract.

---

## File-by-file change list (summary)

| File | Phase | Change |
|---|---|---|
| `libs/vortideck-core/src/plugins/bundle.rs` | 0 | Extend `PluginManifest` with `contributions` list |
| `libs/vortideck-core/src/plugins/contributions.rs` | 0 | New — `Contribution` enum + `WidgetCapabilities` |
| `libs/vortideck-core/src/plugins/permissions.rs` | 1 | Extend `PluginPermission` for widget caps |
| `libs/vortideck-core/src/widgets/registry.rs` | 2.5 | New — widget package loader/registry |
| `libs/vortideck-core/src/plugins/watcher.rs` | 2.5 | Extend to route widget packages to the registry (reuse `notify`) |
| `src-tauri/src/commands/widget_commands.rs` | 1, 6 | New — install/revoke/gate |
| `src-tauri/src/main.rs` | 1, 2.5, 3, 6, 7 | Register cmds; widgets/dev-folder watcher wiring; capability-filtered bridge; outbound gating; widen forwarding |
| `composables/overlay/useOverlayV3Sync.ts` | 2, 3, 6 | Per-widget sandbox+CSP; subscribe payload; WIDGET_ACTION bridge |
| `composables/overlay/widgetManifest.ts` | 0 | New — TS manifest mirror |
| `components/Modal/CapabilityDialog.vue` | 4 | New — the dialog |
| `components/Modal/WidgetPermissions.vue` | 5 | New — manage/revoke |
| `pages/.../overlays/index.vue` | 4, 5 | Open dialog on add; entry to permissions |
| `libs/vortideck-core/src/services/flow/nodes/emit_event.rs` | 8 | New — emit node |
| `libs/vortideck-core/src/services/execution/*` | 8 | Macro lifecycle events |
| `packages/plugin-sdk` | 9 | Widget module |

---

## ⭐ Capability Dialog — full spec (Phase 4)

### Purpose
The trust-establishing moment when a user adds a widget. Surfaces exactly what the widget wants, in plain language, before any code runs.

### Trigger — when the dialog appears

**Core rule:** the dialog appears only when a widget is about to run with capabilities this user hasn't approved yet — not on every render, not on restart.

| Moment | Dialog? | What happens |
|---|---|---|
| First **Add** (library/marketplace), `trust ≠ builtin` | ✅ Full install dialog | The trust moment — review all capabilities before it runs |
| First **Add**, `trust = builtin` | ❌ Silent | Ships with the app, fully trusted |
| App **restart / project load**, widget already in grant store | ❌ Silent | Already approved — grant persists, widget just renders |
| **Loading a shared/imported project** with a widget not in your grant store | ✅ Full install dialog | Effectively a first install on this machine |
| Widget **update**, no capability change | ❌ Silent (optional "Updated" toast) | Nothing new to decide |
| Widget **update that adds capabilities** | ✅ Diff dialog ("v2 now also wants X") | New capability = new decision |
| Widget **attempts an un-granted action at runtime** | ⚠️ Lightweight runtime prompt (Allow once / Always / Deny) — *not* this modal | Different, smaller UX mid-use |
| User opens the **Permissions/Extensions panel** | ❌ (management UI, not a dialog) | Review/revoke, user-initiated |

**Exact pop-up moment:** at the *Add* action (clicking Add in the widget library/marketplace), **before** the widget lands on the canvas:
`click Add → dialog → grant → widget appears on canvas` (or `cancel → nothing added`).

Three distinct surfaces (don't conflate): **(1) install dialog** (modal, full review) at add-time / capability-changing updates / un-granted shared widgets; **(2) runtime prompt** (inline, one capability) when a widget tries something mid-use; **(3) management UI** (panel) for user-initiated review/revoke. All are **editor-only** — in OBS a widget with missing grants silently degrades (no data, no action), never prompts the viewer.

**Implementation note:** "widget already in grant store" = `PermissionChecker.get_granted_permissions(widget_id)` is non-empty **and** covers the manifest's current capabilities. If a grant exists but the manifest now requests more (update), show the diff dialog for only the new capabilities.

### Folder-loaded widgets (side-loading on start)

**Principle: folder presence is not consent.** A widget file appearing in the scanned folder does **not** mean the user approved it — a malicious process, sync, or another app could drop a file there. So folder-loading **discovers** widgets; it never **runs**, **renders**, or **grants** them.

On start, the app scans the widgets folder and for each widget:
1. **Registers it in the library** as "available" — but does **not** render it, subscribe it, or execute its code.
2. **Checks the grant store:**
   - **Already granted** (capabilities match) → runs normally when used in a project (no dialog — restart case).
   - **Not granted, or capabilities changed** → flagged "needs review," stays inert.
3. Optional non-blocking toast: *"N new widgets found in your library"* (discoverability, not a prompt).

The widget never executes — not even sandboxed rendering — until the user **Adds** it (→ dialog → grant) or **loads a project referencing it** while un-granted (→ dialog). The dialog still gates it; the trigger is the user's action, not the file's existence.

**Escape hatches:**
- **Dev folder** — a designated local path (e.g. `%APPDATA%/VortiDeck/widgets-dev/`) auto-trusted *only when dev mode is enabled*. For widget authors iterating locally. Off by default.
- **Builtin widgets** — ship signed with the app (`trust: builtin`), no dialog ever (separate path from folder side-loading).

Folder widgets without marketplace attestation default to `trust: community` → strictest sandbox + CSP when they eventually run.

| Folder scenario | Behavior |
|---|---|
| New file appears, never added | Discovered in library, flagged "needs review", **never runs** |
| File present, user clicks Add | Install dialog → grant → runs |
| File present + granted, app restarts | Runs silently (grant persists) |
| File present, manifest updated with new caps | Diff dialog on next Add/use |
| Project references a folder widget not granted | Install dialog on project load |
| File in dev folder + dev mode on | Auto-trusted (dev convenience) |

### Layout (ASCII mockup)

```
┌──────────────────────────────────────────────────────┐
│  Add "Follower Goal"                          [ ✕ ]   │
│  by @streamerdevs                                    │
│  ✓ Verified creator        ★ 12,480 installs        │
│ ─────────────────────────────────────────────────── │
│                                                      │
│  This widget wants to:                               │
│                                                      │
│   ✓  see when someone follows you                    │
│   ✓  see when someone raids you                      │
│   ✓  trigger the "Celebration" automation            │
│   ✕  read chat messages          (not requested)     │
│   ✕  network access              (not requested)     │
│                                                      │
│  Runs sandboxed — it can't see the editor, other     │
│  widgets, or your accounts.                          │
│                                                      │
│              [ Cancel ]      [ Add widget ]          │
└──────────────────────────────────────────────────────┘
```

### Component contract

```ts
// layers/desktop/components/Modal/CapabilityDialog.vue
defineProps<{
  manifest: WidgetManifest           // id, name, creator, trust, capabilities
  installs?: number                  // marketplace signal (optional)
}>()

defineEmits<{
  (e: 'grant', approved: ApprovedCapabilities): void   // user clicked Add
  (e: 'cancel'): void
}>()
```

- **Plain-language mapper** — a `capabilityLabel(cap)` helper turns `"follow"` → "see when someone follows you", `"triggerFlow:flow_42"` → "trigger the Celebration automation", `"network"` → "network access". One place to keep human-readable.
- **Trust badge** — green ✓ for `verified`, gray for `community`, hidden for `builtin`.
- **Reassurance line** — fixed copy explaining sandboxing (builds trust).
- **Styling** — matches existing modals (`layers/desktop/components/Modal/*`): glassmorphic, Transition animations, `InfoDropdown`-style polish.

### Flow on `grant`
1. `CapabilityDialog` emits `grant` with the approved capabilities.
2. Parent calls `invoke('install_widget', { manifest, approved })`.
3. Backend: `PermissionChecker.grant_permissions(widget_id, …)`; persist manifest + grants; return the widget element.
4. Editor adds the widget to the canvas (sandbox+CSP applied per Phase 2).
5. Bridge begins delivering only subscribed events (Phase 3).

### Edge cases
- **Manifest requests a dangerous cap** (e.g., `network: full`, or keyboard-sim macro) → show an explicit high-risk callout + require a second confirm.
- **Update changes capabilities** → re-open the dialog showing the diff ("Follower Goal now also wants X").
- **Offline/local widget** (no marketplace metadata) → skip install count, show `community` trust, slightly stronger caution copy.

---

## Sequencing & dependencies

```
Phase 0 (manifest) ─► Phase 1 (permissions)
                 └──► Phase 2 (sandbox+CSP)
                 └──► Phase 4 (dialog)  ─► Phase 5 (manage UI)
Phase 1 + 3 ─► Phase 6 (two-way gating) ─► Phase 7 (widen bridge)
Phase 8 (emit/macro bus) — independent
Phase 9 (SDK) — last (documents the stabilized surface)
```

**Minimum viable security slice:** Phases 0 → 1 → 2 → 4. That alone makes a marketplace widget safe to install and render. Phases 3/6/7 deepen integration; 8/9 round out the engine + creator story.

---

## Open questions

1. **Where do widget grants persist?** DB table (consistent with sessions/messages) vs `tauri-plugin-store`. Recommend DB for queryability.
2. **Runtime prompt UX** — silent-deny vs browser-style prompt? Recommend prompt for first attempt, then remember.
3. **Redaction policy** — per-event field allowlists need a canonical map; start minimal (strip free-text `message`/`email` unless explicitly granted).
4. **Marketplace trust attestation** — how is `verified` assigned? Out of scope here, but the manifest `trust` field presumes a signing/attestation flow exists.
