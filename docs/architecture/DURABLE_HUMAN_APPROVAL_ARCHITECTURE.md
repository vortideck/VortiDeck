# Durable Human Approval Architecture

Status: core architecture and desktop/mobile adapters implemented; desktop presentation and native-notification hardening completed on 2026-09-04. Email and outbound-webhook delivery are intentionally not advertised until a configured notification gateway exists.

## Decision

Human Approval is one durable workflow primitive, not a desktop modal node, mobile node, email node, or webhook node. A workflow creates one approval request. Any installed notification adapter may present it, but every answer returns to the same approval service and durable record.

```text
workflow approval node
    -> durable approval wake (authority)
        -> ApprovalRequested event
            -> desktop notification adapter
            -> paired mobile adapter (local or remote relay)
            -> future email/webhook notification gateway

authenticated decision adapter
    -> resolve_approval_wake(wakeId, decision, actor, comment)
        -> atomic compare-and-set
            -> ApprovalResolved event
            -> approved / denied / timed_out route
```

This boundary is provider-neutral and does not depend on OBS, Discord, overlays, a particular UI, or a particular transport.

## Implemented contract

The durable `WorkflowWake` remains the storage envelope. An approval wake contains:

- a globally unique wake ID, run ID, and node ID;
- `waiting`, `ready`, `consumed`, or `cancelled` state;
- an optional absolute deadline;
- title, message, severity, requested notification channels, and a non-secret operation snapshot;
- a decision policy listing authenticated channels allowed to resolve it;
- after resolution, the decision, timestamp, authenticated actor, transport channel, and optional comment.

Decisions are explicit values: `approved`, `denied`, `timed_out`, and `cancelled`. Timeout and cancellation are never collapsed into denial, because that would make routing and audit history dishonest.

The workflow outputs are:

- control routes: `approved`, `denied`, and `timed_out`;
- data: `decision`, `isApproved`, `wakeId`, `decidedAt`, `decidedBy`, and `comment`.

The editor renders named control ports as `flow-output-{route}`, but persists them as typed control edges with a `route` field. The scheduler reads that field directly, recalculates the reachable graph after a control result is known, removes unselected branches, and preserves correct join behavior.

## State and crash semantics

The approval request is committed before any notification is emitted. A decision is committed before a live waiter or recovered workflow is notified.

```text
waiting --human decision--> ready --node checkpoint--> consumed
waiting --deadline-------> ready(timed_out) ----------> consumed
waiting --run cancel-----> cancelled/cancelled result
```

Resolution uses an immediate SQLite transaction and a conditional state transition. The first valid decision wins. Retrying the same decision returns the persisted result; a conflicting later decision is rejected. This makes reconnects and duplicate mobile messages safe.

If the process stops while waiting, startup reloads the workflow and wake. If the decision was already committed, recovery materializes the node outputs and resumes only the selected branch. If no decision exists, the request remains pending. There is no in-memory-only approval authority.

## Authentication and authorization

The core accepts a decision only with an actor ID and channel. Current authorized channels are configurable per node and default to:

- `desktop` — the local desktop operator;
- `mobile_direct` — a paired device over the authenticated local WebSocket;
- `mobile_relay` — the same paired device through the remote relay.

Direct and relay mobile commands already pass through HMAC signature and replay checks before reaching the approval adapter. Remote callers may submit only `approved` or `denied`; only the workflow runtime may create `timed_out` or `cancelled` decisions.

The policy is persisted with the request, so authorization does not change accidentally if defaults change while a workflow is waiting. Actor roles, named approver lists, quorum approval, and delegation can later be implemented by replacing or enriching the policy evaluator; they do not require another scheduler or storage model.

## Notification channels are not decision authorities

Desktop and mobile delivery are implemented today. The node UI advertises only those working adapters.

Email and webhook support should be added through one notification gateway, not by letting the local workflow engine send arbitrary URLs or embedding a reusable approval secret in a message. The gateway contract is:

