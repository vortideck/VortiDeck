# VortiDeck Flow, Stream Action, and Integration Direction

> **Planning update, 5 September 2026:** The provider/capability/stable-action/common-invocation direction is retained. Earlier phase-complete or readiness-percentage statements are not end-to-end certification. Reconcile R02/R07, A01/A02 and W01/W02 with the current checkout, and use P02 for any reopened product decision. Use root TODO and the detailed backlog for current task status and acceptance evidence.

Status: approved direction; Phases 1 through 7, durable Human Approval, typed edges, and graph-native failure policies are implemented.

## Implementation checkpoint

Phase 1 now has a concrete implementation:

- saved integration actions have immutable UUID identities (`stable_id`);
- persisted integration invocations use `SavedActionId(String)`, not OBS row IDs;
- drag-and-drop emits one typed `action-reference` payload for decks, flows, context menus, and stream commands;
- flows persist and execute `invoke-action` with `{ kind, id }`;
- `ActionInvocationService` is the public invocation boundary and delegates resolved actions to the configured singleton `UniversalExecutionEngine`;
- the database migration rewrites existing button, webhook, stream-command, and flow records atomically;
- `assignment-action`, `assignmentRef`, `assignmentMode`, and `ObsActionId` have no permanent runtime reader or executor branch.

The “What exists today” section below records the audited baseline that motivated this cutover. The seven-phase architecture cutover is implemented; remaining work is product hardening, provider expansion, richer repair matching, and fault-injection acceptance testing.

The post-cutover Human Approval work is also implemented as one durable, transport-neutral primitive. Approval requests are persisted before notification; desktop, direct mobile, and remote-relay mobile resolve through one atomic service; authenticated actor/channel and final resolution are audited; timeout/cancellation are explicit decisions; and named control routes schedule only the selected branch. Email and outbound webhook remain notification-gateway adapters, not separate nodes or alternate decision authorities. See `docs/architecture/DURABLE_HUMAN_APPROVAL_ARCHITECTURE.md`.

Remote device transport is specified separately in `docs/architecture/REMOTE_RELAY_GATEWAY_ARCHITECTURE.md`. The dedicated relay and signed protocol-v2 envelope are implemented; remote first-time pairing, encrypted application payloads, and atomic multi-mobile Redis state remain explicit milestones.

Phase 2 now also has a concrete engine foundation:

- `UniversalIntegrationManager` is the single runtime provider and status registry; no third connector registry was introduced;
- every executable provider publishes a versioned `ActionContract`; providers without a contract fail closed;
- the universal execution engine resolves and dispatches arbitrary registered integration IDs instead of constructing OBS directly;
- OBS and Discord publish contracts and are registered before commands become reachable, avoiding a startup race;
- saved-action persistence has no integration switch and stores contract version, connection selector, parameters, and cached presentation fields;
- old OBS tuple CRUD commands and their compatibility implementations were removed, not retained as a second path;
- deck and flow action invocations expose provider preflight health before execution;
- deletion is guarded across buttons, deck-widget bindings, flows, webhooks, stream commands, and context-menu items.

Phase 3 now has a concrete product implementation:

- `CapabilityDefinition` is the one provider-owned model for identity/version, presentation, typed inputs, nested choices, dynamic option sources, typed outputs, and operational execution policy;
- the former discovery `ActionDefinition`/`ParameterDefinition` schema and separate `action_contract` lookup were removed rather than retained as parallel APIs;
- OBS maps its live companion catalog into canonical capabilities, including nested scene/source choices and live option resolvers;
- Discord publishes the same canonical model for message, notification, and alert actions;
- one reusable schema form renders text, textarea, number, color, toggle, select, multi-select, JSON, nested fields, defaults, required values, and live options;
- the saved-action creator is provider-neutral, supports cached offline capability presentation, and provides a usable Discord webhook connection/disconnection flow;
- Discord credentials live in the operating-system credential manager, candidate credentials are validated before replacing a working connection, and passive status checks do not send messages;
- deck assignments, the global action picker, and streaming chat-command configuration consume the same all-provider saved-action catalog;
- provider-specific action-list and direct-execution Tauri convenience commands were removed; user-visible invocation remains behind the universal execution path;
- nested option validation and stale-child cleanup prevent scene/source configuration from drifting silently.

Phase 4 now has a concrete durable invocation implementation:

