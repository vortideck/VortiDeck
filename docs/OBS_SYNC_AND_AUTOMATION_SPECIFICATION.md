# OBS Sync, Overlay UI & GitHub Automation Specification

This specification documents the architecture, UX design, and automation pipelines for VortiDeck's OBS Sync engine, Overlay Editor UI (Desktop & Mobile), and GitHub Release Automation workflows.

---

## 1. OBS Sync & Realtime Messaging Architecture

### 1.1 Overview & Data Flow
VortiDeck operates as the central control plane for streaming overlays. When an element is added, moved, styled, or edited in the canvas editor, state updates are broadcast in real-time to two targets
1. OBS Studio C++ Plugin (`obs-plugin`) Communicates via local IPCWebSocket for direct native banner rendering.
2. OBS Browser Source (`overlayv3`) Renders HTMLCSS templates inside OBS Browser Sources with zero latency.

```mermaid
flowchart TD
    SubGraph1[VortiDeck Editor Canvas] --Reactive State Change SyncEngine[v3Sync  useOverlayV3Sync]
    SyncEngine --Debounced WebSocket Event TauriBridge[Tauri IPC  Rust Server]
    TauriBridge --Local Server Broadcaster OBSPlugin[Native C++ OBS Plugin]
    TauriBridge --WebSocket Broadcast OBSBrowser[OBS Browser Source iframe]
```

### 1.2 Synchronization Settings
 Auto-Sync Toggle Automatically transmits element positioning and property changes as the user drags or edits properties.
 Debounce Window 16ms (60 FPS frame boundary) for spatial drag coordinates, 150ms for colorfont property changes.
 Initial State Handshake When an overlay client connects or reloads, VortiDeck automatically sends the full canvas state (`initialSync`).

---

## 2. Overlay UI & Component Property Management (Desktop & Mobile)

### 2.1 Top Bar & Floating `+` Plus Action Button
To improve mobile and desktop usability on smaller viewports, the Overlay Editor header features a dedicated `+` Plus Action Button on the top-right toolbar.

#### UI Placement
 Desktop Top Header (`OverlayHeader.vue`) Positioned next to the OBS Sync and Settings icons on the top-right toolbar.
 Mobile Floating Action Button (`OverlayCanvas.vue`) Positioned as a floating backdrop-blur `+` button on the top-right or bottom-right of the canvas.

#### Interaction Flow
```
[ Top Right + Button ] ──► Opens [ Add Element  Component Creator Modal ]
                                  ├── Category Tabs (Basic, Widgets, Graphics, Alerts, Effects)
                                  ├── Search & Filter Bar
                                  └── One-Click Insert onto Canvas
```

---

### 2.2 Overlay Settings Modal (`OverlaySettingsModal.vue`)
Clicking the gear icon next to the `+` button opens the central Overlay & OBS Settings Modal

1. OBS Connection Settings
    Local Server Port (default `4450`)
    WebSocket Connection Status Indicator (`Connected`  `Disconnected`)
    Auto-Sync Toggle (`On`  `Off`)
    Copy Browser Source URL (`httplocalhost4450overlayv3`)
2. Canvas & Editor Preferences
    Grid Lines Toggle (`Show`  `Hide`)
    Snap to Grid (`On`  `Off`)
    Safe Zones Overlay (`169 Stream Safe Zone`, `Vertical 916 Zone`)
    Render Quality & Frame Rate Limit (30 FPS  60 FPS)
3. Theme & Styling Presets
    Global Theme Picker (Dark Glass, Cyberpunk Neon, Minimal Stream, Solarized)
    Custom Theme Variable Overrides (`--theme-primary`, `--theme-accent`, `--theme-radius`)

---

### 2.3 Element Property System (`componentSchema.ts` & `elementRegistry.ts`)
Every element placed on the canvas is backed by a formal Component Schema

