# VortiDeck architecture: current foundation and decision guide

Review date: 5 September 2026. Scope: the current VortiDeck working tree at HEAD `5f26f18a1aa85313416ae9f00a46112c4047492e`, including substantial uncommitted work. This is an evidence-backed recommendation, not a release sign-off or a declaration that every proposal is approved.

Updated 6 September after the remaining implementation work: post-plan review and bounded stabilization plan. The original plan is 30/31 complete; W02 remains a physical-client gate. The older validation section below describes its dated snapshot, not every later edit.

Use this guide for the overall architectural direction. Use TODO.md for task completion, the agent backlog for implementation acceptance, and subsystem specifications for their detailed contracts. Where older documents conflict with this guide, the reconciliation table below states exactly what is being corrected. Preserve historical evidence; do not resume an old migration from its unchecked boxes.

The concise current answers for execution ownership, durability, graph edits,
supported modes and deferred proposals are in
[`PRODUCT_RUNTIME_DECISIONS.md`](PRODUCT_RUNTIME_DECISIONS.md).

Review records: evidence log, Markdown coverage register, source hashes.

## 1. Architectural conclusion

Keep a modular local application with a Rust execution runtime, desktop/native adapters, shared frontend components and optional external interfaces. The recent invocation, graph, recovery, plugin-authority and widget changes provide useful foundations. A wholesale rewrite, mandatory service hierarchy, microservices migration or replacement agent runtime is not justified by this review.

The next architectural improvement is consolidation around actual ownership: one production executor per runtime, one provider registry for actual dispatch, one definition of each durable state transition, and transport adapters that reuse those operations. Reduce misleading scaffolding and duplicated policy while preserving working behavior.

The application is more dependable in several important areas than the earlier reviewed snapshot. Current tests exercise real persistence, crash windows and contract behavior. This does not make the whole product fully verified: installed OBS behavior, connected iframe revocation, physical clients and some release paths still have open acceptance gates.

There is no uniquely best September 2026 Rust architecture, and no guarantee against future product changes. The durable choices are explicit ownership, small public contracts, recoverable state, replaceable external providers, understandable failure semantics and tests at real boundaries. These reduce the cost of future change without predicting an AGI timeline.

## 2. The product should determine the boundaries

Recommended focus: a dependable programmable live studio. Deck buttons/widgets, OBS, overlays, live events and repeatable routines make a coherent product. The flow editor exposes procedure; the copilot helps author, inspect and operate it; external agents can become additional authorized clients.

General HTTP/MCP/script capabilities remain useful escape hatches. They do not require making VortiDeck a generic hosted automation platform or embedding a separate always-running agent framework. The architecture should permit those experiments without making the everyday studio depend on them.

A useful feature must improve a recurring job: preparing a stream, handling an audience event, switching a scene safely, recovering from a failed action, explaining what happened, or distributing a reusable control. Evaluate optional cloud hosting, gateways and agent features against an actual scenario and operating cost.

Keep operator-approved routines usable without asking for confirmation at every repetition. Approval belongs to the operation's authority and consequence, with an explicit grant/revision policy; it is not a universal modal inserted into all execution.

## 3. Runtime ownership, in plain terms

| Responsibility | Current owner/foundation | Architectural decision |
|---|---|---|
| Desktop startup and native facilities | Tauri `main.rs` and host adapters | Keep the composition root here initially. Extract startup functions and owned task handles before introducing a second host. |
| Registered provider capabilities/status | `UniversalIntegrationManager`, `AppIntegration`, capability contracts | Keep the live registry and concrete provider implementations. Do not add a CLI/MCP registry of the same capabilities. |
| Saved-action resolution and effect receipts | `ActionReference`, converter, `ActionInvocationService`, universal engine | Keep the meaningful invocation boundary and its durable records. Internal engine dispatch remains an implementation detail. |
| Graph execution and run state | `FlowExecutor`, run-owned scheduler, `FlowStorage` | Keep these. A workflow is not a second implementation of every provider action. |
| Persisted flow definition | Main database plus canonical frontend serializer and revision checks | Runtime loads/validates canonical documents; UI edits are projections, not another source of authority. |
| Runtime recovery records | `HybridFlowStorage`, inbox/outbox, wakes, receipts and history | Keep the existing state machines; document cross-store consistency instead of claiming a global transaction. |
| Package grants | `PermissionChecker` and package lifecycle | Keep effective grants and revocation as authority. Each transport binds the true source and filters delivery. |
| Deck interaction | Persisted host bindings plus widget intent/state contracts | Keep rendering separate from execution. Clients send intent, not executable action definitions. |
| Overlay delivery | Overlay host, durable delivery records and correlated browser/widget receipts | Keep host acceptance and rendered evidence distinct; scene composition remains overlay-specific. |
| Conversation/task state | Existing session actor, conversation persistence and bounded tool lifecycle | Keep actor ownership where it provides real serialization. Share effect operations with the runtime. |
| CLI / inbound MCP | Proposed adapters; inbound MCP is not implemented | Add only when useful. Reuse meaningful operations directly; no compulsory wrapper class. |