- `ActionInvocationService` is the single public boundary for resolving `ActionReference` and database `Action` values, invoking the configured engine, and returning a normalized `InvocationResult`;
- results distinguish `applied`, `transport_accepted`, `failed`, `timed_out`, `uncertain`, and `skipped`; transport acceptance is never upgraded to applied success;
- timeouts for connectors that promise correlated applied receipts become `uncertain`, preventing unsafe blind replay;
- deck buttons/widgets, `invoke-action` flow nodes, context-menu items, macro commands, streaming chat commands, manual program execution, the Tauri API command, and executable MCP/AI tools now use this boundary;
- the unused flow program provider and desktop program executor that created fresh default engines were removed;
- flow recovery and resource leases resolve the provider-owned capability contract instead of classifying OBS action names in generic flow code;
- OBS preserves correlated applied receipts where its companion supports them and reports only transport acceptance for older transports;
- Discord webhook actions explicitly report transport acceptance and keep fail-closed recovery semantics;
- Overlay is a first-party capability provider with typed parameters, a target-derived resource key, idempotent retry semantics, versioned command IDs, and honest transport-accepted results;
- the existing durable workflow overlay node remains the stronger path for `hostAccepted`/`widgetRendered` acknowledgements, outbox persistence, expiry, and recovery; the generic overlay action does not falsely claim rendering;
- old framework registries can discover/register providers but can no longer execute around the canonical invocation boundary;
- MCP action tools are registered only when an `ActionInvocationService` is supplied; they no longer emit fake “executed” events and claim success.

Preflight now distinguishes ready, syncing, disconnected, missing integration/target, permission denial, invalid actions, contract drift, and degraded receipt quality. Discord completes the second-provider persistence, schema UI, connection, and dispatch proof.

## Executive decision

VortiDeck should not add a separate hardcoded backend flow node for every OBS operation such as `Switch Scene`, `Show Source`, and `Start Recording`.

The existing OBS direction is fundamentally correct:

1. The connected OBS companion publishes the actions and parameters it supports.
2. The OBS action creator builds a configured, reusable saved action from that live catalog.
3. The Assignments UI exposes the saved action.
4. The same action can be dragged onto a deck destination or into a flow.
5. Execution resolves it and dispatches it through the universal execution engine.

This gives VortiDeck one definition of an OBS operation that decks, macros, flows, context menus, AI, and future surfaces can reuse.

The architecture was not finished at the audited baseline: it was OBS-specific beneath a universal-looking API, the flow used a special `assignment-action` carrying a database integer, and action definitions were too weak. Phases 1-4 removed those structural flaws and established one invocation boundary across the current surfaces; later phases address richer reconnection/repair states, streaming providers, and queued overlay/media capabilities.

The target model is:

```text
Connector capability definition
    -> Saved action binding
        -> Invocation from deck, flow, macro, context menu, AI, or API
            -> Universal execution contract
                -> Applied/failed receipt, durable result, and recovery decision
```

Users should experience one action everywhere. Internally, discovery, configuration, invocation, and execution must remain separate layers.

## Audited baseline before implementation

This section is retained as the historical evidence that motivated the plan. It describes the pre-Phase-1 system, not the current implementation checkpoint above.

### Dynamic OBS discovery

The OBS companion registers its current action catalog with VortiDeck. The catalog includes action IDs, labels, parameters, scene selections, source selections, and nested selections. The backend keeps the current catalog in `StateManager` and exposes it through the universal integration manager.

The OBS creator calls `get_integration_actions("obs")` and refreshes after connection, integration registration, and `obs_register_action`. On disconnect, it clears live choices but leaves database actions untouched. This distinction is correct: live capabilities are connection state; saved user intent is durable state.

Relevant implementation:

- `layers/desktop/pages/deck/new/action.vue`
- `apps/tauri-windows/libs/vortideck-core/src/core/state_manager.rs`
- `apps/tauri-windows/libs/vortideck-core/src/integrations/builtin/obs/mod.rs`
- `apps/tauri-windows/src-tauri/src/commands/universal_integration_commands.rs`

### Saved OBS actions

A saved OBS action currently contains a database ID, integration ID, display name, OBS action ID, and JSON parameters. For example:

```text
Go to Gameplay
obs_scene_activate
scene_name = Gameplay
```

This reusable binding is better than a flow-only node because it can be assigned to hardware buttons, macros, context menus, and flows.

### Assignment drag-and-drop

Saved OBS actions remain listed while OBS is offline. Dragging creates a payload approximately like:

```json
{
  "event": "action",
  "data": {
    "actionType": "obs",
    "actionData": 123,
    "label": "Go to Gameplay"
  }
}
```

The flow editor converts this into a local virtual `assignment-action` node. It persists `assignmentRef` in node inputs and UI custom properties, so the reference survives save and reload. This path works today.

Relevant implementation:

- `layers/base/components/Submenu.vue`
- `layers/desktop/composables/useHandleDragAction.ts`
- `layers/flow/pages/flow/index.vue`
- `layers/flow/components/Flow/Canvas.vue`
- `layers/flow/components/Flow/Panel/NodeInspector.vue`

### Flow execution

The executor recognizes `assignment-action`, converts its reference into the existing database `Action` enum, loads the saved OBS action, and creates:

```text
UniversalAction::Integration {
    integration_id: "obs",
    action_id: saved action ID,
    parameters: saved parameters
}
```

The execution path already has strong production concepts:

- durable side-effect outbox records;
- resource leases for conflicting OBS mutations;
- idempotency metadata;
- connector-owned recovery classification;
- live contract and parameter validation;
- correlated applied/failed receipts when supported;
- timeout behavior;
- fail-closed recovery for uncertain toggle, next, previous, increment, and decrement operations.

This is much stronger than sending an OBS command and assuming it worked.

Relevant implementation:

