# Widget & Plugin Creation — Assessment & Direction

**Status:** Investigation + recommendation (pre-decision)
**Triggered by:** questions about the properties panel (text lacks background), the `ComponentCreator` prototype, whether a separate plugin-creation app makes sense, and whether the SDK needs changes for the capability work.
**Context:** This is a direction-setting doc. After it, we return to the capability/dialog plans (`WIDGET_CAPABILITY_ARCHITECTURE.md`, `WIDGET_CAPABILITY_DIALOG_PLAN.md`).

---

## TL;DR

- The **properties system is healthy** — schema-driven and generic; "text has no background" is a deliberate schema choice, not a bug. It just needs a shared "common props" library so authors don't hand-pick per element.
- The **`ComponentCreator` is a broken prototype** (save/preview are `console.log` stubs). The *concept* (visual builder) is fine for simple widgets, but it was built **before a stable widget contract exists** — that's why it stalled.
- The **SDK needs extending** for widgets — today it only covers flow plugins.
- **Recommended direction:** stabilize the **widget contract** first (manifest + capabilities + SDK + bridge filtering — exactly the capability plans we made). The visual creator is a layer *on top* of that contract, built once it's stable. A **separate plugin-creation app is not recommended now** (fragmentation); revisit only post-marketplace if creation grows IDE-complex.

---

## 1. Properties system — integrity: GOOD

### How it works
Each element schema in [componentSchema.ts](../layers/desktop-streaming/composables/overlay/componentSchema.ts) declares `editableProps: EditableProp[]`. The right panel ([OverlayRightPanel.vue](../layers/desktop-streaming/components/overlay/OverlayRightPanel.vue)) is **generic** — it splits them into:
- **`dataProps`** (have `dataProp`) → bound to `element.data`.
- **`styleProps`** (have `styleProp`) → bound to `element.style` (and mapped to a `--c-*` cssVar where applicable).

Supported types: `text`, `number`, `color`, `range`, `font`, `select`. The panel renders whatever the schema declares — it is **not** hardcoded per element.