One runtime owner does not mean one mutex, one giant struct or one global event bus. Different resources have different owners. A chat session, workflow run and external connection can each have an appropriate lifetime without becoming competing engines.

## 4. What the abstractions mean

A **capability** describes a provider operation and its contract. A **saved action** stores a configured reference to behavior. A **universal action** is an internal executable representation. A **flow node** participates in graph data/control scheduling. An **application operation** implements a user-facing use case. An **adapter** translates a particular interface into that operation.

An application operation can be an existing method or a focused function. It does not require a new `Service`, `Manager`, trait, facade or RPC layer.

```text
Deck intent / CLI command / native assistant tool / MCP request
    -> authenticate source and parse input at the adapter
    -> resolve and authorize the requested operation
    -> existing invocation service or graph repository/runtime operation
    -> persist the appropriate result and return an honest response
```

For effects, `ActionInvocationService` earns its existence through identity, durable intent, owned execution and receipt persistence. For graph edits, `replace_inactive_flow_at_revision` already performs meaningful checks and conditional persistence. A new method that only forwards to either adds no new architectural guarantee.

### FlowService and other scaffolding

H01 removed `flow/flow_service.rs`, the placeholder `Runtime`/`CoreRuntime`/`Lifecycle`/`LifecycleManager` surfaces, and the root no-op `FlowRegistry`. The old wrapper predated v0.6.9 and was not the working production engine. Do not recreate these files from earlier proposals.

The new `services/flow/runtime.rs::FlowRuntime` has a meaningful responsibility: shared startup/recovery, durable-ingress installation and event-router task ownership. Tauri supplies its existing executor/registry/chat handles. This supports the intended architecture without another executor or forwarding-only facade. Its current stop path still needs admission closure and complete task joining, described in F03; the passing H01 delay/reconnect proof does not establish every shutdown race.

`VortiDeckCore` is used for real plugin work, so deleting the whole type would be wrong. However, its constructor creates a `StateManager`, `MacroEngine` and older `IntegrationManager`, while `main.rs` separately creates the live desktop state and universal provider manager. The older integration manager's `get_integration` returns `None`. This is evidence of overlapping ownership and misleading APIs, not proof that two live OBS connections currently exist. Audit consumers field by field, inject shared handles where needed, and remove unused ownership.

## 5. Communication: use the mechanism that fits the guarantee

| Communication | Appropriate mechanism | Required behavior |
|---|---|---|
| Request with a result | Direct async method or bounded request/reply channel | Typed input/result, caller identity, timeout and explicit ownership. |
| UI/provider status projection | Broadcast or latest-value watch | Bounded delivery, lag handling and snapshot resynchronization. |
| Consequential workflow input | Explicit durable ingress/inbox before live notification | Acceptance means persisted input; failure does not masquerade as acceptance. |
| Work assigned to a resource owner | Bounded queue/channel | Capacity policy, cancellation boundary and observable rejection. |
| Cross-process client | Authenticated transport adapter | Same operation semantics, protocol compatibility and redacted output. |

The recent stream path persists matching consequential input through `DurableStreamEventIngress` before EventBus publication. Nonmatching observation traffic need not all become durable workflow history. `EventRouter` still handles the live projection and duplicate detection; it is not the universal command dispatcher. Webhooks legitimately keep request/response-specific coordination.

Do not route every module call through the event bus. Do not persist every animation/progress tick. Do not assume a larger broadcast buffer creates durable delivery. Tokio broadcast explicitly reports lag and permits the receiver to continue; the consumer must implement the policy. [Tokio broadcast](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html)