- `apps/tauri-windows/libs/vortideck-core/src/services/flow/executor.rs`
- `apps/tauri-windows/libs/vortideck-core/src/services/execution/action_converter.rs`
- `apps/tauri-windows/libs/vortideck-core/src/services/execution/mod.rs`
- `apps/tauri-windows/libs/vortideck-core/src/integrations/contracts.rs`
- `apps/tauri-windows/libs/vortideck-core/src/integrations/builtin/obs/mod.rs`

## Direct answers

### Is dynamic creation from OBS the right approach?

Yes. OBS and its companion are the best authority for the actions, scenes, sources, and choices available in that installation. Dynamic discovery prevents VortiDeck from offering controls that cannot execute.

The live catalog must not be the only authority for saved workflows. It is ephemeral. VortiDeck needs a durable cached contract and a saved binding in addition to live discovery.

### Is it based on the OBS connection?

Creation and live editing currently depend on the OBS connection because choices come from the companion. Execution also requires a connected companion. Saved actions and flows remain visible offline, which is correct.

The missing product layer is an explicit state model:

- ready and validated;
- offline but previously valid;
- catalog loading;
- configuration drift detected;
- target missing;
- action unsupported;
- legacy connector without applied receipts;
- needs user rebind.

### Does drag-and-drop already work?

Yes. Saved OBS actions become compact `assignment-action` nodes, persist, and execute through the universal engine.

Keep this UX, but generalize the payload and node identity. The node is not conceptually a deck assignment; it is a reusable action invocation that happened to be dragged from the Assignments browser.

### Should OBS operations become first-class nodes?

They should feel first-class in the UI without becoming separately implemented backend node classes. Searching for `switch scene` should show a polished `OBS · Switch Scene` result, but its schema and behavior should come from the connector contract.

First-class UX does not require duplicated first-class code.

## Current strengths

- The connector is the capability authority.
- Saved actions separate configuration from invocation.
- Offline saved actions remain visible.
- Modern OBS execution waits for applied/failed receipts.
- Recovery correctly distinguishes setters from toggles.
- Resource keys can serialize conflicting recording, streaming, scene, and source mutations.
- Flow execution has an outbox and requires manual review for uncertain duplicate side effects.

## Current flaws and risks

### 1. Assignment is a UI origin leaking into the runtime

`assignment-action`, `assignmentRef`, and `source: "deck"` describe where the item came from, not what it is. Replace this conceptually with:

```text
node type: invoke-action
reference type: saved-action
source surface: optional analytics metadata only
```

### 2. References use fragile database integers

Flows store `actionType = obs` and `actionData = 123`. Deleting row 123 leaves a broken flow. Add stable UUIDs, dependency lookup, guarded deletion, replace-all, and deliberate broken-reference handling. Never allow a reused integer to silently point to a different action.

### 3. Universal persistence only supports OBS

The universal saved-action commands route OBS to legacy storage. Other integrations return unsupported, and Discord is represented as an empty list. The UI model looks universal while persistence is not.

### 4. Durable universal integration execution is OBS-specific

`UniversalExecutionEngine` directly constructs `ObsIntegration` and rejects other integration IDs. Discord is registered and exposes actions, but it cannot yet use the same durable saved-action flow path.

### 5. Action definitions are too weak

`ActionDefinition` mostly provides a name, description, and string/list parameters. Generated nodes need stable IDs and versions, typed input/output schemas, secret fields, dynamic options, target requirements, permissions, timeout, idempotency, receipts, recovery, conflict keys, deprecation, and migration metadata.

Some execution semantics already exist in `ActionContract`. Discovery and execution should become two views of one versioned capability contract.

### 6. OBS targets are name-based

Scene and source renames safely fail validation, but break workflows. Save stable OBS resource IDs when available plus a display-name fallback. Otherwise use an explicit exact-name selector. Never silently choose a similarly named resource.

### 7. Account and instance targeting are incomplete

A saved binding identifies `obs`, but not robustly which OBS instance, profile, scene collection, or remote connection it intends. This is necessary for multiple desktops and virtual OBS instances.

### 8. Deep validation happens too late

Graph validation does not fully resolve the saved row, current connector capability, parameters, target, and permissions. Add preflight before flow activation and after reconnect/catalog changes.

### 9. Labels are snapshots

The flow stores the drag-time label. Resolve current presentation from the binding while retaining cached text for offline and missing-reference display.

### 10. The OBS creator knows specific parameter names

The frontend explicitly serializes `scene_name`, `source_name`, and `collection_name`. New companion parameters can require frontend changes. Replace this with generic schema rendering and serialization.

### 11. Connection and readiness are collapsed

Connected does not mean the full action catalog has arrived. Model `disconnected`, `connecting`, `transport_connected`, `catalog_syncing`, `ready`, and `degraded` explicitly.

### 12. Execution paths can diverge

The universal integration manager can invoke actions directly, while decks/flows use the execution engine. Every user-visible action should pass one policy boundary for permissions, idempotency, receipts, logs, and recovery.

### 13. Streaming platforms are separate from connector actions