### "Text has no background" — intentional, not a bug
The `text` schema ([componentSchema.ts:121](../layers/desktop-streaming/composables/overlay/componentSchema.ts#L121)) declares only: textColor, font, fontSize, fontWeight, textAlign. Its `defaultStyle` sets `backgroundColor: 'transparent'`, `borderStyle: 'none'`. So text is designed as **transparent text** — no bg control. Other elements (chat, goal bar) declare bg/radius/border because their schemas list those props.

The system fully supports adding background to text — someone just has to add `bgColorProp` to its `editableProps`. So the question isn't "is it broken," it's "what should each element expose?"

### Recommendation: a shared common-props library
Today each schema hand-picks from scattered prop constants (`bgColorProp`, `radiusProp`, …). Tighten this:
- A **`COMMON_PROPS` set** (`background`, `border`, `radius`, `opacity`, `shadow`, `padding`) that schemas **compose** (`editableProps: [...COMMON_PROPS, textColorProp, ...]`), so the property surface is consistent and elements don't each reinvent it.
- The **widget author (or creator) declares `editableProps`** — that *is* where properties are "added." No separate mechanism needed; the schema is the source of truth.

This is a cleanup, not a redesign. The architecture is correct.

---

## 2. `ComponentCreator` — integrity: BROKEN PROTOTYPE

### What it is
A "Pro Feature" modal ([ComponentCreator.vue](../layers/desktop-streaming/components/overlay/ComponentCreator.vue)) — a visual builder: component library, building blocks, templates, a canvas, Preview + Save buttons. Aspires to be a no-code widget composer.

### State (why it's "broken")
- `previewComponent()` → `console.log("Component Preview:", …)`. ([L810](../layers/desktop-streaming/components/overlay/ComponentCreator.vue#L810))
- `saveComponent()` → `console.log`s + `emit("save", componentDefinition)`. ([L859](../layers/desktop-streaming/components/overlay/ComponentCreator.vue#L859))

Nothing **persists** a widget, nothing **renders** the composed component as a real overlay widget, and `componentDefinition` has no path to a manifest/schema the runtime understands. It's a UI shell over stubs.

### Why it stalled (the real diagnosis)
It was built **before a stable widget contract existed** — there's no widget manifest, no schema-persistence format, no capability/permission model, no SDK surface for widgets. So the creator has nothing to *target*: it can compose blocks in memory but can't produce a runnable, installable widget. **Building a visual creator on an unstable foundation is exactly how you get a permanent prototype.**

### Verdict
The **concept** (visual composition for simple widgets) is legitimate and useful for non-coders. But the prototype is premature. **Park it** until the widget contract (below) is stable, then rebuild the creator *on top of* that contract — at which point "Save" produces a real manifest and the widget runs.

### Current UI state (action taken)
The creator entry point in the left rail ([OverlayLeftSidebar.vue](../layers/desktop-streaming/components/overlay/OverlayLeftSidebar.vue)) is now **disabled** — greyed, non-clickable (`disabled`), "Coming soon" tooltip, with a re-enable comment. A broken feature shouldn't be reachable; this keeps it visible as a coming feature without the broken experience.

### Must-have for the rebuilt creator: real-time preview
A hard requirement for the rebuild is **live preview** — as the user composes blocks / edits props, the widget renders in real time (in the editor canvas, ideally also mirrored to OBS) exactly as it'll appear in production. "Compose → see it live → Save as manifest." This is what makes a visual creator worth using (and is the natural payoff of building on a stable contract: the preview *is* just the widget runtime rendering the in-progress manifest).

---

## 3. Creation options — visual creator vs code SDK vs dedicated app

| Option | Best for | Pros | Cons |
|---|---|---|---|
| **In-app visual creator** (ComponentCreator, rebuilt) | Simple widgets (text, shapes, layouts, stat cards) | No-code; fast for trivial widgets; in-context | Limited for complex/interactive/WebGL widgets; big UI to maintain |
| **Code SDK + manifest** (the capability path) | Real/marketplace widgets (chat, effects, interactive, custom logic) | Full power; anything possible; versionable; the marketplace substrate | Requires dev skill |
| **Dedicated "Studio" app** | Hypothetical heavy authoring (visual + code + debug + publish) | Focused environment; room to grow | Fragments experience; doubles maintenance; overkill until volume justifies |

### Recommendation: layered, contract-first
1. **Code SDK + manifest first.** This is the foundation — it's what the capability system requires *anyway*, and it's the marketplace substrate. Without it, neither the creator nor the marketplace can work.
2. **Visual creator second**, rebuilt on the stable contract, scoped to **simple widgets** (compose blocks → emits a valid manifest → runs). This is the no-code layer; it does not need to handle WebGL/interactive — those use the code path.
3. **Dedicated app: not now.** An in-app creator + code SDK covers creation without fragmenting the product or doubling maintenance. Revisit **only after** a marketplace exists *and* authoring demand grows to justify an IDE-like tool. That's a post-marketplace decision.

---

## 4. SDK — does the capability work require SDK changes? **Yes.**

Today `@vortideck/plugin-sdk` ([index.ts](../packages/plugin-sdk/src/index.ts)) exports **flow-plugin** host functions: `registerNode`, `emitEvent`, `getContext`, `storageGet/Set`, `readFile/writeFile`, `httpRequest`, plus `createPlugin`/`defineNode`/`PluginPermission`. It is generated from Rust host functions by `pdk-gen`.

It does **not** cover overlay widgets. For the capability model to be authorable, the SDK needs a **widget module**:

- **Manifest authoring** — `defineWidget({ id, capabilities, content, … })` + types (`WidgetCapabilities`, `NetworkCapability`).
- **Typed event types** — the `StreamEventPayload` / `FlowEventPayload` / `System` shapes a widget subscribes to (so authors get compile-time types instead of guessing `vortideck-stream-event` payloads).
- **Widget runtime API** — `widget.subscribe(...)`, `widget.trigger(...)`, `widget.data`, mirroring the bridge.
- **Theming constants** — the `--c-*` / `--theme-*` var reference, so authors use tokens instead of hardcoded colors.
- **Capability declaration helpers** — so the manifest's `capabilities`/`permissions` are typed and validated.

This is **Phase 9** of the capability plan. It's required, not optional — without it, widget authoring is reverse-engineered against an undocumented bridge (which is the current state, and why the contract is "missing").

---

## 5. Recommended direction (sequence)

```
1. Widget contract  — manifest + capabilities + PermissionChecker wiring (capability plan Phases 0–1)
2. Runtime safety   — per-widget sandbox + CSP (Phase 2)
3. Bridge filtering — subscribe + redact inbound; gate outbound (Phases 3, 6)
4. Capability dialog + management UI (Phases 4–5)   ← user-facing
5. SDK widget module (Phase 9)                       ← the author contract
6. THEN: rebuild ComponentCreator on the stable contract (simple-widgets layer)
7. LATER (post-marketplace): reconsider a dedicated Studio app
```

The principle: **contract before creator.** The capability plans we already wrote *are* steps 1–5. The `ComponentCreator` is step 6 — and only after the contract it targets actually exists.

---

## 6. Direct answers

- **"Do we need to change the SDK?"** — Yes. Extend it with a widget module (manifest authoring, typed events, `subscribe`/`trigger`, theming constants, capability helpers). Phase 9.
- **"Some elements lack properties (text has no bg) — intended? Should props be added in the creator?"** — Intended (schema choice). Props are added in the **schema's `editableProps`** (the source of truth), which the creator will surface once rebuilt. Add a shared `COMMON_PROPS` set for consistency.
- **"Is the creator even making sense / it's a broken prototype?"** — Correct, it's a stub prototype. The concept is sound for simple widgets but premature (no contract to target). Park it; rebuild after the contract lands.
- **"Separate app for making plugins?"** — Not now. In-app creator (rebuilt) + code SDK is enough and avoids fragmentation. Reconsider post-marketplace if authoring complexity grows.

---

## Next step

Return to the capability plans and begin **Phase 0 (manifest schema) → Phase 1 (permission wiring) → Phase 4 (CapabilityDialog)**, i.e. the "minimum viable security slice." The `ComponentCreator` and the dedicated-app question stay parked until that contract is real.
