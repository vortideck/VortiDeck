# Example Plugin with SDK

This demonstrates the **cleanest possible** plugin development experience with `@vortideck/plugin-sdk`.

## Before (Without SDK) - 100+ lines

```typescript
// index.ts
import { Host } from '@extism/js-pdk'
import { executeImpl, get_metadataImpl, registerImpl } from './main'

export function execute(): number {
  const input = JSON.parse(Host.inputString())
  const output = executeImpl(input)
  Host.outputString(JSON.stringify(output))
  return 0
}

export function get_metadata(): number {
  const input = Host.inputString()
  const output = get_metadataImpl(input)
  Host.outputString(JSON.stringify(output))
  return 0
}

export function register(): number {
  const input = Host.inputString()
  const output = registerImpl(input)
  Host.outputString(output)
  return 0
}

// main.ts
import { vortideck_log, vortideck_register_node, vortideck_get_context } from './pdk.generated'

export function executeImpl(input: any): any {
  vortideck_log("Processing...")
  // ... 50+ lines of implementation
}

export function get_metadataImpl(input: string): any {
  return metadata
}

export function registerImpl(input: string): string {
  // ... registration logic
}
```

## After (With SDK) - **One authoring entry point**

```typescript
// main.ts - THIS IS ALL YOU WRITE!
import vortideck, { createPlugin, defineNode, FlowParams } from '@vortideck/plugin-sdk'

export const { execute, register, get_metadata } = createPlugin({
  nodes: {
    uppercase: defineNode({
      metadata: {
        id: "example:text-uppercase",
        name: "Text to Uppercase",
        description: "Convert text to uppercase",
        category: "text",
        type_color: "#4A90E2"
      },
      execute(input) {
        vortideck.log("Processing...")
        return { success: true, output: new FlowParams(input).getInput().toUpperCase() }
      }
    })
  }
})
```

## Key Improvements

✅ **No Host imports** - SDK handles all Extism boilerplate
✅ **No index.ts** - Just export from `main.ts`
✅ **Clean API** - `vortideck.log()` not `vortideck_log()`
✅ **Auto error handling** - SDK wraps everything in try/catch
✅ **Type safety** - Full TypeScript types auto-generated from Rust
✅ **FlowParams helper** - Easy parameter extraction

## Setup

```bash
npm install
npm run build
```

The build process will automatically:
- ✅ Compile TypeScript to WASM
- ✅ Generate `manifest.json` from `package.json` vortideck config
- ✅ Create complete plugin folder structure in `dist/`
- ✅ Copy icon and web extensions (if present)

Output: `dist/example-plugin-with-sdk-plugin/` ready to install in VortiDeck.

The SDK is installed from the local `packages/plugin-sdk` package.

## Plugin Configuration

Plugin metadata is configured in `package.json` under the `"vortideck"` field:

```json
{
  "vortideck": {
    "id": "example.text-processor",
    "name": "Text Processor Suite",
    "version": "2.0.1",
    "author": "VortiDeck Team",
    "description": "Text processing plugin with 7 nodes",
    "vortideck_version": ">=0.7.0",
    "schema_version": "1.0",
    "permissions": [],
    "web_extensions": false
  }
}
```

**Field Explanations:**
- `id` - Unique installable package identifier. Use 2-128 letters, numbers, dots, underscores, or hyphens, starting with a letter or number. A dotted namespace such as `example.text-processor` is recommended.
- `name` - Display name
- `version` - Plugin version
- `author` - Plugin author
- `description` - Plugin description
- `vortideck_version` - Minimum VortiDeck **runtime** version required (for this SDK, `">=0.7.0"`)
- `schema_version` - Manifest **format** version (defaults to "1.0")
- `permissions` - Requested permissions array
- `web_extensions` - Enable web UI extensions (false for pure flow nodes)

The build tool reads this config and auto-generates `manifest.json`.

**Note:** Always use package.json as the single source of truth. The build process generates manifest.json automatically.

Package IDs and executable node IDs have different jobs:

- Package IDs become cross-platform directory and storage namespace components, so they must be path-safe and cannot contain `:`.
- Node IDs are logical execution-engine identifiers and may use namespaces such as `example:text-uppercase`, consistently with IDs such as `macro:...`.
- The host can expose a node as `plugin.example:text-uppercase`; this does not change the owning package ID (`example.text-processor`).

## What the SDK Does For You

1. **Handles all Host boilerplate** - No more `Host.inputString()` / `Host.outputString()`
2. **Generates Extism exports** - `execute`, `register`, `get_metadata` automatically
3. **Error handling** - Automatic try/catch with proper error responses
4. **Clean API** - Converts `vortideck_log` → `vortideck.log`
5. **Type safety** - All types auto-generated from your Rust host functions

## Developer Experience

**Old way**: 100+ lines across 3 files, manual Host handling, verbose API

**New way**: one TypeScript authoring entry point with the Extism ABI boilerplate handled by the SDK. Its size scales with the number and complexity of the nodes you define.

## Host integration evidence

Build the current artifact before running the generated-artifact host test:

```bash
pnpm --filter example-plugin-with-sdk build
cargo test --manifest-path ../../apps/tauri-windows/libs/vortideck-core/Cargo.toml --lib plugins::manager::execution_budget_tests::built_sdk_example_emits_progress_and_custom_events_with_or_without_subscribers -- --ignored --nocapture
```

The test loads and invokes the generated WASM through the real host wrappers, verifies progress and custom events under the package principal, and invokes it again without an EventBus subscriber. No subscribers is a successful no-op, not a plugin failure.