Twitch, YouTube, Kick, and TikTok events use the streaming relay/event bus, while chat and moderation use streaming commands. Different transports are fine, but workflows need one capability contract.

### 14. Large editor pages are a change risk

The flow page is roughly 5,782 lines and the overlay page roughly 3,270 lines. Connector behavior should not be added directly to them. Extract schema forms, action binding, preflight, and generated-node presentation.

## Target domain model

### Integration connection

Represents an authenticated account or local/remote provider instance:

```text
connectionId
integrationId
accountId or instanceId
displayName
status and readiness phase
capabilityRevision
lastSeenAt
configuration scope
```

Examples include a local OBS instance, remote OBS instance, Twitch channel, YouTube channel, Discord server/webhook, and overlay host.

### Capability definition

Connector-owned, versioned description of an available trigger, action, or query:

```text
integrationId
capabilityId and version
kind: trigger | action | query
input and output schemas
dynamic option resolvers
permissions and target requirements
timeout and receipt semantics
idempotency and recovery semantics
resource-key strategy
deprecation and migration metadata
```

### Saved action binding

Durable user intent configured from a capability:

```json
{
  "id": "stable-uuid",
  "name": "Go to Gameplay",
  "integrationId": "obs",
  "capabilityId": "obs_scene_activate",
  "capabilityVersion": 1,
  "connectionSelector": {
    "mode": "exact",
    "connectionId": "local-obs-main"
  },
  "parameters": {
    "scene": {
      "resourceId": "scene-uuid-if-available",
      "fallbackName": "Gameplay",
      "matchPolicy": "exact"
    }
  },
  "lastValidatedRevision": "catalog-revision",
  "cachedPresentation": {
    "integrationName": "OBS",
    "actionName": "Switch Scene",
    "summary": "Gameplay"
  }
}
```

### Action invocation

A flow node or button references a saved binding and may supply runtime parameter overrides:

```json
{
  "bindingId": "stable-uuid",
  "parameterBindings": {
    "message": "{{event.message}}"
  }
}
```

Support two modes:

- **Saved action** for reused streamer operations and deck buttons.
- **Inline integration action** for flow-specific dynamic operations.

Both compile to the same universal action contract.

### Normalized invocation result

All connectors should return:

```text
status: applied | failed | timed_out | uncertain | skipped
requestId and providerReceiptId
startedAt and completedAt
typed outputs
error code and message
retryability
recovery recommendation
per-target results when applicable
```

## Responsibility boundaries

| Responsibility | Connector | Binding service | Runtime | UI |
|---|---:|---:|---:|---:|
| Discover supported capabilities | Yes | No | No | No |
| Define schemas and operational semantics | Yes | No | Consume | Render |
| Supply live scenes/channels/options | Yes | Cache | No | Render |
| Store configured intent | No | Yes | Resolve | Edit |
| Validate current target | Yes | Coordinate | Enforce | Display |
| Declare permissions | Yes | Preserve | Enforce | Explain |
| Schedule, retry, outbox, and leases | No | No | Yes | Display |
| Produce provider receipt | Yes | No | Correlate | Display |

Platform-specific behavior belongs in connectors, not in the generic flow editor.

## How each provider should integrate

### OBS

Keep dynamic companion discovery. Bind actions to a selected OBS connection/instance and stable resource identities. Validate against the current catalog and wait for an applied receipt. Prefer convergent setters such as `Set Camera Visible = true` over toggles.

### Twitch, YouTube, Kick, and TikTok

The streaming relay can remain the transport but should expose contracts for supported actions:

- send chat message or announcement;
- timeout/ban a user;
- delete a message;
- create a clip;
- update title or category where proven supported.

A cross-platform action can target the originating platform, selected platforms, or all active capable platforms. Unsupported targets and partial success must be explicit.

### Discord

Discord already declares send-message, notification, and alert actions. It should become the second provider proving generic saved-action persistence and dispatch. Discord event subscription is still TODO and should be separate from outbound actions.

### Overlays

Treat the overlay runtime as a first-party connector with typed actions:

- show/hide element;
- play and stop animation;
- set text/image/media;
- set/increment/reset counter;
- enqueue alert;
- change state/theme;
- wait for animation or playback completion.

The overlay delivery path already has authenticated, versioned, instance-targeted commands and acknowledgements. Reusing the capability contract will join flows and overlays into one product.

## Recommended node experience

The node library should visually list generated first-class entries:

```text
OBS · Switch Scene
OBS · Set Source Visibility
Twitch · Send Announcement
Discord · Send Message
Overlay · Play Alert
```

These are generated presentations of connector contracts, not independent implementations.

Add a `My Actions` section for saved bindings:

```text
Go to Gameplay      OBS · Switch Scene
Hide Camera         OBS · Source Visibility
Community Alert     Discord · Send Message
Raid Celebration   Overlay · Play Alert
```

Dragging a saved item creates `invoke-action`. Dragging a capability creates an inline action configured in the inspector, with an option to save it for reuse. Search aliases such as `scene`, `camera`, `mute`, and `clip` should make generated actions feel native.

## Seamless user journey