1. Consume the canonical `ApprovalRequested` event.
2. Apply configured recipient/template and outbound destination policy.
3. Create a short-lived, single-use capability whose secret is stored only as a hash.
4. Send an email link or a signed webhook notification containing the public request reference, not workflow secrets.
5. Authenticate the person or verify the webhook signature when the link/callback is used.
6. Exchange that capability for one correlated decision through the existing remote relay adapter.
7. Mark the capability used and reject expiry, replay, changed payloads, and conflicting answers.

Email is therefore a presentation and authentication handoff. A generic outbound webhook is a machine-notification adapter. Neither bypasses `resolve_approval_wake`, and neither can claim success merely because delivery returned HTTP 2xx.

The repository currently has no production notification gateway, email provider configuration, or safe outbound-webhook destination registry. Fabricating a direct URL sender in the desktop process would introduce SSRF, secret-distribution, availability, and audit problems. Keeping unavailable channels out of the editor is a deliberate completeness rule, not missing core architecture.

## Product surfaces now connected

- Desktop receives the canonical flow event and, by default, shows one correlated in-app notification plus a persistent pending-approval badge. Users may instead choose immediate dialog presentation in General Settings.
- The notification Review action opens and focuses the same approval inbox. It identifies the exact durable wake and never loads a saved flow over the editor's current unsaved draft.
- Desktop notifications are transient projections and are never persisted or cross-window serialized. On launch, the UI queries durable waiting wakes and discards resolved, timed-out, or cancelled requests.
- Optional native operating-system notifications are implemented for periods when every VortiDeck window is unfocused. They point back to the same wake and have no authority to resolve it.
- Mobile requests an authoritative snapshot after pairing/reconnection, receives live requests, displays context and deadline, allows an optional comment, and submits signed approve/deny commands.
- Both local mobile and internet relay paths use the same command adapter and actor model.
- Resolution events remove stale requests from mobile clients.
- Runtime operations can inspect and resolve pending approvals on desktop.

## Default desktop behavior

With the default node settings, reaching Human Approval pauses the run and creates a durable request. It does not approve, deny, or continue automatically. The default desktop presentation is a non-interrupting notification and pending badge; Review opens the request with its title, immutable run context, optional deadline and audit comment, then offers explicit Approve and Deny actions. Users who prefer interruptive behavior can select `Open approval dialog`. If no timeout was configured, the run waits until an authorized desktop or paired-mobile operator decides, or until it is cancelled.

The application does not navigate away from the operator's current work automatically. A main-window or native Review action uses the dedicated `open_workflow_approval` adapter: an existing editor is focused without changing its flow, while a newly created editor receives the wake ID in its initial route. This avoids both the unsaved-changes prompt and the startup event race. Native notifications remain optional and are not required for durable approval semantics.

## Non-goals for the current implementation

The following are product policies, not requirements for a sound first release:

- multi-person quorum or ordered approval chains;
- organization role-directory integration;
- escalation schedules and reminders;
- arbitrary unauthenticated approval URLs;
- automatic approval by AI or by receipt of an email/webhook;
- placing ordinary workflow work into an approval queue.

One request has one durable winner. Multiple simultaneous approvals are independent wake records and may wait concurrently.

## Operational acceptance criteria

- A request survives application termination and restart.
- The request is visible again after a paired phone reconnects.
- The first authenticated answer wins atomically.
- Repeating the same answer is harmless.
- A conflicting answer is rejected.
- Unauthorized decision channels are rejected by the core, not just hidden in UI.
- Timeout is fail-closed and follows only the `timed_out` route.
- Only the selected control branch becomes runnable.
- Notification delivery failure does not resolve the request.
- No transport may report an approval before durable storage accepts it.
- Email/webhook adapters, when installed, use the same request, decision, actor, audit, and recovery contracts.

## Remaining bounded work

The engine architecture is complete for single-person approvals. Remaining work is adapter/product hardening:

1. Add a process-kill end-to-end acceptance test covering request, restart, remote decision, selected branch, and final consumption.
2. Build and operate the notification gateway before exposing email or signed-webhook choices.
3. Add role/named-actor policy only when VortiDeck has a real organization/device permission model.

These additions extend the approval service. None requires a parallel engine, provider-specific approval node, or legacy execution path.
