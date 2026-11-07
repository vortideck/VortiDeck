# Example Service Plugin

This is an example service plugin for VortiDeck that demonstrates:

- ✅ JSON-RPC 2.0 communication over stdin/stdout
- ✅ Node registration
- ✅ Calling VortiDeck Host APIs (HTTP, File I/O, Logging)
- ✅ Handling node execution requests
- ✅ Pure Node.js (no compilation required)

## Architecture

```
┌──────────────┐                    ┌──────────────┐
│   VortiDeck  │  ←─── stdin ────   │  Node.js     │
│     Host     │                    │   Plugin     │
│              │  ───→ stdout ────→ │              │
└──────────────┘                    └──────────────┘
     JSON-RPC 2.0 Protocol
```

## How It Works

### 1. Plugin Startup
- VortiDeck spawns `node index.js`
- Plugin reads from stdin, writes to stdout
- Plugin registers nodes via `vortideck.registerNodes` notification

### 2. Node Execution
- User triggers a node in the flow
- VortiDeck sends `execute` request to plugin via stdin
- Plugin processes the request
- Plugin calls VortiDeck APIs (HTTP, File, etc.)
- Plugin sends result back via stdout

### 3. Communication Protocol

**Request from VortiDeck to Plugin:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "execute",
  "params": {
    "nodeId": "example-service-plugin:fetch-data",
    "inputs": { "url": "https://api.github.com/zen" }
  }
}
```

**Response from Plugin to VortiDeck:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "success": true,
    "data": "Keep it logically awesome.",
    "status": 200
  }
}
```

**Plugin calling VortiDeck API:**
```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "method": "vortideck.http.request",
  "params": {
    "url": "https://api.github.com/zen",
    "method": "GET",
    "headers": { "User-Agent": "VortiDeck-Plugin/1.0" }
  }
}
```

## Registered Nodes

### 1. Fetch Data
- **ID**: `example-service-plugin:fetch-data`
- **Description**: Fetch data from a URL
- **Inputs**: `{ url: string }`
- **Outputs**: `{ success: boolean, data: string, status: number }`

### 2. Save File
- **ID**: `example-service-plugin:save-file`
- **Description**: Save content to a file
- **Inputs**: `{ path: string, content: string }`
- **Outputs**: `{ success: boolean, path: string }`

## VortiDeck Host APIs Used

This plugin demonstrates calling:

1. **vortideck.http.request** - Make HTTP requests
   ```javascript
   await callHost('vortideck.http.request', {
     url: 'https://api.github.com/zen',
     method: 'GET',
     headers: { 'User-Agent': 'VortiDeck-Plugin/1.0' }
   });
   ```

2. **vortideck.file.write** - Write files
   ```javascript
   await callHost('vortideck.file.write', {
     path: '/tmp/output.txt',
     content: 'Hello World'
   });
   ```

3. **vortideck.log** - Log messages
   ```javascript
   await callHost('vortideck.log', {
     level: 'info',
     message: 'Processing request...'
   });
   ```

## Installation

1. Copy this folder to VortiDeck's plugins directory
2. VortiDeck will auto-detect the service plugin
3. Nodes will appear in the flow editor

## Testing Standalone

You can test the plugin standalone by sending JSON-RPC messages:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"execute","params":{"nodeId":"example-service-plugin:fetch-data","inputs":{"url":"https://api.github.com/zen"}}}' | node index.js
```

## Development

The plugin is pure JavaScript with no dependencies. You can:

- Add more nodes by extending `executeNode()`
- Call additional VortiDeck APIs
- Use any npm packages you want
- Debug with `console.error()` (goes to stderr)

### Important: stdin/stdout Setup

**CRITICAL**: When using `readline` for stdin, do NOT include `output: process.stdout`:

```javascript
// ✅ CORRECT - Only specify input
const rl = readline.createInterface({
  input: process.stdin,
  terminal: false
});

// ❌ WRONG - Don't include output
const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout,  // This breaks JSON-RPC responses!
  terminal: false
});
```

**Why?** Because the plugin uses `process.stdout.write()` directly to send JSON-RPC messages. Having readline manage stdout causes conflicts that prevent responses from reaching VortiDeck.

## Next Steps

This is a minimal example. A production plugin would:

- Add input validation
- Implement proper error handling
- Add TypeScript types
- Use the VortiDeck Plugin SDK (when available)
- Add comprehensive logging
- Include unit tests