### First connection

```text
Connect OBS
    -> Synchronizing actions, scenes, and sources
    -> Catalog ready
    -> Suggested high-value actions
    -> Create Go to Gameplay
    -> Drag to deck button or flow
    -> Test safely
```

### Flow creation and activation

```text
Choose trigger
    -> drag saved or inline action
    -> inspector shows human-readable intent
    -> preflight validates connection, target, permission, and receipt support
    -> simulate event without side effects
    -> test selected actions deliberately
    -> activate
```

### Offline editing

Nodes remain readable:

```text
Go to Gameplay
OBS · Switch Scene · Gameplay
OBS offline — last validated yesterday
```

Users may inspect and rearrange flows. The UI must not claim current validation while offline.

### Reconnect and configuration drift

If the capability revision and targets remain valid, return the binding to ready automatically. If a target changed, show `Needs rebind` with the previous target and current candidates. Do not guess unless a stable resource ID proves identity.

### Deletion

Before deleting a saved action, show all dependent flows, buttons, macros, and context-menu items. Offer replace-all, cancel, or deliberate break. Never silently delete a referenced action.

## Preflight and health

Each action node needs one of these health states:

- `ready`: validated against the current connection;
- `offline`: connection unavailable, last-known contract retained;
- `syncing`: transport connected, catalog incomplete;
- `drifted`: capability or options changed;
- `missing_target`: scene/source/channel no longer exists;
- `missing_binding`: referenced action was removed;
- `permission_required`: required grant missing;
- `legacy_unconfirmed`: only transport acceptance is available;
- `unsupported`: current connector lacks the capability;
- `disabled`: deliberately excluded.

Allow saving in every state. Block activation for missing bindings, invalid parameters, missing permission, and unsupported capabilities. Offline-but-previously-valid nodes may be an acknowledged warning. Never silently skip a required action in a live run.

Revalidate when a connector or catalog changes, a binding changes, a resource changes, permissions change, an action is removed, or a flow is imported/activated.

## Runtime reliability rules

### One invocation boundary

Every surface should call one service that:

1. resolves the binding and target connection;
2. loads the current capability contract;
3. resolves runtime expressions;
4. validates types, permissions, and targets;
5. claims idempotency/outbox state;
6. acquires resource leases;
7. dispatches through the selected connector;
8. correlates the receipt;
9. persists a normalized result;
10. applies connector-declared recovery policy.

Deck buttons may use lighter run records than workflows, but must not bypass policy or receipts.

### Honest receipt states

Distinguish transport accepted, applied, rejected, and timed-out/uncertain. Never present `sent` as `applied`.

### Convergent operations

Prefer setters over toggles. Setters can reconcile after restart; uncertain toggles must fail closed and require review.

### Idempotency identity

Keys must include workflow run identity and node execution identity. Replaying the same source event should deduplicate, while a legitimate new run must not collide with an old completed run.

### Resource conflicts

Connectors calculate resource keys. The runtime supports queue, replace previous, reject while busy, and coalesce where safe. Emergency workflows may preempt lower-priority visuals while preserving deterministic cleanup.

### Partial multi-target outcomes

For a multi-platform action, return per-target results and an `overall: partial` status when appropriate. Flows must be able to branch on full, partial, and failed outcomes.

## Triggers versus actions

Triggers and actions share connector identity and schemas but remain separate capability kinds. Replace the eventual one-trigger-per-flow limitation with multiple ingress routes or an event router, because follow, subscription, and raid may legitimately converge on one workflow.

Canonical events should include:

```text
eventId and eventType
source integration and connection/account
streamId
occurredAt and receivedAt
correlationId
subject/user
normalized fields
platform-specific payload
```

## Overlay and external video/media direction

Build a provider-neutral moderated media pipeline before direct Discord-call capture:

```text
Discord attachment, approved URL, upload, or webhook
    -> validate source/type
    -> moderation or approval
    -> controlled download/proxy cache
    -> inspect size, duration, codec, and limits
    -> enqueue media
    -> overlay/OBS presentation
    -> playback-started receipt
    -> playback-completed receipt
    -> policy-based cleanup
```

Start with local MP4/WebM, approved HTTPS URLs, Discord attachments through a bot/webhook, uploads, and supported clip URLs. Add WebRTC/SRT/RTMP guest feeds later.

Direct Discord call capture should not be first because API access, privacy, consent, latency, and capture reliability are harder. OBS window capture remains a practical fallback while VortiDeck develops a generic guest/media source contract.

## Implementation sequence

### Phase 1: Normalize the current abstraction

Implementation status: runtime cutover complete. Stable identity, canonical payload/node/runtime semantics, atomic data migration, and guarded deletion are implemented. Versioned drag MIME and a full dependency/replace-all UI remain follow-up UX work.

- Add `SavedActionBinding` with a stable UUID.
- Add `ActionReference` and `invoke-action` terminology.
- Perform a one-time atomic migration from `assignment-action` and `assignmentRef` to the canonical model.
- Remove the old node type, payload shape, producer code, and executor branch immediately after migrated data is verified.
- Version the drag payload and give it an action-specific MIME type.
- Resolve current labels with cached offline fallback.
- Add dependency lookup and guarded deletion.