```typescript
export interface ComponentSchema {
  id string                  Unique key ('follower', 'chat', 'goal')
  name string                Display name ('Follower Goal')
  category 'basic'  'widget'  'graphic'  'alert'  'effect'
  width number
  height number
  template string            HTML template with var(--c-) and {data.} placeholders
  cssVars Recordstring, CssVarDef         Schema-defined CSS variables
  editableProps EditableProp[]              Controls shown in Right Properties Panel
  defaultStyle Recordstring, string
  defaultData Recordstring, string  number
}
```

#### Property Edit Pipeline
1. User adjusts a control in Right Properties Panel (`OverlayRightPanel.vue`) (e.g. font size, fill color, text title).
2. The property value updates `element.style` or `element.data`.
3. `buildComponentVars(schema, element.style)` resolves element CSS variables (`--c-`) into concrete values.
4. `useOverlayV3Sync` broadcasts the updated HTMLstyles to OBS immediately.

---

## 3. GitHub Release & CI Automation Architecture

### 3.1 `git-cliff` Changelog Automation
`git-cliff` generates structured release changelogs during GitHub Actions builds. To ensure consistency between Desktop (`desktop-v`) and Mobile (`mobile-v`) releases

 Tag Scoping
   `desktop-cliff.toml` ➔ `tag_pattern = desktop-v.`
   `mobile-cliff.toml` ➔ `tag_pattern = mobile-v.`
 Commit Grouping Rules
   `desktop-feat`, `both-feat`, `feat` ➔ 🚀 Features
   `desktop-fix`, `both-fix`, `fix` ➔ 🐛 Bug Fixes
   `desktop-refactor`, `both-refactor`, `refactor` ➔ 🚜 Refactor
   `desktop-perf`, `both-perf`, `perf` ➔ ⚡ Performance

---

### 3.2 Build Verification & Version Guard (`checktauri-versions`)
To prevent mismatched NPM package versions (`@tauri-appsapi`, `@tauri-appsplugin-log`) and Rust crate versions (`tauri`, `tauri-plugin-log`) from breaking builds
 `scriptscheck-tauri-versions.js` validates version symmetry between `appspackage.json`, `pnpm.overrides`, and `Cargo.toml`.
 Automated step in `.githubworkflowsdesktop-release.yml` and `.githubworkflowsmobile-release.yml`
  ```yaml
  - name Verify Tauri versions
    run pnpm checktauri-versions
  ```

---

### 3.3 Automated Failure Diagnostics & Push Notifications
When a release build fails on a GitHub Actions runner, an automated failure triage step triggers using `if failure()`

```yaml
- name 🚨 Auto-Diagnose & Send Failure Alert
  if failure()
  uses actionsgithub-script@v7
  env
    DISCORD_WEBHOOK_URL ${{ secrets.DISCORD_WEBHOOK_URL }}
  with
    script 
      const logs = await github.rest.actions.downloadJobLogsForWorkflowRun({
        owner context.repo.owner,
        repo context.repo.repo,
        job_id context.job,
      });
       Extract error lines & dispatch instant push notification to DiscordTelegram
```

---

## 4. Summary Table of Architecture Components

 Component  Responsibility  Primary Files 
 ---  ---  --- 
 OBS Sync Engine  Broadcasts spatial & style state in real time to OBS clients  `useOverlayV3Sync.ts`, `html.rs` 
 Overlay Editor UI  Canvas interaction, floating `+` button, top toolbar controls  `OverlayHeader.vue`, `OverlayCanvas.vue`, `index.vue` 
 Property Panel  Dynamic schema-driven property editor for colors, text, sizes  `OverlayRightPanel.vue`, `componentSchema.ts` 
 Settings Modal  Manages WebSocket ports, auto-sync, grid snap, and themes  `OverlaySettingsModal.vue` (proposed), `OverlayModal.vue` 
 Changelog Engine  Tag-isolated conventional commit changelogs  `desktop-cliff.toml`, `mobile-cliff.toml` 
 Version Guard  Enforces version alignment between Rust crates & NPM packages  `check-tauri-versions.js`, release workflows 
