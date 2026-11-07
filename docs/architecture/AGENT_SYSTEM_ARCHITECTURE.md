# VortiDeck Agent System — Architecture Vision

> **Architecture clarification, 5 September 2026:** This is a vision, not the current runtime map. The [consolidated guide](ARCHITECTURE_GUIDE.md) preserves the existing flow and conversation runtimes. Flow-versus-agent lifetime comparisons below are illustrative and misleading as architecture rules: durable flows can wait for hours or days. A second agent runtime is not required by that distinction.

> **Planning update, 5 September 2026:** Architecture vision, not a description of a complete agent runtime. A03/A04/P02 reconsider fixed polling, lifespan-based flow/agent distinctions and additional runtimes. Begin with one owned task lifecycle, bounded authorized effects and actual evidence. Use root TODO and the detailed backlog for current task status and acceptance evidence.

---

## 1. App Structure Overview

```mermaid
graph TD
    subgraph VortiDeck["VortiDeck App"]
        DECK["🎛️ Deck\nButtons / Hotkeys"]
        FLOWS["⚡ Flows\nVisual Builder"]
        AGENTS["🤖 Agents\nAI Watchers"]
        CONN["🔌 Connections\nMCP Servers"]
    end

    subgraph ToolExec["Unified Tool Executor (existing)"]
        SYS["System Tools\nkeyboard / mouse / clipboard"]
        MCP["MCP Tools\nOBS / Twitch / Custom"]
        FLOW_TOOL["Flow Runner\nrun_flow()"]
        DECK_TOOL["Deck Control\npress_button()"]
    end

    DECK --> ToolExec
    FLOWS --> ToolExec
    AGENTS --> ToolExec
    CONN --> MCP

    VortiDeck -->|"exposes as MCP Server"| CLAUDE["Claude Desktop\nor any MCP Client"]
```

---

## 2. Agent vs Flow — What's the Difference

```mermaid
graph LR
    subgraph Flow["⚡ Flow (ephemeral)"]
        F1["Trigger"] --> F2["Node"] --> F3["Node"] --> F4["Done\n(dies)"]
    end

    subgraph Agent["🤖 Agent (persistent)"]
        A1["Activates"] --> A2["Watch Tick"]
        A2 --> A3{"LLM\nDecides"}
        A3 -->|"call tool"| A4["Tool Executes"]
        A4 --> A5["Log Result"]
        A5 --> A6["Sleep N seconds"]
        A6 --> A2
    end
```

| | Flow | Agent |
|---|---|---|
| Lifetime | Milliseconds, then dies | Runs for hours, persists |
| Trigger | User action / webhook | Autonomous: timer, event |
| Intelligence | Deterministic steps | LLM is the brain |
| State | Stateless between runs | Memory builds over time |
| You define | Every step | Just the goal |

---

## 3. Agent Anatomy

```mermaid
classDiagram
    class AgentDefinition {
        +String id
        +String name
        +String system_prompt
        +String model
        +Vec~String~ tools
        +AgentTrigger trigger
        +AgentMemoryConfig memory
        +bool enabled
    }

    class AgentTrigger {
        <<enumeration>>
        AlwaysOn
        Timer(interval_secs)
        Manual
        EventDriven(Vec~WatchTarget~)
    }

    class AgentMemoryConfig {
        +bool persistent
        +usize max_tokens
    }

    class AgentRuntime {
        +String agent_id
        +bool is_running
        +Vec~Message~ conversation_history
        +Vec~ToolCallRecord~ tool_call_log
        +Option~AbortHandle~ abort_handle
    }

    class ToolCallRecord {
        +DateTime timestamp
        +String tool_name
        +Value input
        +Value output
        +u64 duration_ms
    }

    AgentDefinition --> AgentTrigger
    AgentDefinition --> AgentMemoryConfig
    AgentRuntime --> ToolCallRecord
```

---

## 4. Agent Runtime Loop