**Remaining concrete bridge issue:** `src-tauri/src/adapters/state_manager.rs::create_event_bridge` uses `while let Ok(...) = receiver.recv().await`. A lag error exits this bridge permanently, so subsequent state-to-Tauri events stop forwarding. The newer workflow router correctly matches `Lagged` separately. Repair this older bridge with explicit lag/closed handling and a relevant state resnapshot; prove a post-overflow update reaches its consumer. This is a localized correctness issue, not a reason to replace the bus.

## 6. Rust and performance decisions that matter here

### Blocking database work is the first performance investigation

`db/database.rs` uses synchronous Diesel pool acquisition and SQL inside `async fn`. `flow/runtime_storage.rs` implements async storage methods using synchronous rusqlite connections/transactions. Its `connection()` opens a connection per call and configures a five-second busy timeout. These methods can occupy Tokio workers while waiting on disk, pool availability or a write lock.

This is a demonstrated blocking mechanism in source, not a measured claim that a normal deck click currently takes seconds. The risk becomes relevant under concurrent events, checkpoints, history reads and startup/recovery. Profile that workload before changing the storage engine.

Recommended first implementation: one bounded database execution boundary around each complete synchronous transaction, acquiring the pool connection inside the blocking worker. Use owned request data and hold capacity until the blocking job actually finishes. Do not launch unlimited `spawn_blocking` tasks or split one SQL transaction across multiple tasks. A dedicated connection-owning worker is an alternative if measurements show per-call overhead or contention warrants it. Keep Diesel/rusqlite and the current schema initially.

Tokio documents that started `spawn_blocking` work cannot generally be aborted; a timeout can stop waiting without stopping the operation. The current Extism path correctly adds an explicit runtime cancellation mechanism rather than assuming dropping a future stops WASM. Preserve that pattern and report uncertain effects accurately. [Tokio blocking work](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)