Migration compatibility is a temporary data-boundary tool, not a permanent runtime architecture. The finished runtime must understand only `invoke-action`, `ActionReference`, and `SavedActionBinding`. If no released installation or user data requires preservation, update development fixtures/data directly and omit the compatibility reader entirely. If preservation is required, retain only an idempotent, versioned one-time migrator; do not retain dual execution paths.

### Phase 2: Make persistence genuinely universal

Implementation status: engine and repository foundation complete. Generic persistence, contract version fields, provider registry dispatch, OBS/Discord capabilities, and preflight are implemented. Phase 3 subsequently completed cached presentation authoring and the Discord creator; multi-connection targeting remains a later concern.

- Replace OBS-only command switches with a generic action repository.
- Store integration, capability/version, connection selector, parameters, and cached presentation.
- Migrate existing `saved_actions` without breaking deck buttons or flows.
- Add Discord saved actions as the second proof provider.

### Phase 3: Unify contracts and schema rendering

Implementation status: complete. The canonical capability schema, OBS and Discord mappings, dynamic option route, typed output declarations, reusable renderer, provider-neutral saved-action editor, and all-provider action catalogs are implemented. More field widgets can be added as capability needs appear without introducing provider branches.

- Replace hardcoded OBS parameter extraction with schema-driven forms.
- Join discovery definitions and operational contracts into one versioned capability model.
- Add output schemas and dynamic option providers.
- Extract reusable schema renderer and action inspector components from the large pages.

### Phase 4: Establish `UniversalExecutionEngine` as the durable invocation boundary

Implementation status: complete. `ActionInvocationService` is the durable public boundary around the singleton engine; all current user-facing action entry points use it, normalized receipt quality is explicit, provider contracts drive flow recovery, and Discord/overlay have honest first-party contracts.

- Route deck, macro, flow, context menu, AI, and API invocation through one boundary.
- Dispatch arbitrary integration IDs through the integration registry.
- Preserve OBS outbox, leases, receipts, validation, and recovery behavior.
- Define equivalent contracts for overlays and Discord.

The service and engine are deliberately not duplicates. `ActionInvocationService` owns durable identity, source attribution, idempotency claims, the resolved-action snapshot, normalized outcomes, and inspection. `UniversalExecutionEngine` remains the execution mechanism for process, macro, flow, system, HTTP, and provider dispatch. Specialized overlay delivery keeps its visual ACK adapter, but begins and completes the same canonical invocation record.

The workflow side-effect outbox also remains intentional. It is the workflow's crash-safe dispatch claim and recovery checkpoint, while `action_invocations` is the cross-surface operational record. They share an invocation ID; neither is a fallback execution path or legacy compatibility layer.

### Phase 5: Add preflight and repair UX

Implementation status: complete for the current provider/action model. Health is revalidated when a node is loaded or selected, when integration/plugin lifecycle events arrive, and authoritatively at activation. The backend rejects activation when a saved action is missing, disconnected, invalid, permission-blocked, target-missing, or contract-drifted. Transport-only receipt capabilities remain executable with an explicit warning.

- Add node health badges and a flow health summary.
- Validate on activation and connector catalog changes.
- Add reconnect, rebind, replace-all, and disable repairs.
- Add a test panel showing resolved action, request, receipt, duration, and result.

Implemented repair/inspection surfaces include:

- refresh and connection-settings repair;
- in-place rebind while preserving stable identity;
- dependency inspection and guarded deletion;
- replace-all across deck buttons, widget bindings, flows, webhooks, stream commands, and context-menu items;
- disable-all-affected-flows;
- test invocation and per-node recent invocation history;
- canonical request, provider receipt, result, duration, recovery decision, and normalized status persistence.

### Phase 6: Wrap streaming platform actions

Implementation status: complete for the relay operations the application currently supports. Streaming platforms now publish provider-neutral `send_chat`, `ban_user`, `timeout_user`, and `delete_message` capabilities. Saved actions use dynamic account selectors, every mutation enters through `ActionInvocationService`, and multi-account dispatch exposes normalized overall and per-target outcomes to workflows.

- Represent relay chat/moderation commands as capabilities.
- Add account and target selectors.
- Normalize partial multi-platform outcomes.
- Add clips and channel metadata only where provider support is verified.

The current relay protocol confirms local WebSocket transport acceptance, not provider application. These actions therefore persist `transport_accepted` (or `partial`) and use manual-review recovery; they never claim that Twitch, YouTube, Kick, or another provider applied the operation. True applied receipts require the relay to return a correlated request ID and per-account provider result. Clips and channel metadata remain intentionally deferred because no verified command/receipt contract exists for them yet.

The former direct Tauri streaming mutations keep their public command names for UI compatibility, but they are no longer an execution path: each command builds a canonical integration invocation. This is API stability, not a second or legacy engine.

### Phase 7: Add overlay and media capabilities