```mermaid
sequenceDiagram
    participant App as VortiDeck App
    participant AR as Agent Runtime
    participant LLM as LLM (Claude/Llama)
    participant TE as Tool Executor
    participant EXT as OBS / Twitch / Flow

    App->>AR: start_agent("stream_assistant")
    activate AR

    loop Every 5 seconds (timer trigger)
        AR->>LLM: Send system_prompt + conversation_history + tools list
        LLM-->>AR: tool_use: twitch_read_chat()
        AR->>TE: execute("twitch_read_chat")
        TE->>EXT: Twitch MCP call
        EXT-->>TE: 47 messages, sentiment: frustrated
        TE-->>AR: result
        AR->>LLM: tool result
        LLM-->>AR: tool_use: run_flow("slow_mode")
        AR->>TE: execute("run_flow", "slow_mode")
        TE->>EXT: Execute flow
        EXT-->>TE: completed in 340ms
        TE-->>AR: result
        AR->>LLM: tool result
        LLM-->>AR: "Done. Chat calmed by slow mode."
        AR->>AR: Log decision + append to history
    end

    App->>AR: stop_agent("stream_assistant")
    deactivate AR
```

---

## 5. How Agents and Flows Work Together

```mermaid
graph TD
    subgraph "Direction A — Flow calls Agent"
        WH["Webhook Trigger"] --> HTTP["HTTP Request\n(raw data)"]
        HTTP --> RAN["RunAgentNode\n🤖 Analyze this data"]
        RAN --> ROUTE["Route Node"]
        ROUTE -->|urgent| SLACK["Slack + Alert"]
        ROUTE -->|normal| LOG["Log to file"]
    end

    subgraph "Direction B — Agent calls Flow"
        WATCHER["Agent: Stream Assistant\n(always-on watcher)"]
        WATCHER -->|"detects: chat overwhelmed"| RF["run_flow()"]
        RF --> FLOW_SM["Flow: Enable Slow Mode"]
        FLOW_SM --> T1["Twitch API: slow_mode 30s"]
        FLOW_SM --> T2["OBS: Switch overlay"]
        FLOW_SM --> T3["Discord: log to mod channel"]
    end
```

---

## 6. MCP Layer — Two Directions

```mermaid
graph LR
    subgraph External["External MCP Servers"]
        OBS_MCP["OBS MCP\nport: 8001\nobs_set_scene\nobs_toggle_source"]
        TW_MCP["Twitch MCP\nport: 8002\ntwitch_read_chat\ntwitch_slow_mode"]
        CUSTOM["Any Custom\nMCP Server"]
    end

    subgraph VortiDeck["VortiDeck"]
        CONN_MGR["Connections Manager\nDiscovers tools\nAssigns to agents"]
        VD_MCP["VortiDeck as\nMCP Server\nport: 9000"]
    end

    subgraph Out["External AI Clients"]
        CLAUDE_D["Claude Desktop"]
        OTHER["Any MCP Client"]
    end

    OBS_MCP -->|"host: consumes tools"| CONN_MGR
    TW_MCP --> CONN_MGR
    CUSTOM --> CONN_MGR

    VD_MCP -->|"exposes: run_flow\npress_button\nstart_agent"| CLAUDE_D
    VD_MCP --> OTHER
```

---

## 7. Rust Module Structure

```mermaid
graph TD
    subgraph "services/ (Rust)"
        FLOW_SVC["flow/\n executor.rs\n registry.rs\n standard_nodes.rs\n ai_executor.rs"]
        MCP_SVC["mcp/\n tool.rs\n unified_tool_executor.rs\n session_manager.rs\n mcp_server.rs ← NEW"]
        AGENT_SVC["agents/ ← NEW MODULE\n agent_service.rs\n agent_definition.rs\n agent_runtime.rs\n agent_executor.rs\n agent_store.rs\n run_agent_node.rs"]
    end

    AGENT_SVC -->|"registers RunAgentNode"| FLOW_SVC
    AGENT_SVC -->|"calls tools via"| MCP_SVC
    MCP_SVC -->|"exposes VortiDeck"| EXT["External Clients"]
```

---

## 8. UI Structure