SQLite WAL supports concurrent readers with a writer, but does not provide unlimited simultaneous writers. Verify connection reuse, busy policy, transaction duration and checkpoint behavior together. The `synchronous` setting currently appears in initialization while normal connections are recreated: inspect effective per-connection settings before making any durability or speed claim. Do not switch synchronization off to win a benchmark. [SQLite WAL](https://www.sqlite.org/wal.html)

### Lock data briefly; preserve resource ownership

`Arc` is appropriate for shared runtime handles, not a sign of bad Rust. `FlowExecutor::clone` shares its internal handles; it does not create a new independent executor. Copying a whole graph/result map is a different cost and deserves separate measurement.

Use a synchronous mutex for short data-only critical sections with no await. Use an async mutex where suspension while owning a resource is intentional. An async mutex being legal across await does not make a global lock around a long operation desirable. `UnifiedToolExecutor` already obtains a tool under its registry lock, drops the guard, then awaits execution: keep that good boundary. [Tokio mutex guidance](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html)

At several host/engine flow paths, an outer `Arc<Mutex<Option<FlowExecutor>>>` guard remains held while calling async executor operations. For immutable initialized handles, obtain a clone briefly and then perform the operation. Retain appropriate synchronization for replacement/bootstrap; do not blindly remove every lock.

Use actors when one task should own a stateful session or connection, as the chat actor already does. Do not convert every domain object into an actor or add channels around cheap synchronous validation.

### Keep the scheduler; measure its actual costs

The run-owned `FuturesUnordered` scheduler and distinct per-flow/global/resource limits are sensible. The global permit fixes prevent a composite action consuming the permit its child requires. Current concurrency tests passed at limit one and under bounded load.

The scheduler currently clones node-result maps and recalculates reachability/readiness as nodes finish. This is a candidate for increasing cost on larger graphs, not an established live-stream bottleneck. Benchmark chains, fan-out/join and branch-heavy graphs at representative sizes and payloads. Only then consider immutable compiled adjacency/index data keyed by graph revision and smaller run-state snapshots. Do not add a second compiled graph format or optimize away recovery semantics.

Measure separate latency components: queue wait, authorization/resolution, database work, provider round trip, receipt confirmation and UI delivery. Record p50/p95/p99 and allocations/retained state where useful. A synthetic test's 66 ms total does not establish a production responsiveness target.

### Supervise long-lived tasks

The desktop starts several background tasks, watchers and bridges during composition. Before H01 creates another host, give each long-lived task an owner, cancellation signal, shutdown order and observed join result. Stop ingress, finish or classify accepted work, persist state, then close providers and storage. Do not abort all tasks and declare a clean shutdown. Tokio's shutdown guidance separates cancellation notification from waiting for task completion. [Graceful shutdown](https://tokio.rs/tokio/topics/shutdown)

An explicit handle struct is sufficient. Empty lifecycle classes or a dependency-injection framework do not solve this.

### Make Rust contracts precise where decisions depend on them

Use enums/newtypes for IDs, result states, revision conflicts and public error categories where accidental interchange or string matching changes behavior. Keep `serde_json::Value` for dynamic plugin payloads and preserved documents; it should not replace the control structure of the API. Use `thiserror` for domain errors and contextual errors at host boundaries. Migrate touched boundaries rather than rewriting all strings at once. The Rust API Guidelines recommend type distinctions that prevent invalid use. [Type safety](https://rust-lang.github.io/api-guidelines/type-safety.html)

Traits are justified for provider implementations, storage fixtures, clocks and true platform facilities. A trait for every struct is not a requirement. Dynamic dispatch and `async_trait` in plugin/storage interfaces are not automatically performance problems; do not replace them with large generic hierarchies without measurements.

Reduce public exports to actual consumers and use `pub(crate)` for internal operations. D03 removed the core crate's broad warning allowance; the desktop binary still has its own allowance. Start with suspicious unused results, unsafe state and lifecycle defects; do not spend a release fixing hundreds of cosmetic names. The 6 September focused core/host commands reported 252 core warnings; the older clippy-error totals are a separate diagnostic baseline. F08 reconciles those checks with release policy.

## 7. Persistence and recovery: preserve the hard-won distinctions

The main database and runtime database serve different data lifetimes. Two stores are not inherently a mistake. They do mean graph persistence, trigger projection, invocation receipts and workflow outbox cannot all be assumed to commit atomically together.

Retain existing R05 reconciliation and R08 retention/quarantine behavior. The complete core suite now tests forced process termination, uncertain effects, deduplication, atomic approvals and readback. Add a cross-store test for a new transition when its consistency requirement demands one; do not migrate everything to one ORM merely for stylistic uniformity.

Keep these distinctions in every new interface:

- A request was accepted versus an effect was applied.
- A flow launch invocation completed versus the workflow run completed.
- A cancellation request was recorded versus an external operation stopped.
- The provider supports a repeated idempotency key versus an operation might execute twice.
- A persisted graph revision exists versus all runtime references/permissions still permit resume.
- A browser rendered a widget versus viewers saw it on the final stream.

No general exactly-once promise is justified for arbitrary external effects. Recovery is per capability, with explicit uncertain/manual-reconciliation behavior.

## 8. Widgets, overlays, plugins and the copilot

### Widgets and overlays

Keep the shared definition/instance/binding/intent model and the separation of requested and observed state. Declarative portable rendering is useful for mobile, while sandboxed desktop plugin views serve a different trust/deployment need. Different renderers are not automatically duplicate architectures.

Overlay composition and deck interaction have different responsibilities. Share schemas, asset validation, permissions and effect contracts where semantics match. Do not force scene layering, browser render ACKs, rotary input and mobile layout into one universal component class. Preserve compatibility renderers until the documented migration/rollback gates pass.

O01/O02 and W02 still need their installed OBS/physical-client evidence. Passing core/browser-policy fixtures should not close those gates. A token refresh fix must be proven through source URL, page authorization, WebSocket and rendered outcome in the installed combination.

### Plugin trust and lifecycle

Keep package identity, content/version compatibility, consent/grants, runtime activation and live frame revocation connected. A manifest claim is not authority; host-bound identity and effective grants are.

WASM host-call restrictions and service-process isolation are different trust models. `service_plugin.rs` launches an ordinary configured child process. Its RPC host methods check grants, but those checks do not prevent native child code from opening files or sockets directly with its OS account privileges. Describe service plugins as trusted executable extensions unless an actual OS sandbox is implemented. Separate-process execution improves failure containment; it is not automatically a security sandbox. This is a contract clarification, not evidence of a newly exploited vulnerability.

### Copilot and external agents

Keep the recent authoritative reads, inactive revision-checked replacement, ordered call/result identity, conversation persistence and common streamed/nonstreamed task lifecycle. These are more valuable than introducing a second “agent engine.”

Use a bounded loop: inspect actual state, propose a change, validate, apply within authority, read persisted results, then explain the outcome. Pure planning/computation may be direct tools; effects use the existing invocation contract. For a future autonomous watcher, start from events/schedules and explicit budgets rather than an unconditional model polling loop.

Outbound MCP is implemented and now has real transport tests. Inbound MCP is a proposed external control adapter. A scoped inbound adapter must preserve authority through nested flows, saved bindings and resumed work; hiding tools from discovery is not sufficient enforcement. Do not re-export all outbound servers or embed Hermes by default.

## 9. Repository layout, code generation and agent usability

The existing apps/layers/packages layout is understandable enough to extend. The folder name `tauri-windows` containing shared core is imperfect, but a whole-repository move does not improve execution safety. First reduce actual dependency violations and misleading duplicate entry points. Extract small Rust crates when a real consumer needs independent dependencies/builds, not to make a diagram symmetrical.

Recommended near-term organization inside the current core: execution owns invocation/effect contracts; flow owns documents, scheduling and recovery; integrations own provider contracts; plugins own package runtime/grants; MCP owns protocol/client/tool adaptation. Split very large files into cohesive private modules while preserving one owner and API. Do not replace one large file with many mutually dependent “manager/service/factory” files.

**SDK generation has been corrected under D02.** At the initial review snapshot, `vortideck-core/build.rs` wrote generated PDK/SDK files into examples and `packages/plugin-sdk`, with warning-only write failures. The follow-up inspection on 5 September found that the implementation agent had replaced those writes with read-only stale-artifact checks and added explicit generation/check commands. Do not assign the old defect again. D02 was marked complete during that follow-up with local contract/build evidence; the comparison records residual validation questions and the transition to D03.

Keep Rust-driven generation and the explicit command. Verify deterministic output, Windows checkout line endings, detection of edited generated artifacts, and the documented plugin build/load contract. Build-local generated artifacts, if any are needed, belong in `OUT_DIR`; required generation failures should fail the relevant check. Do not add an otherwise unused `OUT_DIR` generation pass merely to satisfy an architectural diagram. [Cargo build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html)

Agent-friendly means accurate entry points, reproducible commands, bounded tests, explicit generated-file ownership and current contract examples. It does not mean an agent can safely infer the active architecture from hundreds of contradictory Markdown files. Keep one completion ledger; attach short decisions to changed contracts; record source/test evidence and limitations. Do not duplicate a full backlog in every subsystem.

## 10. Document reconciliation

The inventory scanned 382 discoverable Markdown files (5.25 MiB, including hidden repository planning files, excluding dependency/build/scratch directories under the stated filters). This is not a line-by-line audit of all those files. Current contract, direction and implementation-evidence sections received targeted review; the coverage register labels that distinction. Authorship cannot be inferred from filenames, so the inventory is repository-wide rather than a claim about who wrote each document.

| Document/family | Retain | Clarification or supersession |
|---|---|---|
| Root TODO and execution briefs/notes | Priorities, evidence, explicit remaining gates | Keep as completion authority. Some notes show newer automated evidence while root boxes remain open for UI/device checks; that is not automatically a contradiction. |
| Flow/stream/action direction | Capability/reference/invocation model and readiness | “One invocation boundary” applies to action effects, not every graph query/editor operation or all communication. Phase titles naming the engine should not hide the service's actual durable role. |
| CLI/MCP design | Thin clients, shared existing behavior, honest receipts, bounded transport scope | Remove compulsory new control facade and compulsory FlowService revival. New files/packages are proposed only when their slice needs them. CLI is complementary to MCP, not the preferred agent interface by default. |
| Product direction and 2026–2030 strategy | Inspectable routines, operational feedback, optional AI | Hermes is an optional reference/integration candidate, not an existing required router. Focus the near-term product on the live studio. Long-range predictions are scenarios. |
| Agent-system vision | Agents can call flows; flows can request AI reasoning | A flow can wait for hours/days. Lifetime alone does not distinguish agents from workflows. Separate actor loops/UI/platform abstractions are proposals, not automatic requirements. |
| AI copilot/MCP architecture | Inspect/design/command experience | Retire “MCP 2.0” and WebSocket-as-standard implications. Use A06's tested outbound matrix; mark inbound exposure as proposed. |
| Cloud suggestion registry | Optional updated suggestions/personalization | Remove mandatory cloud-generated-only chips. Useful local defaults and offline behavior are allowed; cloud telemetry is a separate product/privacy decision. |
| Widget/overlay specifications | Portable contracts, host bindings, rendering receipts and migration criteria | Keep renderer differences where semantics differ. “Implemented” is scoped; physical topology and installed companion gates remain. |
| Human approval specification | Durable decision authority independent of notification channel | Do not require a new generic notification gateway before a bounded local use case. A safe future notification adapter can implement the specified authentication/delivery contract. |
| Marketplace/installer/relay specifications | Artifact provenance, lifecycle, routing identity and actual endpoint ownership | Keep separate from execution orchestration. This pass reviewed their architecture claims; it did not revalidate the sibling backend, release infrastructure or installed binaries. |
| Sponsor/monetization proposal | Scheduling and exportable activity evidence | A signed local receipt does not prove audience exposure or prevent a compromised signer lying. “Un-hackable,” “100% AdBlock-proof” and verified-impression claims need removal or a specific independently verified measurement model. Do not build billing assumptions on render ACKs. |
| Old flow/core separation, universal migration and architecture-summary documents | Historical rationale and reference | Current source wiring overrides copied file trees and “headless ready/zero platform dependencies” assertions. Do not restart those migrations. |
| Plugin docs/SDK plans | Capability schemas, runtime choices, developer examples | Reconcile runtime-specific trust, removed modules and generated exports. No universal “all native child behavior is permission-sandboxed” claim. |
| Old compiler-warning and performance tables | Historical investigation leads | Re-measure current code; deleted modules and illustrative timing tables are not current defects or benchmarks. |

## 11. What to do next, without starting another rewrite

Follow POST_PLAN_STABILIZATION.md and root TODO. O01/O02, S02/S04 and A01/A02 now have recorded live acceptance; do not repeat them as missing. W02 remains open. Later active-overlay persistence needs its own cold-start proof (F01), and recipe review/node identity, shutdown, lag handling, plugin locks, storage scheduling, build feedback and remote replay have scoped follow-ups F02–F10.

H01 has already removed the old scaffolding and introduced the useful lifecycle owner. D02/D03 already implemented generation and truthful tooling. Complete their residual evidence rather than restarting those migrations. After stabilization, choose one useful product or external-control slice. No daemon, general gateway, universal service layer or cloud host is a prerequisite.

## 12. Validation from this reassessment

Executed against the current working tree:

```powershell
$env:CARGO_TARGET_DIR = 'F:\vd-target'
cargo test --locked --manifest-path apps/tauri-windows/libs/vortideck-core/Cargo.toml
```

Result: 173 unit tests, 33 event/runtime integration tests, 3 MCP transport integration tests and 1 doctest passed; 3 doctests intentionally ignored. The integration suite includes isolated process-termination fixtures. This validates these repository boundaries, not a live OBS installation, production remote service or all desktop host behavior.

Also executed the three concurrency lifecycle tests explicitly: all passed; the stated 12-leaf, capacity-2 synthetic workload observed maximum concurrency 2 and 66 ms elapsed in that run. Do not generalize this number to production latency.

From `apps/tauri-windows`:

```powershell
pnpm exec vitest run ../../layers/flow/utils/flowSerialization.spec.ts ../../layers/base/composables/useWidgetInteractionRouter.spec.ts ../../layers/desktop/utils/pluginEventScope.spec.ts --config vitest.config.ts
```

Result: 3 files, 12 tests passed. No UI automation or external service mutation was performed. Source inspection identified the blocking SQL and lag-sensitive bridge; no measured production latency or reproduced live UI outage is claimed. Reviewed-source hashes and final drift checking accompany the evidence log.

The composed desktop also passed `cargo check --locked --manifest-path apps/tauri-windows/src-tauri/Cargo.toml --bin VortiDeck` using the same short target directory, completing in 21.72 seconds. This is compilation evidence, not a desktop/OBS end-to-end test.
