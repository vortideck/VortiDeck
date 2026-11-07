# Product and runtime decisions for VortiDeck 0.7

Status: current decision record, 5 September 2026. This records architectural
policy; task completion and release acceptance remain in `TODO.md`.

Revalidated 6 September: post-plan review and stabilization briefs. The original plan is 30/31 complete, but later runtime/import/persistence findings and release diagnostics remain. The decisions below describe ownership and supported scope; they do not certify every lifecycle race or future adapter.

## Product boundary

VortiDeck 0.7 is a local-first programmable live studio: deck controls, OBS,
overlays, stream events, reusable flows, local/system actions and a copilot that
can inspect and edit those flows. A general hosted automation platform, a
second autonomous-agent runtime and compulsory cloud infrastructure are not
0.7 architecture requirements.

## Ownership and durability answers

| Question | Current decision |
| --- | --- |
| Where do actions execute? | The desktop Tauri host composes the runtime. Saved effects resolve through `ActionInvocationService` into the registered native/integration/plugin provider. Flow nodes execute under the run-owned `FlowExecutor` scheduler. Deck, overlay, assistant and future adapters reuse those owners; they do not each implement an executor. |
| Who owns task state? | `FlowExecutor` plus runtime storage own workflow run/recovery state. The conversation session actor owns one chat task lifecycle and persisted message/tool ordering. A transport event or UI component is a projection, not a task owner. |
| When is input durable? | Consequential stream input routed to workflows is accepted only after the canonical inbox write. The inline session chat-command route currently executes before that write and is not covered by this guarantee; F14 closes its admission gap. Webhooks use their own request/response-aware durable ingress. Ordinary observation/progress traffic may remain transient. A Tokio broadcast send is never called durable acceptance. |
| Which graph edits persist? | Save, Run, Export and assistant edits use the canonical serialized graph. Assistant replacement requires an inactive graph and expected semantic revision, validates the document, performs compare-and-save, then reads the stored revision back. UI-only projection state is not a persisted edit. |
| What does completion mean? | Accepted/queued, running, provider-applied, failed, cancelled and uncertain are distinct. A flow launch receipt is not the terminal result. External effects do not receive a blanket exactly-once promise. Overlay host receipt, browser render and final audience exposure are also distinct. |

## Supported execution modes

| Mode | 0.7 status |
| --- | --- |
| Windows desktop application and local Rust runtime | Implemented; release depends on the remaining TODO/RC gates. |
| Local flow execution, durable recovery and saved-action invocation | Implemented at automated boundaries recorded in R01–R08; remaining live gates stay explicit. |
| OBS native companion/provider | Supported native integration, not a WASM marketplace plugin; O01/O02 now record installed-binary acceptance. Later active-overlay snapshot/cold-start acceptance is F01. |
| WASM plugins | Supported through the public SDK and Extism runtime with declared grants. |
| Service-process plugins | Supported as trusted executable extensions with grant-checked host RPC; not an OS sandbox. |
| Outbound MCP clients | Supported for tested stdio and Streamable HTTP paths documented by A06. The deprecated `sse` configuration alias does not establish a separate tested legacy SSE transport. |
| Inbound MCP control of VortiDeck | Proposed adapter, not implemented or advertised as supported. |
| Mobile companion | Existing client path; I02 records local-pair-first relay/session evidence, W02 still requires two physical clients, and F10 covers consequential endpoint replay beyond the current process lifetime. |
| Non-UI/background host | Shared lifecycle and a bounded authenticated loopback proof exist; no daemon is packaged or supported until the H02 shipping gates are implemented. |
| Self-hosted/cloud execution | Deferred by H02. The backend remains a control-plane relay; a remote service cannot operate a powered-off local OBS/desktop. See [`HOSTING_SCOPE_DECISION.md`](HOSTING_SCOPE_DECISION.md). |

## Reconciled policy choices

- There is no universal five-second AI polling loop. Event/schedule-driven work
  uses an explicit budget and provider-specific cadence when a real scenario
  requires it.
- There is no fixed ten-second boundary between “short” and “durable” waits.
  Durability follows consequence and recovery requirements, not elapsed time.
- Flow versus agent is not defined by lifespan. A durable flow may wait for
  hours or days; a separate agent engine must earn its place through different
  ownership or behavior.
- Suggestion chips may have useful local defaults and offline behavior. Cloud
  updates/personalization are optional and require separate privacy, cost and
  fallback decisions.
- Integrations do not all have to be WASM. OBS and OS facilities can remain
  native providers; sandboxed distributable logic can use WASM; native service
  plugins remain explicitly trusted executables.
- Hermes, Flow-Like, Node-RED, Home Assistant and Temporal are references for
  bounded patterns, not dependencies or replacement runtimes.

## Roadmap disposition

- Current implementation and acceptance work is the original task set plus the
  bounded F01–F16 and overlay-authoring follow-ups in root `TODO.md`; each item links to its brief and
  execution evidence. Completed original tasks are not reset by this review.
- Studio preflight/rehearsal/recipes are P01 and should begin with one useful
  break-scene or start-stream scenario.
- Background hosting is H01; cloud/self-hosted scope is H02; one inbound Discord
  scenario is I01; device gateway races/claims are I02.
- A generic autonomous watcher platform, mandatory cloud suggestion registry,
  universal plugin conversion, Kubernetes/multi-region execution and broad
  marketplace recipe expansion are deferred until a bounded scenario proves
  need, authority, offline behavior and operating cost.
- Unsupported readiness percentages in older completion reports are historical
  prose. Only a checked root task with its current evidence, followed by the
  release gates, changes release readiness.

## Evidence index

- 3 October scope correction: chat/action/release review and F14–F16 input/admission briefs. Shared execution ownership remains the policy; merely passing context variables does not prove that a saved action consumes them.

- Runtime/event/recovery: R01–R08 execution notes.
- Overlay, widget and authority: O01/O02, W01/W02, S01–S04 notes and remaining
  gates.
- Copilot/task/graph/MCP: A01–A06 notes.
- SDK and developer boundaries: D01–D03 notes.
- Overall rationale and source review:
  [`ARCHITECTURE_GUIDE.md`](ARCHITECTURE_GUIDE.md) and the
  `planning-document register`.
