# Overlay Editor — Library UX Redesign

**Status:** Design (ready to implement as the final phase)
**Scope:** The element library in the overlay editor's left sidebar — how elements are organized, discovered, and added.
**Problem:** The current 4-tab structure (Theme / Elements / Widgets / Alerts) is messy: overlapping concepts split across tabs, no search, Theme mixed with elements, effects homeless.
**Goal:** A unified, searchable, category-filtered library that scales to builtin + community + marketplace widgets.

---

## 1. What's wrong today

### Overlapping concepts split across tabs
| Concept | Elements tab | Widgets tab | Alerts tab |
|---|---|---|---|
| **Follower** | "Gradient Follower Counter" | "Goal Bar", "Counter" | "New Follower" alert |
| **Webcam** | "Webcam Frame" | "Webcam" | — |
| **Counter** | "Follower Counter Gradient" | "Counter" | — |
| **Banner** | "Banner" (Stream Graphics) | — | — |

A user looking for "follower" must check **3 tabs**. Same for "webcam" (2 tabs). There's no search to bridge them.

### Other issues
- **No search bar** — discovery requires knowing which tab an element is in.
- **Theme is a library tab** — but it's configuration (colors/fonts), not an element to drag onto the canvas. Mixing config with the element library is confusing.
- **Effects are homeless** — componentSchema has fireballs, confetti, etc., but they don't fit neatly into Elements/Widgets/Alerts. No clear home.
- **Fuzzy tab boundaries** — "banner" is in Elements but behaves like a live widget. "Timer" is in Widgets but is utility/text-like (no streaming data). The boundary between "Element" and "Widget" is arbitrary.
- **No room for community widgets** — adding a 5th tab ("Community") would help but doesn't fix the structural mess.

### Root cause
The tabs mix **source** (builtin), **format** (element/widget/alert), and **behavior** (static/live/event) inconsistently. And they HIDE related items from each other.

---

## 2. Proposed redesign: unified searchable library

Replace the 4 tabs with a **single library panel** that has:
1. **Search bar** (primary discovery).
2. **Category filter chips** (multi-select, optional).
3. **Grid of element cards** (filtered by search + chips).

### Layout

```
┌─────────────────────────────────────────────────────┐
│  🔍 Search elements...                               │
│                                                      │
│  [✓ All] [Static] [Live Data] [Alerts] [Effects]    │
│  [Media] [Community]                                 │
│                                                      ││  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐       │
│  │ 📝 Text│ │ 💬 Chat│ │ ❤️ Foll│ │ 🔥 Fire│       │
│  │ Static │ │ Live   │ │ Alert  │ │ Effect │       │
│  └────────┘ └────────┘ └────────┘ └────────┘       │
│  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐       │
│  │ 📊 Goal│ │ 🎥 Cam │ │ 💰 Don │ │ 🧩 Plug│       │
│  │ Live   │ │ Media  │ │ Alert  │ │ Comm.  │       │
│  └────────┘ └────────┘ └────────┘ └────────┘       │
│                                                      │
│  ⭐ Recently Used: [Chat] [Goal] [Text]              │
└─────────────────────────────────────────────────────┘
```

Theme moves to a **separate toolbar button** (paintbrush icon) → opens theme browser/settings modal. It's configuration, not an element.

---

## 3. Category taxonomy

Based on **what the element does** (behavior), not arbitrary groupings:

| Category | Description | Elements (current → new home) |
|---|---|---|
| **Static** | No data, no events. Just visual. | text, image, shape, video, logo, social bar |
| **Live Data** | Persistent, shows real-time stream data. | chat, goal bar, counter, timer, recent events, follower counter, banner |
| **Alerts** | Event-triggered popups (appear briefly). | follower alert, sub alert, raid, donation, host |
| **Effects** | Visual WebGL/canvas simulations. | fireballs, confetti, particles, glow |
| **Media** | Camera/video capture or embed. | webcam, webcam frame, browser source |
| **Community** | Third-party plugin widgets (sandboxed). | from `list_available_widgets()` |

### Why these categories
- They map to **user intent**: "I want to show my chat" → Live Data. "I want a follow alert" → Alerts. "I want something flashy" → Effects.
- They're **mutually exclusive** (each element has ONE primary category — no overlap confusion). An element like "follower counter" is Live Data (it shows a persistent count), NOT also an Alert (that's a separate element — the popup).
- They **scale**: Community and future categories (Marketplace) fit as new chips without restructuring.

---

## 4. Element re-categorization (the merge)

Flatten the current 4 lists (builtinCategories, streamWidgets, streamAlerts + componentSchema effects) into ONE array with `category` + `tags`:

```ts
interface LibraryElement {
  id: string                    // schema id (e.g., "chat", "follower", "webgl-fireballs")
  name: string                  // display name
  icon: string                  // heroicon id
  description: string
  category: LibraryCategory     // Static | Live Data | Alerts | Effects | Media | Community
  tags: string[]                // for search (e.g., ["follower", "counter", "stream"])
  trust?: 'builtin' | 'community'  // only 'community' shown (badge on card)
  source: 'builtin' | 'community'  // determines add flow (direct vs dialog)
}
```

### Current → new mapping

