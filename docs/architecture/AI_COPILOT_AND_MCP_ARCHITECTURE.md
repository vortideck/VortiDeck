# VortiDeck AI Copilot & Modern MCP Architecture

> **Architecture clarification, 5 September 2026:** See the [consolidated guide](ARCHITECTURE_GUIDE.md) and A06 transport evidence. Outbound MCP and internal tools are implemented; inbound exposure is proposed. “MCP 2.0” and standard-WebSocket implications below are historical terminology, not supported-protocol claims. Shared behavior does not require another wrapper/service hierarchy.

> **Planning update, 5 September 2026:** Retain inspect/design/command goals as direction. Transport names, tool lists and checkboxes below do not establish a wired production copilot. A02–A06 require real tools, revision-bound saved edits, task lifecycle and actual MCP interoperability; verify the official protocol before treating “MCP 2.0” as a requirement. Use root TODO and the detailed backlog for current task status and acceptance evidence.

**Date:** August 2026  
**Scope:** AI Copilot, Native MCP Tools, Multi-Provider LLM Sessions, Modern Streamable HTTP / WebSocket Transports, and Flow Integration.

---

## 1. Executive Direction

VortiDeck is a **Visual Real-Time Orchestration Platform for Live Streamers & Creators**.

The AI system within VortiDeck is **not** an isolated generic chatbot or an unbounded runaway autonomous agent. It is the **Intelligent System Operator & Workflow Copilot**:

> **"VortiDeck turns live events into deterministic actions; the AI Copilot provides the natural-language intelligence to command, design, and diagnose the studio."**

---

## 2. The Dual-Role MCP Architecture

You write native VortiDeck tools once in Rust, and they serve **both internal and external roles**:

```text
                                  ┌──────────────────────────────────────────────┐
                                  │      VORTIDECK NATIVE MCP TOOL SUITE         │
                                  │  (list_flows, run_macro, obs_action, inbox)  │
                                  └──────────────────────┬───────────────────────┘
                                                         │
                         ┌───────────────────────────────┴───────────────────────────────┐
                         ▼                                                               ▼
        [ 1. INTERNAL COPILOT ENGINE ]                                  [ 2. EXTERNAL MCP SERVER ]
         UnifiedToolExecutor (Rust)                                      rmcp::ServerHandler
                         │                                                               │
                         ▼                                                               ▼
        ┌───────────────────────────────────┐                           ┌───────────────────────────────────┐
        │ • Floating Assistant Window       │                           │ • Claude Desktop / External Agents│
        │ • Canvas Flow Copilot Drawer      │                           │ • Streamable HTTP (JSON-RPC)      │
        │ • In-Flow "RunAgent" Nodes        │                           │ • WebSocket (WS Duplex Streaming) │
        └───────────────────────────────────┘                           └───────────────────────────────────┘
```

---

## 3. The Native VortiDeck Tool Suite

| Tool Name | Parameters | Purpose & Execution Path |
|---|---|---|
| **`vortideck_list_flows`** | `filter_active: bool?` | Queries saved flows and returns flow IDs, names, active trigger types, and node count. |
| **`vortideck_get_flow`** | `flow_id: string` | Retrieves the full visual DAG graph structure (nodes, inputs, connections) for inspection or editing. |
| **`vortideck_run_flow`** | `flow_id: string, payload: json?` | Dispatches a workflow directly onto the [`FlowExecutor`](../../apps/tauri-windows/libs/vortideck-core/src/services/flow/executor.rs) with an isolated `run_id`. |
| **`vortideck_run_macro`** | `macro_id: string` | Executes a registered macro or action directly. |
| **`vortideck_obs_action`** | `action: string, scene_name: string?, source: string?, mute: bool?` | Performs instant OBS studio controls (switch scene, toggle source, mute/unmute) via `EventBus`. |
| **`vortideck_query_inbox`** | `limit: number?, event_type: string?, since_minutes: number?` | Searches the SQLite `workflow_event_inbox` for recent stream/webhook events to answer questions and diagnose issues. |
| **`vortideck_send_overlay_cmd`** | `widget_id: string, command: string, payload: json` | Dispatches a typed, targeted overlay animation command to browser HUD widgets. |

---

## 4. Modern MCP 2.0 Transports (Streamable HTTP & WebSocket)

VortiDeck moves away from legacy SSE (Server-Sent Events) and process pipes towards modern protocols:

1. **Streamable HTTP:** Direct bidirectional JSON-RPC over chunked HTTP POST requests with Bearer API tokens.
2. **WebSocket (WS):** Full-duplex persistent channel for low-latency desktop-to-desktop or LAN streaming control.
3. **Security:** Configurable OAuth 2.0 (PKCE) or scoped API Keys (`vd_sec_xxxxxxxxxxxx`) generated in VortiDeck Settings.

---

## 5. User Interaction Modes

### Mode A: The Floating Command Window (`assistant-window.vue`)
- Streamer has a dedicated floating / second-monitor chat window open during their live broadcast.
- Direct voice or text orders:
  - *"Switch to BRB Scene and mute my mic."* $\rightarrow$ Executes `vortideck_obs_action`.
  - *"Who was the last subscriber?"* $\rightarrow$ Executes `vortideck_query_inbox`.
  - *"Run my End Stream macro."* $\rightarrow$ Executes `vortideck_run_macro`.

### Mode B: The Canvas Flow Copilot
- Lives as an interactive assistant alongside the visual Flow Canvas.
- Workflow creation & modification:
  - *"Create a raid alert flow when a raid has >20 viewers."* $\rightarrow$ Places nodes and connects edges on the canvas.
  - *"Add a 60-second delay node and switch back to Game Scene."* $\rightarrow$ Inserts timer and scene node.

### Mode C: In-Flow AI Nodes (`RunAgentNode`)
- A deterministic node in a flow graph that runs one-shot AI inference (e.g. sentiment analysis, superchat summarization, or dynamic text generation).

---

## 6. Implementation Roadmap

- [x] EventBus live backbone & EventRouter SQLite inbox.
- [x] Bounded DAG FlowExecutor with `run_id` isolation and cancellation.
- [x] Webhook Ingress unification & Modern Vue/Pug UI.
- [ ] **Phase 1: Native VortiDeck MCP Tools** in [`tools.rs`](../../apps/tauri-windows/libs/vortideck-core/src/services/mcp/tools.rs).
- [ ] **Phase 2: Default ToolSet Registration** in [`chat.rs`](../../apps/tauri-windows/libs/vortideck-core/src/services/mcp/chat.rs) and [`unified_tool_executor.rs`](../../apps/tauri-windows/libs/vortideck-core/src/services/mcp/unified_tool_executor.rs).
- [ ] **Phase 3: Clean Streamable MCP Server** in `server.rs`.
- [ ] **Phase 4: Copilot Command Hub UI** integration in [`assistant-window.vue`](../../layers/desktop/pages/assistant-window.vue).
