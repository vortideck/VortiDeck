# VortiDeck hosting scope decision

Status: accepted discovery decision for the 0.7 architecture, 5 September 2026.

Review qualification, 6 September: retain this scope decision. The post-plan review identifies admission/task-joining work in FlowRuntime (F03) and deadline/error-isolation/capability requirements for LocalFlowHost before shipping. The passing bounded proof is not evidence that those operational cases are complete.

## Decision

Keep execution local-first. The next host mode, if product demand justifies it,
is an **optional local background process using the same `FlowRuntime`**, not a
cloud execution engine. The existing remote backend remains a control-plane
relay between authenticated devices and an online desktop owner.

A general self-hosted runtime and cloud-owned flow execution are deferred. They
may be reconsidered only for a concrete flow whose providers, credentials,
offline policy and receipt semantics can all be implemented without implying
that a remote process controls an offline OBS session, keyboard or window.

## Compared modes

| Mode | Concrete useful scenario | Available capabilities | Identity boundary | Offline / expiry policy | Credential owner | Operational cost | Decision |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Desktop runtime | Normal deck, OBS, overlay, stream-event and system automation while VortiDeck is open | Full registered desktop/provider capability set | Logged-in local VortiDeck profile plus paired-device/session identity | Local work continues without the cloud when its provider is local; remote commands expire according to their envelope | OS keyring/local encrypted state or provider-specific store | Existing application lifecycle | Primary supported mode |
| Optional local background host | Keep an allowed saved timer, webhook or stream workflow alive after the editor window closes | Only providers advertised by that host; interactive window/input/OBS actions require the user's desktop session and must be reported unavailable otherwise | One local runtime owner; editor is an authenticated loopback client | Runs while the PC and user host are running; interruption becomes explicit recovery work | Same machine, but a daemon needs an explicit unlocked-session/secret policy | Packaging, autostart, upgrades, locking, diagnostics and support | Recommended next mode only after UX/ownership gates below |
| Self-hosted runtime | A creator runs 24/7 schedule/API/AI-to-Discord workflows on a home server | Portable HTTP, schedule, AI and explicitly compatible plugins; no implicit desktop/OBS/input capabilities | Separate host installation, user/service identity and provider grants | Continues when the creator PC is off; local effects are rejected or sent as expiring requests to an online worker | Self-hosted secret store administered by the user | Cross-platform releases, upgrades, backups, TLS, support and migration | Deferred until one demanded portable scenario earns a target |
| Cloud coordinator with local worker | A paired mobile client triggers a saved action while away from the LAN and the desktop is online | Routing, identity, delivery state and bounded coordination; effects execute on the selected local worker | Cloud account/device/session identity maps to a specific registered local target | If the worker is offline, reject or queue only with an explicit deadline; never claim an applied local effect | Cloud owns relay credentials; local/provider secrets remain local | Existing backend plus stronger atomic session, expiry and observability work | Keep as control plane; do not turn it into the flow owner |
| Cloud-owned flow runtime | Server-side schedules/API workflows without any local dependency | Only cloud-installed portable providers | New tenant isolation, execution authority, quotas and billing boundary | Requires durable cloud queues and explicit downstream expiry | Managed multitenant secret service | Highest: isolation, abuse controls, billing, regional operations and incident response | Rejected for the current roadmap |

## Why the local host is the only current candidate

H01 proves that the desktop composition and a non-UI adapter can share one
runtime lifecycle, durable store and event router. Its loopback test executes a
saved flow, reconnects with a fresh client, rejects desktop-only capability use,
joins cleanly and recovers work interrupted by shutdown. That is enough to
validate the boundary without shipping a second application engine.

The proof does **not** authorize concurrent writers. Exactly one process owns a
runtime database and executor. A future editor either embeds that owner, as it
does today, or connects to the background owner as a client. It must never open
the same writable runtime store independently.

## Gates before shipping a background process

1. Select one launch model per platform (user-session background process is
   more honest than a Windows service for UI/OBS work).
2. Implement an OS-level single-owner lease covering both desktop and
   background startup, including stale-owner and upgrade handling.
3. Store and rotate the local control credential outside command-line arguments;
   restrict its ACL to the signed-in user.
4. Add an editor transport switch with protocol/version negotiation and a clear
   “background host unavailable” state.
5. Define which providers may run while the editor is closed and which require
   an unlocked interactive desktop session.
6. Make install, update, rollback and uninstall stop the owner cleanly without
   losing pending durable work.
7. Add process-level tests for concurrent start rejection, editor/host version
   mismatch, forced termination, OS sign-out, upgrade and recovery.

The transport tests must also cover an unauthenticated client holding a request
open, a client disconnecting during response, and a saved graph requesting an
unavailable provider. Bound request/response time and isolate per-connection
errors; derive/enforce capabilities from the actual host composition. Do not
promote the proof's serial EOF-delimited protocol unchanged merely because its
happy-path test passes.

Until those gates are implemented, `LocalFlowHost` is a bounded architectural
proof and reusable adapter, not a user-visible 0.7 feature.

## Rejected shortcuts

- Do not copy Tauri's `main.rs` into a server binary or construct another flow
  executor beside it.
- Do not expose the local protocol to the LAN. Remote access belongs behind the
  authenticated relay/device boundary.
- Do not call queued relay delivery “execution completed.” Provider-applied and
  visual/audience receipts remain distinct.
- Do not place OBS, input or window credentials/capabilities in a cloud worker
  and imply they operate while the user's machine is off.
- Do not begin with Kubernetes, generic runner pools, multiregion execution or
  billing. None is required by a proven current scenario.

## Existing reusable pieces

- `FlowRuntime`, `FlowExecutor`, durable runtime storage and `EventRouter` from
  H01/R01–R08.
- `LocalFlowHost` for the authenticated loopback proof.
- Existing desktop/mobile relay adapters and device/session identity, subject
  to the I02 race and threat-model validation.
- The action invocation/provider capability contract for truthful availability
  and receipt levels.

## Follow-up boundary

No new executable target is approved by this decision. If background operation
becomes a release goal, create a separate implementation task limited to the
seven shipping gates above. If a 24/7 portable self-hosted scenario is requested,
first document its exact provider set and acceptance test; do not broaden the
local daemon task into a generic server.
