# @vortideck/plugin-sdk

**Zero-configuration VortiDeck Plugin SDK with bundled tooling**

Build VortiDeck plugins with the best developer experience - no manual tool installation required!

## Features

✅ **Zero Configuration** - Just write your plugin and run `npm run build`
✅ **Cross-Platform** - Bundled builds for Windows x64, macOS x64/ARM64, Linux x64, and WSL x64
✅ **Bundled Tooling** - Includes extism-js and binaryen for all platforms
✅ **Type-Safe** - Auto-generated TypeScript definitions from Rust host functions
✅ **Clean API** - Use `createPlugin()` helper, no boilerplate needed

## Installation

```bash
npm install @vortideck/plugin-sdk
```

## Quick Start

### Create a New Plugin

```bash
# Option 1: Create in a new directory
mkdir my-plugin && cd my-plugin
npx @vortideck/plugin-sdk init my-plugin

# Option 2: Use current directory name
npx @vortideck/plugin-sdk init

# Install dependencies
npm install

# Build your plugin
npm run build
```

This creates a complete plugin project with:
- ✅ `package.json` with build scripts
- ✅ `tsconfig.json` with optimal TypeScript settings
- ✅ `src/main.ts` with example plugin code
- ✅ `manifest.json` for plugin metadata
- ✅ `README.md` with documentation
- ✅ `.gitignore` for version control

### Example Plugin

The generated `src/main.ts` looks like this:

```typescript
import vortideck, { createPlugin, defineNode, FlowParams } from '@vortideck/plugin-sdk'

export const { execute, register, get_metadata } = createPlugin({
  nodes: {
    example: defineNode({
      metadata: {
        id: "my-plugin:example",
        name: "My Plugin",
        description: "An example plugin",
        category: "utility",
        type_color: "#4A90E2"
      },
      execute(input) {
        vortideck.log("Plugin executing...")
        const text = new FlowParams(input).getInput("Hello World")
        return { success: true, output: text.toUpperCase() }
      }
    })
  }
})
```

Just customize it and run `npm run build`!

## API Reference

### Namespace Import (Recommended)

```typescript
import vortideck from '@vortideck/plugin-sdk'

vortideck.log(message: string): boolean
vortideck.registerNode(metadata: string): boolean
vortideck.getContext(): string
```

### Named Imports

```typescript
import { log, registerNode, getContext } from '@vortideck/plugin-sdk'

log("Hello!")
const ctx = getContext()
```

## Available Functions

### `vortideck.log(message: string): boolean`

Log a message from your plugin. Messages appear in VortiDeck's console.

**Parameters:**
- `message`: The message to log

**Returns:** Success status

**Example:**
```typescript
vortideck.log("Processing started...")
```

### `vortideck.registerNode(metadata: string): boolean`

Register a custom flow node with VortiDeck.

**Parameters:**
- `metadata`: JSON string containing node metadata (id, name, description, category, etc.)

**Returns:** Success status

**Example:**
```typescript
const metadata = {
  id: "my-plugin:uppercase",
  name: "To Uppercase",
  description: "Convert text to uppercase",
  category: "text",
  type_color: "#4A90E2",
  outputs_count: 1
}
vortideck.registerNode(JSON.stringify(metadata))
```

### `vortideck.getContext(): string`

Get the current execution context.

**Returns:** Context object as JSON string

**Example:**
```typescript
const context = vortideck.getContext()
const parsed = JSON.parse(context)
console.log("Execution ID:", parsed.executionId)
```

## Building Plugins

### Quick Start with `createPlugin()`

**src/main.ts**:
```typescript
import vortideck, { createPlugin, defineNode, FlowParams } from '@vortideck/plugin-sdk'

export const { execute, register, get_metadata } = createPlugin({
  nodes: {
    example: defineNode({
      metadata: {
        id: "my-plugin:example",
        name: "My Plugin",
        description: "Does something useful",
        category: "utility",
        type_color: "#4A90E2"
      },
      execute(input) {
        const text = new FlowParams(input).getInput("Hello World")
        return { success: true, output: text.toUpperCase() }
      }
    })
  }
})
```

**package.json**:
```json
{
  "scripts": {
    "build": "vortideck build"
  }
}
```

**Build**:
```bash
npm run build
```

