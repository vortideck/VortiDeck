# VortiDeck Plugin Architecture

> **Current contract clarification, 5 September 2026:** Use the [architecture guide](../architecture/ARCHITECTURE_GUIDE.md) for current ownership and trust boundaries. Service plugins run as ordinary native child processes; host RPC permission checks do not sandbox their direct OS access. The historical “all plugins” permission wording below must not be interpreted as OS-level confinement. Runtime support, timings and old file references need current evidence.

> **Historical design document.** This file contains planned APIs and runtimes
> that are not all present in 0.7.0. The authoritative TypeScript/WASM SDK
> contract is [`packages/plugin-sdk/README.md`](../../packages/plugin-sdk/README.md)
> and its public [`src/index.ts`](../../packages/plugin-sdk/src/index.ts).

> **Version:** 2.0
> **Date:** December 2025
> **Status:** ✅ Implemented (Core functionality complete)

## Table of Contents

1. [Overview](#overview)
2. [Plugin Types](#plugin-types)
3. [Architecture Principles](#architecture-principles)
4. [Communication Protocols](#communication-protocols)
5. [Development Workflow](#development-workflow)
6. [Distribution & Installation](#distribution--installation)
7. [Manifest Format](#manifest-format)
8. [SDK Overview](#sdk-overview)
9. [Examples](#examples)

---

## Overview

VortiDeck's plugin system is designed to be **modular, secure, and developer-friendly**. It supports two plugin runtimes:

- **WASM Plugins**: Sandboxed, in-process execution for simple transformations
- **Service Plugins**: External processes for complex services with full ecosystem access

### Key Principles

1. **Zero Domain Logic in Core**: VortiDeck Core is a generic infrastructure. All domain-specific functionality (telephony, AI, databases) comes from plugins.

2. **One SDK, Multiple Runtimes**: Developers write code once, deploy as WASM or Service plugin.

3. **Process Isolation**: Service plugins run as separate processes, ensuring host stability.

4. **Permission-Based Security**: All plugins operate under a granular permission system.

---

## Plugin Types

### 1. WASM Plugins

**Best for:** Simple, stateless operations

```
Characteristics:
- Runs in-process (VortiDeck's memory space)
- Sandboxed via WebAssembly
- Fast startup (10-100ms)
- Low memory (2-10MB)
- No native dependencies
- Cross-platform (single .wasm file)
```

**Use Cases:**
- Text transformations
- Data processing
- JSON/XML parsing
- HTTP API wrappers
- Simple calculations

**Example:**
```typescript
// Text uppercase plugin
export const nodes = {
  uppercase: {
    name: "Text to Uppercase",
    async execute(inputs) {
      return { result: inputs.text.toUpperCase() };
    }
  }
};
```

---

### 2. Service Plugins

**Best for:** Complex services, persistent connections, native libraries

```
Characteristics:
- Runs as separate process
- Full OS access (with permissions)
- Higher memory (50-150MB for Node.js)
- Can use any npm/pip/cargo package
- Supports persistent connections
- Language-agnostic (Node.js, Python, Rust, Go, etc.)
```

**Use Cases:**
- Telephony systems (ARI, SIP)
- Database connection pools
- AI/ML services (Python)
- WebSocket servers
- Long-running services

**Example:**
```typescript
// ARI telephony plugin
export const nodes = {
  connectToAri: {
    name: "Connect to ARI",
    async execute(inputs) {
      const conn = await VortiDeck.ws.connect(
        `ws://${inputs.host}:8088/ari/events`
      );
      return { connectionId: conn };
    }
  }
};
```

---

## Architecture Principles

### Design Philosophy

#### 1. **Generic Host, Specific Plugins**

```
┌────────────────────────────────────────────┐
│ VortiDeck Core (Generic Infrastructure)    │
│  ├─ Flow Engine (domain-agnostic)          │
│  ├─ Plugin Runtime (WASM + Service)        │
│  └─ Host APIs (HTTP, WS, TCP, File I/O)    │
└────────────────────────────────────────────┘
             │
             │ Loads & Manages
             ▼
┌────────────────────────────────────────────┐
│ Plugins (Domain-Specific)                  │
│  ├─ ARI Telephony (Service)                │
│  ├─ Google Transcription (Service/Python)  │
│  ├─ Text Utilities (WASM)                  │
│  └─ Database Connectors (Service)          │
└────────────────────────────────────────────┘
```

**Result:** VortiDeck Core has ZERO telephony, AI, or database code. Everything is a plugin.

---

#### 2. **IPC via stdin/stdout (Pipes)**

**Why stdin/stdout over sockets?**

Based on [Plugin Tech Research](https://nullderef.com/blog/plugin-tech/):

| Method | Performance | Availability | Safety | Our Rating |
|--------|-------------|--------------|--------|------------|
| **stdin/stdout** | 4/5 (1-5ms) | 5/5 (stdlib) | 5/5 (no unsafe) | ⭐⭐⭐⭐⭐ |
| Sockets | 3/5 (5-10ms) | 5/5 | 3/5 | ⭐⭐⭐ |
| Shared Memory | 5/5 (<1ms) | 2/5 | 2/5 (lots of unsafe) | ⭐⭐ |

**stdin/stdout wins because:**
- ✅ Built into every language (Node.js, Python, Rust, Go)
- ✅ Zero configuration (no ports to manage)
- ✅ Easy debugging (just read/write JSON lines)
- ✅ Same pattern as LSP (Language Server Protocol)
- ✅ Performance is sufficient for flow nodes (1-5ms latency)

---

## Communication Protocols

### WASM Plugin Communication

**Pattern:** Host Functions

```rust
// VortiDeck Core (Rust) - Host Function
#[vortideck_host]
fn vortideck_http_request(config_json: String) -> Result<String, String> {
    // Execute HTTP request
    // Return response
}
```

```typescript
// Plugin (TypeScript/WASM) - Calls Host Function
const response = await VortiDeck.http.request({
  url: "https://api.example.com",
  method: "GET"
});
```

**Flow:**
```
WASM Plugin
    │
    │ Call host function (direct)
    ▼
VortiDeck Core (Rust)
    │
    │ Execute operation
    ▼
Return result to WASM
```

---

### Service Plugin Communication

**Pattern:** JSON-RPC over stdin/stdout

```typescript
// Plugin (Node.js) - Calls VortiDeck API
const response = await VortiDeck.http.request({
  url: "https://api.example.com",
  method: "GET"
});
```

**Behind the scenes:**

```json
// Plugin → VortiDeck (stdout)
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "vortideck.http.request",
  "params": {
    "url": "https://api.example.com",
    "method": "GET"
  }
}

// VortiDeck → Plugin (stdin)
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "status": 200,
    "body": "{...}"
  }
}
```

**Flow:**
```
Service Plugin Process
    │
    │ Write JSON-RPC to stdout
    ▼
VortiDeck Core (Rust)
    │
    │ Parse JSON-RPC
    │ Execute operation
    │ Write response to plugin's stdin
    ▼
Service Plugin Process
    │
    │ Parse response
    │ Return to plugin code
```

---

### Flow Node Execution

**VortiDeck calls plugin nodes via same protocol:**

```json
// VortiDeck → Plugin (stdin)
{
  "jsonrpc": "2.0",
  "id": 42,
  "method": "executeNode",
  "params": {
    "nodeId": "uppercase",
    "inputs": {
      "text": "hello world"
    }
  }
}

// Plugin → VortiDeck (stdout)
{
  "jsonrpc": "2.0",
  "id": 42,
  "result": {
    "result": "HELLO WORLD"
  }
}
```

---

## Development Workflow

### Quick Start

#### Option 1: TypeScript (WASM or Service)

```bash
# Create plugin
npx create-vortideck-plugin my-plugin --runtime service-node

# Install dependencies (any npm package!)
cd my-plugin
npm install axios socket.io

# Develop
code .

# Dev mode (hot reload)
npm run dev
```

**Plugin code:**

```typescript
// src/index.ts
import { VortiDeck } from '@vortideck/plugin-sdk';

export const nodes = {
  fetchData: {
    name: "Fetch Data",
    category: "API",
    inputs: {
      url: { type: "string" }
    },
    outputs: {
      data: { type: "object" }
    },

    async execute(inputs) {
      // Use VortiDeck API (works in WASM OR Service!)
      const response = await VortiDeck.http.request({
        url: inputs.url,
        method: "GET"
      });

      return { data: JSON.parse(response.body) };
    }
  }
};
```

**Dev workflow:**
1. Edit `src/index.ts`
2. Save (hot reload)
3. Node appears instantly in VortiDeck flow editor
4. Drag & drop to test

---

#### Option 2: Python (Service Plugin)

```bash
# Create plugin
pip install vortideck-plugin-sdk

# Create plugin.py
```

```python
# plugin.py
from vortideck import FlowNode, VortiDeck

class TranscribeAudioNode(FlowNode):
    metadata = {
        "id": "google_transcribe",
        "name": "Google Transcribe Audio",
        "category": "AI"
    }

    async def execute(self, inputs):
        # Use any Python library!
        from google.cloud import speech_v1

        client = speech_v1.SpeechClient()
        response = client.recognize(
            config={"encoding": "MULAW"},
            audio={"content": inputs['audio']}
        )

        return {
            "transcription": response.results[0].alternatives[0].transcript
        }
```

```bash
# Run
python plugin.py

# VortiDeck auto-discovers nodes
```

---

#### Option 3: Rust (WASM or Service Binary)

**As Service Binary:**

```rust
// src/main.rs
use vortideck_plugin_sdk::{VortiDeck, FlowNode};

struct MyNode;

impl FlowNode for MyNode {
    async fn execute(&self, inputs: serde_json::Value) -> Result<serde_json::Value> {
        // Use Rust crates
        let response = VortiDeck::http_request(HttpOptions {
            url: "https://api.example.com",
            method: "GET"
        }).await?;

        Ok(json!({ "result": response }))
    }
}

#[tokio::main]
async fn main() {
    vortideck_plugin_main!(vec![Box::new(MyNode)]).await;
}
```

```bash
# Build
cargo build --release

# Creates: target/release/my-plugin (single binary!)
```

---

### Hot Reload (Development)

**For folder-based plugins:**

```bash
cd plugins/my-plugin/
npm run dev

# VortiDeck watches for changes:
# 1. Edit src/index.ts
# 2. Save
# 3. VortiDeck detects change
# 4. Reloads plugin
# 5. Nodes update in UI
```

**No repackaging needed during development!**

---

## Distribution & Installation

### Plugin Lifecycle

```
Development → Build → Package → Publish → Download → Install → Load
```

---

### 1. Development (Folder-Based)

```
plugins/my-plugin/
├── manifest.json
├── package.json
├── src/
│   └── index.ts
└── node_modules/    ← 150 MB (lives on disk, not in memory!)
    └── ...
```

**VortiDeck loads directly from folder during development.**

---

### 2. Build

```bash
# Build for production
npm run build

# Creates: dist/index.js (bundled)
```

---

### 3. Package (.vdext)

```bash
# Package for distribution
vortideck package

# Creates: my-plugin.vdext (ZIP file)
```

**Contents of .vdext:**
```
my-plugin.vdext (ZIP)
├── manifest.json
├── dist/
│   └── index.js
└── node_modules/     ← Included for service plugins!
    └── ...
```

---

### 4. Publish (Marketplace)

```bash
# Upload to VortiDeck marketplace
vortideck publish my-plugin.vdext
```

---

### 5. Download & Install

```bash
# User installs
vortideck install my-plugin
```

**What happens:**
1. Download `my-plugin.vdext` (150 MB)
2. Extract to `plugins/my-plugin/` folder
3. Delete `.vdext` file (save disk space)
4. Load from folder

---

### 6. Runtime (Always Folder-Based)

```
Documents/VortiDeck/plugins/
├── text-processor/       ← WASM plugin (500 KB)
│   ├── manifest.json
│   └── plugin.wasm
│
└── ari-telephony/        ← Service plugin (150 MB)
    ├── manifest.json
    ├── dist/index.js
    └── node_modules/     ← Lives on disk, streamed as needed
        └── ...
```

**VortiDeck loads from folders:**
- WASM: Load `plugin.wasm` into memory (~500 KB)
- Service: Spawn `node dist/index.js` (process reads from disk)

**Result:** No huge ZIP files in memory!

---

## Manifest Format

### Basic Structure

```json
{
  "schema_version": "1.0",
  "id": "my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "author": "Your Name",
  "description": "Plugin description",
  "vortideck_version": ">=0.1.0",

  "runtime": "wasm" | "service",

  "entry": {
    "wasm": "plugin.wasm",
    // OR
    "service": {
      "command": "node",
      "args": ["dist/index.js"],
      "cwd": ".",
      "env": {
        "NODE_ENV": "production"
      }
    }
  },

  "permissions": [
    "network",
    "filesystem"
  ]
}
```

---

### WASM Plugin Manifest

```json
{
  "id": "text-processor",
  "name": "Text Processor",
  "version": "1.0.0",
  "runtime": "wasm",

  "entry": {
    "wasm": "plugin.wasm"
  },

  "permissions": []
}
```

---

### Service Plugin Manifest (Node.js)

```json
{
  "id": "ari-telephony",
  "name": "ARI Telephony System",
  "version": "1.0.0",
  "runtime": "service",

  "entry": {
    "service": {
      "command": "node",
      "args": ["dist/index.js"],
      "env": {
        "NODE_ENV": "production"
      }
    }
  },

  "permissions": [
    "network",
    "filesystem"
  ]
}
```

---

### Service Plugin Manifest (Python)

```json
{
  "id": "ai-transcription",
  "name": "AI Transcription",
  "version": "1.0.0",
  "runtime": "service",

  "entry": {
    "service": {
      "command": "python3",
      "args": ["plugin.py"]
    }
  },

  "permissions": ["network"]
}
```

---

### Service Plugin Manifest (Rust Binary)

```json
{
  "id": "high-performance-processor",
  "name": "High Performance Processor",
  "version": "1.0.0",
  "runtime": "service",

  "entry": {
    "service": {
      "command": "./plugin-binary"
    }
  },

  "permissions": ["filesystem"]
}
```

---

### Multi-Runtime Manifest (Advanced)

```json
{
  "id": "my-plugin",
  "runtime": "auto",

  "entry": {
    "wasm": "plugin.wasm",
    "service": {
      "command": "node",
      "args": ["dist/index.js"]
    }
  }
}
```

**VortiDeck tries WASM first (faster), falls back to Service if needed.**

---

## SDK Overview

### Unified API

**Same code works in WASM and Service plugins!**

```typescript
import { VortiDeck } from '@vortideck/plugin-sdk';

// HTTP request
const response = await VortiDeck.http.request({
  url: "https://api.example.com",
  method: "GET",
  headers: { "Authorization": "Bearer token" }
});

// WebSocket client
const wsConn = await VortiDeck.ws.connect("ws://localhost:8088");
VortiDeck.ws.send(wsConn, "Hello");

// File I/O
const content = await VortiDeck.fs.readFile("workspace/data.json");
await VortiDeck.fs.writeFile("workspace/output.txt", "result");

// Emit events (for flow triggers)
await VortiDeck.emitEvent({
  type: "data_processed",
  data: { count: 100 }
});

// Storage (plugin-scoped)
await VortiDeck.storage.set("key", "value");
const value = await VortiDeck.storage.get("key");
```

---

### Auto-Runtime Detection

```typescript
// SDK automatically detects runtime
const runtime = detectRuntime();

if (runtime === 'wasm') {
  // Calls WASM host function
  vortideck_http_request(options);
} else {
  // Sends JSON-RPC via stdout
  process.stdout.write(JSON.stringify({
    method: "vortideck.http.request",
    params: options
  }) + '\n');
}
```

**Developer never needs to know which runtime they're in!**

---

### Flow Node Definition

```typescript
export const nodes = {
  myNode: {
    name: "My Node",
    category: "MY_CATEGORY",
    icon: "🔥",  // Optional
    color: "#FF6B6B",  // Optional

    inputs: {
      text: {
        type: "string",
        default: "hello"
      },
      count: {
        type: "number",
        default: 1
      }
    },

    outputs: {
      result: { type: "string" },
      processed: { type: "boolean" }
    },

    async execute(inputs) {
      // Your logic here
      const result = inputs.text.repeat(inputs.count);

      return {
        result,
        processed: true
      };
    }
  }
};
```

**Nodes automatically:**
- ✅ Register with VortiDeck
- ✅ Appear in flow editor
- ✅ Support drag & drop
- ✅ Validate inputs/outputs

---

## Examples

### Example 1: Simple WASM Plugin (Text Utilities)

**manifest.json:**
```json
{
  "id": "text-utilities",
  "name": "Text Utilities",
  "version": "1.0.0",
  "runtime": "wasm",
  "entry": { "wasm": "plugin.wasm" }
}
```

**src/index.ts:**
```typescript
import { VortiDeck } from '@vortideck/plugin-sdk';

export const nodes = {
  uppercase: {
    name: "Text to Uppercase",
    inputs: { text: { type: "string" } },
    outputs: { result: { type: "string" } },
    async execute(inputs) {
      return { result: inputs.text.toUpperCase() };
    }
  },

  reverse: {
    name: "Reverse Text",
    inputs: { text: { type: "string" } },
    outputs: { result: { type: "string" } },
    async execute(inputs) {
      return { result: inputs.text.split('').reverse().join('') };
    }
  }
};
```

**Build:**
```bash
npm run build:wasm
# Creates: plugin.wasm (500 KB)
```

---

### Example 2: Service Plugin (ARI Telephony)

**manifest.json:**
```json
{
  "id": "ari-telephony",
  "name": "ARI Telephony System",
  "version": "1.0.0",
  "runtime": "service",
  "entry": {
    "service": {
      "command": "node",
      "args": ["dist/index.js"]
    }
  },
  "permissions": ["network"]
}
```

**src/index.ts:**
```typescript
import { VortiDeck } from '@vortideck/plugin-sdk';

export const nodes = {
  connectToAri: {
    name: "Connect to ARI",
    category: "TELEPHONY",
    inputs: {
      host: { type: "string", default: "localhost" },
      port: { type: "number", default: 8088 },
      username: { type: "string" },
      password: { type: "string", secret: true }
    },
    outputs: {
      connectionId: { type: "string" }
    },

    async execute(inputs) {
      // Use VortiDeck's WebSocket client
      const wsConn = await VortiDeck.ws.connect(
        `ws://${inputs.host}:${inputs.port}/ari/events?app=vortideck`
      );

      // Listen for ARI events
      VortiDeck.ws.onMessage(wsConn, async (message) => {
        const ariEvent = JSON.parse(message);

        // Emit as VortiDeck event (for flow triggers)
        await VortiDeck.emitEvent({
          type: `ari.${ariEvent.type}`,
          data: ariEvent
        });
      });

      return { connectionId: wsConn };
    }
  },

  answerChannel: {
    name: "Answer Channel",
    category: "TELEPHONY",
    inputs: {
      connectionId: { type: "string" },
      channelId: { type: "string" }
    },
    outputs: {
      success: { type: "boolean" }
    },

    async execute(inputs) {
      // Simple HTTP request to ARI
      await VortiDeck.http.request({
        url: `http://localhost:8088/ari/channels/${inputs.channelId}/answer`,
        method: "POST"
      });

      return { success: true };
    }
  }
};
```

**package.json:**
```json
{
  "dependencies": {
    "@vortideck/plugin-sdk": "^1.0.0",
    "axios": "^1.6.0",
    "socket.io": "^4.6.0"
  }
}
```

**Build:**
```bash
npm install
npm run build
# Creates: dist/index.js + node_modules/
```

---

### Example 3: Python AI Plugin

**manifest.json:**
```json
{
  "id": "google-transcription",
  "name": "Google Transcription",
  "version": "1.0.0",
  "runtime": "service",
  "entry": {
    "service": {
      "command": "python3",
      "args": ["plugin.py"]
    }
  },
  "permissions": ["network"]
}
```

**plugin.py:**
```python
from vortideck import FlowNode, VortiDeck
from google.cloud import speech_v1

class TranscribeNode(FlowNode):
    metadata = {
        "id": "transcribe_audio",
        "name": "Transcribe Audio",
        "category": "AI",
        "inputs": {
            "audio": {"type": "buffer"},
            "language": {"type": "string", "default": "en-US"}
        },
        "outputs": {
            "transcription": {"type": "string"},
            "confidence": {"type": "number"}
        }
    }

    def __init__(self):
        self.client = speech_v1.SpeechClient()

    async def execute(self, inputs):
        audio = speech_v1.RecognitionAudio(content=inputs['audio'])
        config = speech_v1.RecognitionConfig(
            encoding=speech_v1.RecognitionConfig.AudioEncoding.MULAW,
            sample_rate_hertz=8000,
            language_code=inputs['language']
        )

        response = self.client.recognize(config=config, audio=audio)

        if response.results:
            result = response.results[0].alternatives[0]
            return {
                "transcription": result.transcript,
                "confidence": result.confidence
            }

        return {"transcription": "", "confidence": 0}

# Start plugin
if __name__ == "__main__":
    from vortideck import run_plugin
    run_plugin([TranscribeNode()])
```

---

## Architecture Diagrams

### System Overview

```
┌──────────────────────────────────────────────────────┐
│ VortiDeck Core (Rust/Tauri)                          │
│                                                       │
│  ┌────────────────────────────────────────────────┐  │
│  │ Flow Engine                                    │  │
│  │  ├─ Execute flows                              │  │
│  │  ├─ Manage connections                         │  │
│  │  └─ Handle triggers                            │  │
│  └────────────────────────────────────────────────┘  │
│                                                       │
│  ┌────────────────────────────────────────────────┐  │
│  │ Plugin Manager                                 │  │
│  │  ├─ Load plugins                               │  │
│  │  ├─ Manage lifecycle                           │  │
│  │  └─ Route node execution                       │  │
│  └────────────────────────────────────────────────┘  │
│                                                       │
│  ┌────────────────────────────────────────────────┐  │
│  │ Plugin Runtimes                                │  │
│  │  ├─ WASM Runtime (Extism/Wasmtime)            │  │
│  │  └─ Service Runtime (Process spawner)         │  │
│  └────────────────────────────────────────────────┘  │
│                                                       │
│  ┌────────────────────────────────────────────────┐  │
│  │ Host APIs (Protocol-Level)                     │  │
│  │  ├─ HTTP client/server                         │  │
│  │  ├─ WebSocket client/server                    │  │
│  │  ├─ TCP/UDP sockets                            │  │
│  │  ├─ File I/O                                   │  │
│  │  └─ Event bus                                  │  │
│  └────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
```

---

### WASM Plugin Flow

```
┌──────────────────┐
│ Flow Editor      │
│ User runs flow   │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Flow Engine      │
│ Execute node     │
└────────┬─────────┘
         │
         ▼
┌──────────────────────────────┐
│ WASM Plugin (in-process)     │
│  plugin.execute(inputs)      │
│         ↓                    │
│  VortiDeck.http.request()    │
│         ↓                    │
│  vortideck_http_request()    │← Host function call
└────────┬─────────────────────┘
         │
         ▼
┌──────────────────┐
│ VortiDeck Core   │
│ Execute HTTP req │
│ Return response  │
└────────┬─────────┘
         │
         ▼
Back to WASM plugin
         │
         ▼
Return result to Flow Engine
```

---

### Service Plugin Flow

```
┌──────────────────┐
│ Flow Editor      │
│ User runs flow   │
└────────┬─────────┘
         │
         ▼
┌──────────────────────────────┐
│ Flow Engine                  │
│ Call executeNode via JSON-RPC│
└────────┬─────────────────────┘
         │ stdin
         ▼
┌───────────────────────────────┐
│ Service Plugin Process        │
│ (Node.js/Python/Rust)         │
│                               │
│  plugin.execute(inputs)       │
│         ↓                     │
│  VortiDeck.http.request()     │
│         ↓                     │
│  Write JSON-RPC to stdout     │
└────────┬──────────────────────┘
         │ stdout
         ▼
┌──────────────────────────────┐
│ VortiDeck Core               │
│ Parse JSON-RPC               │
│ Execute HTTP request         │
│ Return via stdin             │
└────────┬─────────────────────┘
         │ stdin
         ▼
┌───────────────────────────────┐
│ Service Plugin Process        │
│ Parse response                │
│ Return to plugin code         │
└────────┬──────────────────────┘
         │ stdout (JSON-RPC)
         ▼
┌──────────────────────────────┐
│ Flow Engine                  │
│ Receive result               │
└──────────────────────────────┘
```

---

## Performance Characteristics

### WASM Plugins

| Metric | Value |
|--------|-------|
| Load time | 10-100ms |
| Memory usage | 2-10MB |
| Execution overhead | <1ms |
| HTTP request latency | 10-50ms |
| File I/O latency | <1ms |

---

### Service Plugins

| Metric | Node.js | Python | Rust Binary |
|--------|---------|--------|-------------|
| Load time | 200-500ms | 300-600ms | 50-200ms |
| Memory usage | 50-150MB | 50-100MB | 5-50MB |
| Execution overhead | 1-5ms | 1-5ms | <1ms |
| HTTP request latency | 10-50ms | 10-50ms | 10-50ms |

**Note:** For flow nodes (not real-time audio), 1-5ms overhead is imperceptible.

---

## Security Model

### Permission System

All plugins (WASM and Service) operate under permissions:

```json
{
  "permissions": [
    "network",          // HTTP/WS/TCP/UDP
    "filesystem",       // Read/write files
    "dom",              // Modify UI (web extensions)
    "css",              // Inject styles
    "execute_commands", // Run system commands (Service only)
    "flows",            // Run other flows
    "storage"           // Plugin-scoped storage
  ]
}
```

**Enforcement:**
- WASM: Host functions check permissions before execution
- Service: VortiDeck rejects JSON-RPC calls without permission

---

### Sandboxing

| Plugin Type | Sandboxing Level | Isolation |
|-------------|------------------|-----------|
| **WASM** | ✅✅✅ Strong | Memory isolation via WASM |
| **Service** | ⚠️ Process isolation | Separate process, can be killed |

**Additional Security (Future):**
- Code signing for marketplace plugins
- Containerization (Docker) for service plugins
- Resource limits (CPU, memory, disk)

---

## Future Enhancements

### Planned Features

1. **WASI Sockets** (when stable)
   - Native WebSocket/TCP support in WASM
   - Better performance for WASM plugins

2. **Plugin Marketplace**
   - Browse and install community plugins
   - Ratings and reviews
   - Automatic updates

3. **Hot Module Replacement**
   - Update plugins without restarting VortiDeck
   - Preserve plugin state during reload

4. **Resource Limits**
   - CPU quotas per plugin
   - Memory limits
   - Disk I/O throttling

5. **Observability**
   - Plugin metrics (CPU, memory, calls)
   - Distributed tracing
   - Performance profiling

---

## VortiDeck Host API Reference

**Status**: ✅ Implemented

Service plugins can call VortiDeck's Host APIs via JSON-RPC. These APIs provide generic infrastructure capabilities without domain-specific logic.

### HTTP API

**Method**: `vortideck.http.request`

Make HTTP requests with full control over method, headers, and body.

```javascript
const response = await vortideck.http.request({
  url: "https://api.github.com/repos/user/repo",
  method: "GET", // GET, POST, PUT, DELETE, PATCH
  headers: {
    "Authorization": "token ghp_...",
    "User-Agent": "VortiDeck-Plugin/1.0"
  },
  body: null // String or JSON object
});

// Response: { status: 200, headers: {...}, body: "..." }
```

**Parameters:**
- `url` (string, required): URL to request
- `method` (string, optional): HTTP method (default: "GET")
- `headers` (object, optional): Request headers
- `body` (string|object, optional): Request body

**Returns:**
```typescript
{
  status: number;
  headers: { [key: string]: string };
  body: string;
}
```

---

### File I/O API

#### Read File

**Method**: `vortideck.file.read`

```javascript
const content = await vortideck.file.read({
  path: "/path/to/file.txt"
});

// Returns: string (file contents)
```

**Parameters:**
- `path` (string, required): Absolute path to file

**Returns:** `string` - File contents as UTF-8 text

---

#### Write File

**Method**: `vortideck.file.write`

```javascript
await vortideck.file.write({
  path: "/path/to/output.txt",
  content: "Hello, World!"
});

// Returns: true on success
```

**Parameters:**
- `path` (string, required): Absolute path to file
- `content` (string, required): Content to write

**Returns:** `boolean` - true on success

---

### Logging API

**Method**: `vortideck.log`

Send log messages to VortiDeck's logging system.

```javascript
await vortideck.log({
  level: "info", // error, warn, info, debug
  message: "Processing request..."
});

// Returns: true
```

**Parameters:**
- `level` (string, optional): Log level (default: "info")
  - `"error"` - Critical errors
  - `"warn"` - Warnings
  - `"info"` - Informational messages
  - `"debug"` - Debug information
- `message` (string, required): Log message

**Returns:** `boolean` - true on success

---

### Future Host APIs (Planned)

These APIs are planned but not yet implemented:

#### WebSocket API
```javascript
// Connect to WebSocket
const ws = await vortideck.ws.connect({ url: "ws://..." });

// Send message
await vortideck.ws.send({ id: ws.id, data: "..." });

// Listen for messages (via events)
```

#### TCP/UDP API
```javascript
// TCP
const tcp = await vortideck.tcp.connect({ host: "...", port: 8080 });
await vortideck.tcp.send({ id: tcp.id, data: "..." });

// UDP
await vortideck.udp.send({ host: "...", port: 8080, data: "..." });
```

#### Database API
```javascript
// Generic SQL interface
const result = await vortideck.db.query({
  connection: "postgres://...",
  query: "SELECT * FROM users WHERE id = $1",
  params: [userId]
});
```

---

### Error Handling

All Host API calls return JSON-RPC errors if they fail:

```javascript
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32000,
    "message": "Failed to read file: No such file or directory"
  }
}
```

**Common Error Codes:**
- `-32000`: Server error (operation failed)
- `-32601`: Method not found (unknown API)
- `-32602`: Invalid params (missing required parameter)

---

## FAQ

### Q: Why not just use WASM for everything?

**A:** WASM can't access native libraries (ari-client, Google Cloud SDK) or run persistent servers. Service plugins provide full ecosystem access.

---

### Q: Why not use REST APIs instead of plugins?

**A:** Plugins integrate directly into flows, have lower latency, and can maintain state between calls.

---

### Q: Can I convert a WASM plugin to Service later?

**A:** Yes! Same code, just change the manifest and rebuild. The SDK abstracts the runtime.

---

### Q: How do I debug service plugins?

**A:** Just run them standalone:
```bash
node dist/index.js
# Set breakpoints in VSCode
# Plugin communicates via stdin/stdout (easy to test)
```

---

### Q: What's the performance overhead of JSON-RPC?

**A:** ~1-5ms per call, which is acceptable for flow nodes (not real-time audio processing).

---

### Q: Can plugins share code?

**A:** Yes, via npm packages (for TypeScript) or shared libraries (for Rust).

---

## Conclusion

VortiDeck's plugin architecture provides:

✅ **Flexibility**: WASM for simple plugins, Service for complex ones
✅ **Developer Experience**: One SDK, any language
✅ **Performance**: Fast enough for flow automation
✅ **Security**: Sandboxing + permissions
✅ **Ecosystem**: Access to npm/pip/cargo packages
✅ **Modularity**: Zero domain logic in core

**Result:** A truly extensible platform where VortiDeck is just infrastructure, and plugins provide all functionality.

---

## References

- [Plugin Tech Research](https://nullderef.com/blog/plugin-tech/)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
- [WebAssembly Security](https://webassembly.org/docs/security/)
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)

---

**Document Version:** 2.0
**Last Updated:** December 2025
**Status:** ✅ Core Implementation Complete
- Dual-runtime system: ✅ Implemented
- Host API handlers: ✅ HTTP, File I/O, Logging
- Example plugin: ✅ Node.js service plugin
- Documentation: ✅ Complete with API reference