```mermaid
graph TD
    subgraph Sidebar
        HOME["🏠 Home"]
        DECK_NAV["🎛️ Deck"]
        FLOWS_NAV["⚡ Flows (existing)"]
        AGENTS_NAV["🤖 Agents ← NEW"]
        CONN_NAV["🔌 Connections ← NEW"]
        SETTINGS["⚙️ Settings"]
    end

    AGENTS_NAV --> AGENTS_PAGE["/agents page"]
    AGENTS_PAGE --> AGENT_LIST["Left: Agent List\n● Stream Assistant\n○ Code Reviewer\n+ New Agent"]
    AGENTS_PAGE --> AGENT_EDITOR["Right: Agent Editor"]
    AGENT_EDITOR --> TAB_PROMPT["Tab: Prompt\nsystem prompt + model"]
    AGENT_EDITOR --> TAB_TOOLS["Tab: Tools\nassign MCP + flow tools"]
    AGENT_EDITOR --> TAB_TRIGGER["Tab: Trigger\nalways-on / timer / event"]
    AGENT_EDITOR --> TAB_LOGS["Tab: Live Logs\nreal-time tool call stream"]

    CONN_NAV --> CONN_PAGE["/connections page"]
    CONN_PAGE --> MCP_LIST["Connected MCP Servers"]
    CONN_PAGE --> TOOL_LIST["Available Tools\n(discovered from servers)"]
    CONN_PAGE --> VD_SERVER["VortiDeck as MCP Server\nON/OFF + port"]
```

---

## 9. RunAgentNode in the Flow Builder

```mermaid
graph LR
    subgraph "Flow Canvas"
        TRIG["Webhook\nTrigger"] --> HTTP_N["HTTP\nRequest"]
        HTTP_N --> RAN_NODE["🤖 RunAgentNode\nAgent: Code Reviewer\nInput: {{code}}"]
        RAN_NODE -->|output: review| ROUTE_N["Route"]
        ROUTE_N -->|has bugs| SLACK_N["Slack Alert"]
        ROUTE_N -->|clean| LOG_N["Log: OK"]
    end
```

**RunAgentNode behavior:**
- One-shot call (no persistent loop, no memory)
- Spawns agent with its configured tools
- Waits for final response
- Returns text or structured JSON to next node
- Same agent definition reused — just different execution mode

---

## 10. Build Phases

```mermaid
gantt
    title Agent System Build Phases
    dateFormat  X
    axisFormat  Phase %s

    section Phase 1 - Core MVP
    Rust agents/ module          :p1a, 0, 1
    Tauri commands               :p1b, 0, 1
    /agents page + editor        :p1c, 0, 1
    Live log tab                 :p1d, 0, 1
    Sidebar item                 :p1e, 0, 1

    section Phase 2 - MCP UI
    /connections page            :p2a, 1, 2
    Tool discovery display       :p2b, 1, 2
    Assign tools to agents       :p2c, 1, 2

    section Phase 3 - Flow Integration
    RunAgentNode                 :p3a, 2, 3
    Expose flow as Agent Tool    :p3b, 2, 3

    section Phase 4 - VD as MCP Server
    mcp_server.rs                :p4a, 3, 4
    Expose flows/agents/deck     :p4b, 3, 4
    Settings UI                  :p4c, 3, 4
```

---

## 11. Stream Assistant — Full End-to-End Example

```mermaid
sequenceDiagram
    participant OBS as OBS Studio
    participant VD as VortiDeck
    participant AG as Agent: Stream Assistant
    participant LLM as Claude Sonnet
    participant TW as Twitch MCP
    participant FL as Flow: Slow Mode

    OBS->>VD: is_streaming = true (Start Condition met)
    VD->>AG: activate()

    loop every 5 seconds
        AG->>LLM: tick context + tools
        LLM->>TW: twitch_read_chat()
        TW-->>LLM: 47 msgs/30s, frustrated tone
        LLM->>FL: run_flow("Enable Slow Mode")
        FL->>TW: set slow_mode: 30s
        FL->>OBS: switch overlay
        FL-->>LLM: completed
        LLM-->>AG: "Slow mode enabled. Monitoring."
        AG->>AG: log + sleep 5s
    end

    OBS->>VD: is_streaming = false
    VD->>AG: deactivate()
```

---

## Summary Table

| Concept | What it is |
|---|---|
| **Agent** | Named AI process with a goal, runs in background autonomously |
| **Watcher** | Agent with timer/event trigger that polls and acts |
| **AgentTool** | Any flow tagged as callable by agents via `run_flow()` |
| **RunAgentNode** | Flow node that calls an agent inline for smart reasoning |
| **MCP Host** | VortiDeck consuming tools from OBS/Twitch/custom servers |
| **MCP Server** | VortiDeck exposing itself so Claude Desktop can control it |
| **Connections** | Page for managing all external MCP server connections |