Implementation status: complete for the authenticated browser-source and approved-HTTPS media contract. Overlay now publishes generated widget show/hide/update actions, queued alerts, queued media playback, and moderated Discord-media ingestion through the same saved-action and invocation architecture.

- Expose overlay operations as generated actions.
- Add alert and media queues.
- Support playback-started and playback-completed receipts.
- Build moderated Discord attachment and URL ingestion.

Alert and media queue items are persisted before dispatch and serialized independently by queue type. A restart never blindly replays an item that was already dispatching: it is marked interrupted for manual review. Items that were only queued remain durable and wait for an authenticated overlay host to reconnect.

The browser source renders first-party alert, image, audio, and video playback and sends correlated `playbackStarted` and `playbackCompleted` receipts. The canonical invocation is reported as applied only after completion. Generic widget commands retain transport semantics, while `widgetRendered` commands now reject host-only acknowledgements.

Discord ingestion is deny-by-default. It requires an explicit approved moderation decision, an HTTPS non-local URL, a declared audio/video/image content type, and a size no greater than 100 MiB before entering the media queue. The boundary accepts normalized Discord attachment metadata from flows, webhooks, or a future bot/gateway connector; it does not pretend that the current outgoing-webhook-only Discord connector receives gateway events.

### Post-cutover: Durable Human Approval

Implementation status: core, desktop, direct-mobile, and relay-mobile paths complete. A durable request is the authority; transports only present or submit it. Resolution is atomic and idempotent, records actor/channel/comment, survives restart, and produces exclusive `approved`, `denied`, or `timed_out` control routing. Notification targets and authorized decision channels are separate policy fields.

Email and signed-webhook delivery require a configured remote notification gateway with short-lived single-use capabilities. They are intentionally not shown as working node channels yet; adding that gateway consumes the existing events and decision API without changing the workflow engine.

### Post-phase hardening: asynchronous runtime bootstrap

Implementation status: complete. Tauri's synchronous setup hook now publishes lightweight service handles without blocking the active Tokio runtime. Streaming session restore, Discord connection inspection, and overlay-media recovery initialize concurrently. Built-in providers are then published as one catalog batch, and integration status polling starts only after publication.

`ActionRuntimeReadiness` is the single barrier shared by canonical invocation and integration discovery. It exposes `initializing`, `ready`, and `failed` snapshots, includes non-fatal provider warnings, bounds waits so a crashed initializer cannot hang actions forever, emits desktop readiness events, and sends the current signed snapshot to direct and relay mobile clients. This replaces the temporary `block_in_place`/`block_on` startup bridge; no synchronous async-runtime nesting remains in this path.

Overlay recovery also resets its start claim when database recovery fails, allowing a later retry instead of reporting a started queue with no workers.

### Post-phase hardening: typed workflow edges

Implementation status: complete. Persisted and runtime graph edges are now a discriminated union, so scheduling and value transport cannot be confused by canvas handle names.

```typescript
type FlowConnection =
  | { kind: "control"; fromNode: string; toNode: string; route?: string }
  | { kind: "data"; fromNode: string; fromOutput: string; toNode: string; toInput: string };
```

Control edges alone participate in reachability, joins, route selection, and cycle validation. Data edges alone resolve node inputs and value dependencies. Named route IDs and data-port IDs remain provider-neutral strings, so plugins can add nodes and capabilities without changing the engine or edge enum.

Canvas handles such as `flow-output-approved` and `value-input-message` now exist only in the editor projection. They are converted once when a user creates an edge and reconstructed only for rendering. Neither the scheduler nor persisted graph logic infers edge meaning from those strings.

Existing four-port flow JSON is accepted only by isolated migration decoders. Imported flows are normalized immediately, and database rows are rewritten to the typed form when loaded. New saves and exports write only typed edges; there is no parallel legacy scheduler or execution path.

The graph is validated before activation and execution: endpoints must exist, self-links and duplicates are rejected, control routes and data ports must be declared by their nodes, and control cycles are rejected. Inactive drafts can still be saved while incomplete. This is compatible with the plugin model because validation uses each node's published input/output contract rather than hardcoded provider or node-type branches.

### Post-phase hardening: graph-native retry, timeout, and error handling

Implementation status: complete. The former `Retry`, `Timeout`, `Try-Catch`, and `Fallback` catalog entries were unfinished wrapper simulations and have been removed, not retained as legacy nodes.

Retry and timeout are execution policies on the node they govern. The scheduler persists retry waits, restores retry counts, applies the configured timeout, and records the final attempt. `Invoke Action` retries are authorized automatically only when the provider contract declares `RetryWithIdempotency`; the same idempotency key is propagated to the provider. Other side effects remain fail-closed and require their explicit recovery path.

Catch and fallback behavior is represented by the graph itself. Selecting **Continue through On Error** on a node publishes one `On Error` control output plus typed `Error Message` and `Error Details` data outputs. A successful node follows its ordinary output. After retries are exhausted or a timeout occurs, only the `On Error` branch becomes reachable. The run retains the failed node result and finishes as completed-with-errors when the recovery branch succeeds, so failure is visible rather than rewritten as success.

### Post-phase hardening: canonical node catalog

