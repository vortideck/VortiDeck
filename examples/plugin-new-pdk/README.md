# Low-level generated PDK fixture

> This directory is an internal generator compatibility fixture. Plugin authors
> should use `@vortideck/plugin-sdk` and `vortideck build`; this is not a second
> supported SDK entry point.

This example demonstrates VortiDeck's **automatic SDK generation system** that creates type-safe TypeScript wrappers from Rust host functions.

## 🎯 What This Demonstrates

### ✅ Auto-Generated PDK
The `src/pdk.generated.ts` file is **automatically generated** from Rust host functions in `apps/tauri-windows/libs/vortideck-core/src/plugins/manager.rs`.

When a `#[vortideck_host]` function changes in Rust, run
`pnpm generate:plugin-sdk`. Ordinary Rust builds verify generated files and fail
if they are stale; they do not rewrite the repository.

### 🔄 The Magic Flow

```
1. Write Rust function with #[vortideck_host] macro
   ↓
2. Run: pnpm generate:plugin-sdk
   ↓
3. build.rs extracts function metadata
   ↓
4. TypeScript PDK auto-generates (pdk.generated.ts)
   ↓
5. Plugin uses clean, type-safe API!
```

## 📊 API Comparison

### ❌ Old Manual Approach
```typescript
// Requires manual ArrayBufferLike encoding
const encoder = new TextEncoder();
const buffer = encoder.encode(message);
vortideck_log(buffer); // No type safety
```

### ✅ New Auto-Generated Approach
```typescript
// Clean, type-safe API!
vortideck_log("Hello World"); // Just a string!
const context = vortideck_get_context(); // Returns parsed JSON
const success = vortideck_register_node(JSON.stringify(metadata)); // boolean
```

## 🏗️ Project Structure

```
plugin-new-pdk/
├── src/
│   ├── pdk.generated.ts  # ✨ AUTO-GENERATED from Rust
│   ├── main.ts           # Plugin implementation
│   ├── index.ts          # Extism entry point
│   └── index.d.ts        # Type definitions
├── manifest.json         # Plugin metadata
├── package.json          # Dependencies
├── esbuild.js           # Build configuration
└── xtp.toml             # XTP configuration
```

## 🚀 Building

### Prerequisites

1. **Node.js & npm** - Already installed ✅
2. **extism-js** - Required for final WASM compilation

#### Install extism-js (Windows PowerShell):
```powershell
Invoke-WebRequest -Uri https://raw.githubusercontent.com/extism/js-pdk/main/install-windows.ps1 -OutFile install-windows.ps1
.\install-windows.ps1
```

#### Install extism-js (Mac/Linux):
```bash
curl -L https://raw.githubusercontent.com/extism/js-pdk/main/install.sh | bash
```

### Build Command

```bash
npm install  # First time only
npm run build
```

### Build Output

- `dist/demo-plugin.wasm` - Compiled plugin
- `dist/demo-plugin.manifest.json` - Plugin metadata
- `dist/index.js` - Bundled JavaScript (intermediate)

## 🧪 Generated Host Functions

The PDK currently includes 3 auto-generated functions:

### `vortideck_log(message: string): string`
Log messages from your plugin.

```typescript
vortideck_log("Plugin started!");
```

### `vortideck_register_node(metadata: string): boolean`
Register your plugin with the flow system.

```typescript
const success = vortideck_register_node(JSON.stringify(metadata));
```

### `vortideck_get_context(): string`
Get execution context information.

```typescript
const context = vortideck_get_context();
console.log("Flow ID:", JSON.parse(context).flow_id);
```

## ➕ Adding New Host Functions

1. **Edit** `apps/tauri-windows/libs/vortideck-core/src/plugins/manager.rs`:

```rust
/// Your new host function description
#[cfg(feature = "extism")]
#[vortideck_host]
fn my_new_function(input: String) -> Result<String, String> {
    println!("Called with: {}", input);
    Ok("Success!".to_string())
}
```

2. **Register** the function in plugin creation (same file):

```rust
let my_new_function = my_new_function_function();

let plugin = extism::Plugin::new(
    manifest,
    [vortideck_log, vortideck_register_node, vortideck_get_context, my_new_function],
    true
)
```

3. **Regenerate** PDK:

```bash
cd apps/tauri-windows/libs/vortideck-core
cargo check
```

4. **Use** in your plugin:

```typescript
import { my_new_function } from './pdk.generated'

const result = my_new_function("test input");
```

## 🎨 Key Features

- ✅ **Type Safety** - Full TypeScript type checking
- ✅ **Auto-Generation** - PDK updates automatically
- ✅ **Clean API** - No manual buffer encoding
- ✅ **JSON Handling** - Automatic serialization/deserialization
- ✅ **Single Source** - Rust code is the source of truth
- ✅ **Manifest Embedding** - Metadata injected at build time

## 📝 Notes

- The `pdk.generated.ts` file should **never be edited manually**
- Run `cargo check` in vortideck-core to regenerate after adding host functions
- The macro generates `{function_name}_function()` wrappers
- Memory API is provided by Extism runtime (declared as global)

## 🔗 Related Files

- Rust host functions: `apps/tauri-windows/libs/vortideck-core/src/plugins/manager.rs`
- Proc-macro implementation: `apps/tauri-windows/libs/vortideck-host-macro/src/lib.rs`
- Build script (generator): `apps/tauri-windows/libs/vortideck-core/build.rs`