That's it! Your plugin WASM is in `dist/`.

## Type Safety

All functions are fully typed with TypeScript. The SDK is auto-generated from VortiDeck's Rust host functions, ensuring perfect type alignment.

```typescript
// ✅ Type-safe - TypeScript knows the signature
vortideck.log("Hello")

// ❌ Type error - wrong argument type
vortideck.log(123)  // Error: Expected string

// ✅ Auto-complete works
vortideck.  // IDE suggests: log, registerNode, getContext
```

## Utilities

The SDK also exports helpful utilities:

```typescript
import {
  safeJsonParse,
  success,
  failure
} from '@vortideck/plugin-sdk'

// Parse JSON safely with fallback
const data = safeJsonParse(input, { default: "value" })

// Create standard responses
const ok = success("result")
const error = failure("Invalid input", "VALIDATION_ERROR")
```

## Platform Support

| Platform | Architecture | Status |
|----------|-------------|--------|
| Windows | x64 | ✅ Tested |
| Windows | ARM64 | Not bundled in SDK 0.1.x |
| macOS | x64 (Intel) | 🔵 Bundled |
| macOS | ARM64 (M1/M2) | 🔵 Bundled |
| Linux | x64 | ✅ Tested |
| Linux | ARM64 | Not bundled in SDK 0.1.x |
| WSL | x64 | ✅ Tested |

**Testing**: You can test on all platforms using GitHub Actions CI/CD (`.github/workflows/test-sdk.yml`)

## Runtime compatibility

SDK `0.1.x` generates plugin schema `1.0` packages for VortiDeck `0.7.0` or
newer. The SDK and desktop application use independent semantic versions; a
plugin's generated `manifest.json` records the minimum compatible VortiDeck
runtime through `vortideck_version`.

## How It Works

The SDK bundles platform-specific native binaries:
- **extism-js** (~73MB): Compiles JavaScript to WASM
- **binaryen** (~58MB): WASM optimization tools (wasm-merge, wasm-opt)

On build, the SDK:
1. Auto-detects your platform (OS + architecture)
2. Selects the correct binaries from the bundle
3. **For WSL**: Copies binaries to `/tmp` with executable permissions (workaround for Windows filesystem)
4. Builds your plugin automatically - zero configuration needed!

The npm package bundles these binaries. A source checkout of the VortiDeck repository does not: there, the first
`vortideck build` downloads the pinned releases (extism-js v1.5.1 and binaryen version_125) from their official GitHub
releases, checks their SHA-256 hashes and caches them per user (`%LOCALAPPDATA%\vortideck\toolchain` on Windows,
`~/Library/Caches/vortideck/toolchain` on macOS, `~/.cache/vortideck/toolchain` on Linux). Set
`VORTIDECK_TOOLCHAIN_DIR` to use another cache folder.

## CLI Commands

### `vortideck init [name]`

Creates a new plugin project with all necessary files.

**Usage:**
```bash
# Create plugin with specific name
vortideck init my-awesome-plugin

# Use current directory name
vortideck init
```

**Creates:**
- `package.json` - Build scripts and dependencies
- `tsconfig.json` - TypeScript configuration
- `src/main.ts` - Plugin implementation with example code
- `manifest.json` - Plugin metadata
- `README.md` - Documentation template
- `.gitignore` - Git ignore rules
- `dist/` - Output directory

### `vortideck build`

Builds your plugin to WASM.

**Pipeline:** TypeScript type check → esbuild bundle → extism-js compile → WASM output

**Output:** `dist/[plugin-name].wasm`

## Legal & Licenses

**SDK License**: MIT

**Bundled Third-Party Software**:
- [Binaryen](https://github.com/WebAssembly/binaryen) (Apache-2.0) - WASM toolchain
- [Extism-JS](https://github.com/extism/js-pdk) (BSD-3-Clause) - JS to WASM compiler

✅ **Both licenses allow redistribution**. See [THIRD_PARTY_LICENSES.md](./THIRD_PARTY_LICENSES.md) for full license texts.

## Size

**Total**: ~131MB (includes binaries for all platforms)

This ensures zero-configuration builds work everywhere, eliminating the need for users to manually install platform-specific tools!

## Version Compatibility

This SDK version corresponds to VortiDeck's host function API. Always use an SDK version that matches your VortiDeck version.