Implementation status: complete. Backend and plugin node metadata now separates stable category identity, category display label, and optional subgroup. `CONTROL_FLOW` is normalized into the canonical `CONTROL` category at the presentation boundary, and both the quick node picker and full Node Library render from the same catalog builder. Duplicate node IDs are rejected during registry publication instead of silently replacing an existing node.

The widget schema demo is a development-only fixture under `Development / Schema Fixtures` and is not published in release builds. Plugin authors can supply `subcategory` without requiring engine changes; omitted subgroup metadata safely appears under `General`.

## Runtime nodes to prioritize

Do not independently code dozens of OBS nodes first. Build the generic/runtime layer:

1. `Invoke Action` — saved binding or inline connector action.
2. `Wait for Event/Receipt` — typed durable wake.
3. `Cooldown` — flow, stream, user, or custom key scope.
4. `Deduplicate Event` — explicit workflow-level dedupe.
5. `Rate Limit` — queue, drop, or coalesce.
6. Resource serialization policy.
7. `Cancel/Replace Previous Run`.
8. `Parallel` and `Join`.
9. `Run Reusable Flow/Macro` with typed inputs/outputs.
10. Structured `On Error` branching. Implemented as a node failure policy plus graph route, not a wrapper node.

Then expose connector-generated OBS, stream platform, Discord, and overlay entries.

## Updated readiness assessment

| Area | Estimate | Meaning |
|---|---:|---|
| OBS discovery and action creation | 88% | Live catalog and generic nested schema rendering are complete; stable OBS resource identity and guided rebind remain. |
| Assignment reuse and drag/drop | 92% | Canonical stable references and all-provider catalogs work across deck, flow, and stream commands; dependency inspection and replace-all are implemented. Versioned MIME remains. |
| OBS flow execution reliability | 90-92% | Outbox, leases, correlated receipts, activation health gates, canonical invocation records, validation, and provider-derived recovery/resource semantics are unified; restart and fault-injection acceptance testing remains. |
| Generic integration action platform | 96-98% | Persistence, canonical capabilities, schema UI, registry dispatch, durable invocation history, normalized outcomes, streaming, Discord, and queued overlay media share one boundary; more provider connectors and acceptance hardening remain. |
| Cross-surface invocation consistency | 96-98% | Deck, flow, macro, context menu, stream commands, manual/API, executable AI tools, and specialized overlay ACK delivery share canonical invocation identity and history. |
| Seamless reconnect and repair UX | 82-88% | Health badges, lifecycle refresh, activation gates, rebind navigation, replace-all, disable-flow repair, testing, and history are implemented; richer target candidate matching and last-known offline validation remain. |
| Streaming platform actions in flows | 72-78% | Chat and moderation are canonical capabilities with dynamic account targeting, durable invocation history, and per-account partial outcomes; correlated provider-applied receipts, verified clips/channel operations, and richer provider permission guidance remain. |
| Overlay command and media actions | 90-94% | Generated widget operations, durable alert/media queues, authenticated browser rendering, lifecycle receipts, and moderated Discord ingestion are implemented; local-file serving, richer visual templates, and a Discord gateway source remain product extensions. |
| Durable human approval | 88-92% | Durable state, atomic/idempotent resolution, restart recovery, exclusive control routing, desktop notification, and signed direct/relay mobile decisions are implemented; a dedicated desktop inbox, process-kill E2E coverage, and optional email/webhook gateway remain. |

The dynamic OBS assignment mechanism is one of VortiDeck's stronger foundations. The fastest path to a product that feels simple and deep is completing this reusable contract, not multiplying independent action systems.

## Acceptance criteria for seamless and dependable behavior

- One saved OBS action can be used by a deck button and flow with the same binding.
- Closing OBS does not erase or corrupt configuration.
- Reopening unchanged OBS automatically returns bindings to ready.
- Renaming/removing a scene or source produces an actionable preflight issue before going live.
- Deleting a referenced action shows every dependent flow, button, macro, and context-menu item.
- Transport acceptance is never reported as applied success.
- An uncertain toggle is never automatically replayed.
- A setter can reconcile safely after restart.
- Conflicting workflows follow a visible queue/replace/reject policy.
- Duplicate source events do not duplicate side effects.
- A legitimate new run is not blocked by an old outbox record.
- Multi-target actions expose per-target outcomes.
- Every run shows its input event, binding, connection, dispatch, receipt, duration, and result.
- Legacy flows and assignments migrate without data loss.

## Final direction

Keep the current OBS action creator and assignment workflow. It is the correct foundation.

Move the conceptual center from “OBS actions dragged from deck assignments” to:

> **Connected systems publish capabilities. Users configure reusable actions. Every VortiDeck surface invokes those actions through one durable, inspectable runtime.**

OBS actions remain platform-specific in what they do, but not in how VortiDeck discovers, stores, displays, invokes, validates, observes, and recovers them. The same model should cover Twitch, YouTube, Kick, TikTok, Discord, overlays, plugins, and future media providers.

That architecture gives users a seamless experience while preserving the depth and reliability required during a live stream.