| Current location | Element | New category | Tags |
|---|---|---|---|
| Elements / Basic | text | Static | text, label |
| Elements / Basic | image | Static | image, picture |
| Elements / Basic | shape | Static | shape, rectangle, circle |
| Elements / Basic | video | Media | video, media, player |
| Elements / Stream Graphics | webcam-frame | Media | webcam, camera, frame |
| Elements / Stream Graphics | banner | Live Data | banner, ticker, scrolling |
| Elements / Stream Graphics | logo | Static | logo, watermark, brand |
| Elements / Stream Graphics | social | Static | social, twitter, instagram |
| Elements / Stream Graphics | follower-counter-gradient | Live Data | follower, counter, count |
| Widgets | chat | Live Data | chat, messages, twitch |
| Widgets | goal | Live Data | goal, progress, follower, sub |
| Widgets | recent | Live Data | recent, events, activity |
| Widgets | webcam | Media | webcam, camera |
| Widgets | timer | Live Data | timer, countdown |
| Widgets | counter | Live Data | counter, number |
| Alerts | follower | Alerts | follower, follow, alert |
| Alerts | subscriber | Alerts | subscriber, sub, alert |
| Alerts | donation | Alerts | donation, tip, alert |
| Alerts | raid | Alerts | raid, alert |
| Alerts | host | Alerts | host, alert |
| componentSchema | webgl-fireballs | Effects | fireball, physics, particle |
| componentSchema | (confetti, etc.) | Effects | confetti, particle, celebration |
| componentSchema | glow | Effects | glow, aura |
| Registry | (community widgets) | Community | (from manifest tags) |

---

## 5. Card design

```
┌──────────────────────────────┐
│  💬  Chat Box                 │
│  Live chat display            │
│  Live Data                    │
│                  [click/drag] │
└──────────────────────────────┘
```

For Community cards (additional elements):
```
┌──────────────────────────────┐
│  🧩  Follower Goal     [🛡️]  │  ← trust badge (Community)
│  by @streamerdevs             │
│  ✓ follows · 🌐 network       │  ← capability summary
│  Community                     │
│                  [Add widget] │  ← triggers CapabilityDialog
└──────────────────────────────┘
```

### Interaction
- **Builtin** (Static/Live/Alerts/Effects/Media): click or drag → add directly to canvas (trusted, no dialog).
- **Community**: click → `useCapabilityDialog().open(manifest)` → CapabilityDialog → `install_widget` → add to canvas (sandboxed).

---

## 6. Search mechanics

- Search matches `name`, `tags`, `description` (case-insensitive, fuzzy).
- Type "follower" → shows: follower counter (Live Data), goal bar (Live Data), follower alert (Alerts), gradient follower counter (Live Data). ALL variants in one list.
- Type "webcam" → shows: webcam (Media), webcam frame (Media).
- Category chips filter IN ADDITION to search (both apply: search "follower" + chip "Alerts" → only the follower alert).

---

## 7. Implementation plan (incremental — no rebuild)

### Step 1: Flatten + re-tag (data)
Merge `builtinCategories`, `streamWidgets`, `streamAlerts` + componentSchema effects into ONE `libraryElements: LibraryElement[]` with `category` + `tags`. (~1 hour — the data exists, just reorganize.)

### Step 2: Search bar
Add a search input to the left sidebar. Filter `libraryElements` by name/tags/description. (~30 min.)

### Step 3: Category chips
Replace the 4 tabs with category filter chips (All / Static / Live Data / Alerts / Effects / Media / Community). Multi-select. (~1 hour.)

### Step 4: Theme → separate button
Move Theme out of the library tabs. Add a paintbrush icon button (toolbar or sidebar header) → opens theme browser/settings. (~30 min.)

### Step 5: Community category
Populate from `list_available_widgets()` (Phase 2.5 backend, done). Cards show trust badge + capability summary. "Add" → CapabilityDialog (Phase 4, done). (~1 hour.)

### Step 6: Recently Used / Favorites
Track added elements. Show a "Recently Used" row at the top of the library. (~30 min.)

**Total: ~4-5 hours of focused frontend work. No backend changes. No element schema changes. Pure presentation reorganization.**

---

## 8. What NOT to change

- **Element schemas** (componentSchema) — unchanged. The schemas define rendering/properties, not library organization.
- **Element rendering** (generateElementHTML, useOverlayV3Sync) — unchanged. How elements render on canvas is separate from how they're listed in the library.
- **Element sync** (batch_sync_overlay, install_widget) — unchanged.
- **Capability system** (Phases 0–4) — unchanged. The library just surfaces community widgets; the capability system secures them.

The redesign is purely the **library presentation layer** — search + categories + cards. The data and rendering pipelines are untouched.

---

## 9. Session context

This redesign is the **last phase** of the overlay improvement work started in 0.7.0. The preceding phases built:
- ✅ Z-index/layering fixes (bring to front/back/forward/backward + OBS sync)
- ✅ Chat in OBS (widget stream-event listener + typed history replay)
- ✅ Live theme system (cascade, no reload, persistence, all-iframe, out-of-the-box theming)
- ✅ Widget capability system (Phases 0–4: contract, permissions, sandbox/CSP, registry, global dialog)
- ✅ Architecture audit + planning docs

The library redesign is the **user-facing capstone** — it makes all the underlying improvements (community widgets, capability system, sandboxing) discoverable and organized for the user.
