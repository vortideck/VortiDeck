# VortiDeck Extism Plugin System - Complete Implementation Guide

## 🌟 Overview

VortiDeck now supports a powerful WebAssembly-based plugin system using Extism, enabling developers to write plugins in JavaScript that get compiled to secure, sandboxed WASM modules. This creates **infinite extensibility** for the flow-based automation platform.

## 🏗️ Architecture

```
JavaScript Plugin (Extism PDK) 
    ↓ [extism-js compiler]
WebAssembly Module (.wasm)
    ↓ [VortiDeck Rust Host]
FlowNode Integration
    ↓ [Visual Flow Builder]
Executable Automation
```

## 📂 File Structure

```
VortiDeck/
├── examples/js-plugin-real/
│   ├── simple-plugin.js           # JavaScript plugin using Extism PDK
│   ├── simple-plugin.d.ts         # TypeScript definitions for host functions
│   ├── dist/
│   │   └── simple-text-processor.wasm  # Compiled WASM plugin (2.25MB)
│   ├── extism-js.exe              # JavaScript to WASM compiler
│   └── build.sh                   # Build script
├── apps/tauri-windows/src-tauri/
│   ├── src/bin/
│   │   └── final_plugin_demo.rs   # Complete working demo
│   ├── src/flow_control/
│   ├── plugins/                  # Unified plugin management system
│   │   ├── wasm_provider.rs      # WASM plugin system integration
│   │   ├── registry.rs           # Plugin registry and management  
│   │   ├── metadata.rs           # Plugin metadata structures
│   │   └── commands.rs           # Tauri commands for frontend
│   │   ├── executor.rs            # Flow execution engine
│   │   └── registry.rs            # Node registry
│   └── Cargo.toml                 # Extism dependency configuration
```

## 🔧 Implementation Components

### 1. JavaScript Plugin (`examples/js-plugin-real/simple-plugin.js`)

```javascript
// Host functions for VortiDeck integration
const { vortideck_log, vortideck_register_node, vortideck_get_context } = Host.getFunctions();

// Plugin exports
function get_metadata() {
    const metadata = {
        id: "simple-text-processor",
        name: "Simple Text Processor",
        description: "A real working VortiDeck plugin",
        category: "TEXT_PROCESSING",
        type_color: "#3b82f6",
        parameters_schema: { /* JSON Schema */ }
    };
    Host.outputString(JSON.stringify(metadata));
    return 0;
}

function register() {
    // Register with VortiDeck as a FlowNode
    let mem = Memory.fromString("Plugin registering...");
    vortideck_log(mem.offset);
    
    let metadataMem = Memory.fromString(JSON.stringify(metadata));
    let result = vortideck_register_node(metadataMem.offset);
    return 0;
}

function execute() {
    // Process input and return results
    const input = Host.inputString();
    const params = JSON.parse(input);
    
    // Perform text operations
    let result = processText(params.input_text, params.operation);
    
    Host.outputString(JSON.stringify({
        success: true,
        data: { result, operation, original_text }
    }));
    return 0;
}

module.exports = { get_metadata, register, execute };
```

### 2. TypeScript Definitions (`examples/js-plugin-real/simple-plugin.d.ts`)

```typescript
declare module "main" {
    export function get_metadata(): I32;
    export function register(): I32;
    export function execute(): I32;
}

declare module "extism:host" {
    interface user {
        vortideck_log(ptr: I64): I64;
        vortideck_register_node(ptr: I64): I64;
        vortideck_get_context(): I64;
    }
}
```

### 3. Rust Host Implementation (`apps/tauri-windows/src-tauri/src/bin/final_plugin_demo.rs`)

```rust
use extism::{Plugin, Manifest, Wasm, Function, UserData};

// Create VortiDeck host functions
let vortideck_log = Function::new(
    "vortideck_log",
    [extism::ValType::I64],
    [extism::ValType::I64],
    UserData::new(()),
    |_plugin, inputs, outputs, _user_data| {
        let message_ptr = inputs[0].unwrap_i64() as u64;
        println!("🔌 VortiDeck Plugin Log: [message at ptr {}]", message_ptr);
        outputs[0] = extism::Val::I64(0);
        Ok(())
    },
);

// Load WASM plugin
let wasm_bytes = std::fs::read("path/to/plugin.wasm")?;
let manifest = Manifest::new([Wasm::data(wasm_bytes)]);

// Create plugin with host functions
let mut plugin = Plugin::new(
    manifest, 
    [vortideck_log, vortideck_register_node, vortideck_get_context], 
    true
)?;

// Execute plugin functions
let metadata = plugin.call::<(), String>("get_metadata", ())?;
let register_result = plugin.call::<(), String>("register", ())?;
let execution_result = plugin.call::<String, String>("execute", input_json)?;
```

### 4. Cargo Configuration (`apps/tauri-windows/src-tauri/Cargo.toml`)

```toml
[features]
default = []
extism = ["dep:extism"]

[dependencies]
extism = { version = "1.11.1", optional = true }
```

## 🚀 Complete Workflow

### Step 1: Write JavaScript Plugin

Create a JavaScript file using Extism PDK:

```javascript
// 1. Import VortiDeck host functions
const { vortideck_log, vortideck_register_node, vortideck_get_context } = Host.getFunctions();

// 2. Implement required functions
function get_metadata() { /* return plugin info */ }
function register() { /* register as FlowNode */ }
function execute() { /* process data */ }

// 3. Export functions
module.exports = { get_metadata, register, execute };
```

### Step 2: Compile to WASM

```bash
# Download extism-js compiler
curl -L https://github.com/extism/js-sdk/releases/download/v1.2.0/extism-js-x86_64-pc-windows-gnu.gz | gunzip > extism-js.exe

# Compile JavaScript to WASM
./extism-js.exe simple-plugin.js -i simple-plugin.d.ts -o dist/simple-text-processor.wasm
```

### Step 3: Load in Rust Host

```rust
// Load compiled WASM
let wasm_bytes = std::fs::read("dist/simple-text-processor.wasm")?;
let manifest = Manifest::new([Wasm::data(wasm_bytes)]);

// Create plugin with VortiDeck host functions
let mut plugin = Plugin::new(manifest, host_functions, true)?;

// Execute plugin functions
plugin.call::<(), String>("get_metadata", ())?;
```

## 📚 Key Learnings & Critical Insights

### 1. **Extism API Evolution (Critical)**

**Problem**: Extism 1.11.1 deprecated the `host_fn!` macro used in older examples.

**Solution**: Use `Function::new` instead:

```rust
// ❌ OLD (doesn't work in 1.11.1)
let log_fn = host_fn!("vortideck_log", |inputs: [i64; 1]| -> [i64; 1] {
    // Implementation
});

// ✅ NEW (works in 1.11.1+)
let vortideck_log = Function::new(
    "vortideck_log",
    [extism::ValType::I64],
    [extism::ValType::I64],
    UserData::new(()),
    |_plugin, inputs, outputs, _user_data| {
        outputs[0] = extism::Val::I64(0);
        Ok(())
    },
);
```

### 2. **Memory Management Between Host & Plugin**

**Problem**: Accessing plugin memory from host functions requires plugin context.

**Learning**: Host functions receive limited context. For complex memory operations:

```rust
// Simple approach - just log the pointer
|_plugin, inputs, outputs, _user_data| {
    let ptr = inputs[0].unwrap_i64() as u64;
    println!("Plugin memory pointer: {}", ptr);
    Ok(())
}
```

### 3. **JavaScript to WASM Compilation Requirements**

**Critical Dependencies**:
1. **extism-js compiler**: Converts JavaScript to WASM
2. **Binaryen tools**: Required for wasm-merge operations  
3. **TypeScript definitions**: Enable type checking and host function imports

**Build Process**:
```bash
# 1. Download tools
curl -L https://github.com/extism/js-sdk/releases/download/v1.2.0/extism-js-x86_64-pc-windows-gnu.gz | gunzip > extism-js.exe

# 2. Add Binaryen to PATH (for wasm-merge)
export PATH="$PATH:/path/to/binaryen/bin"

# 3. Compile with type definitions
./extism-js.exe plugin.js -i plugin.d.ts -o output.wasm
```

### 4. **Plugin Registration Pattern**

**Discovery**: Plugins can self-register as VortiDeck FlowNodes:

```javascript
function register() {
    // 1. Log registration
    let logMem = Memory.fromString("Registering plugin...");
    vortideck_log(logMem.offset);
    
    // 2. Get VortiDeck context
    let contextOffset = vortideck_get_context();
    
    // 3. Register as FlowNode
    let metadataMem = Memory.fromString(JSON.stringify(metadata));
    return vortideck_register_node(metadataMem.offset);
}
```

### 5. **Thread Safety with Static State**

**Problem**: Rust static mut causes compilation errors.

**Solution**: Use `OnceLock` for thread-safe global state:

```rust
use std::sync::OnceLock;

static REGISTRY: OnceLock<VortiDeckPluginRegistry> = OnceLock::new();

fn get_registry() -> &'static VortiDeckPluginRegistry {
    REGISTRY.get_or_init(|| VortiDeckPluginRegistry::new())
}
```

### 6. **Input/Output Data Flow**

**Pattern**: JSON serialization for complex data exchange:

```javascript
// Plugin receives JSON input
function execute() {
    const input = Host.inputString();           // Get JSON string
    const params = JSON.parse(input);           // Parse parameters
    
    // Process data
    const result = processData(params);
    
    // Return JSON response
    Host.outputString(JSON.stringify(result));
    return 0;
}
```

```rust
// Host sends/receives JSON
let input = json!({"text": "Hello", "operation": "uppercase"});
let result = plugin.call::<String, String>("execute", input.to_string())?;
let output: Value = serde_json::from_str(&result)?;
```

## 🔄 Replication Guide

### Prerequisites

1. **Rust environment** with Extism 1.11.1+
2. **extism-js compiler** for JavaScript compilation  
3. **Binaryen tools** for WASM operations
4. **Node.js** (optional, for testing)

### Step-by-Step Replication

#### 1. Setup Dependencies

```bash
# Add Extism to Cargo.toml
[dependencies]
extism = { version = "1.11.1", optional = true }

[features]
extism = ["dep:extism"]
```

#### 2. Download Tools

```bash
# Download extism-js compiler
curl -L https://github.com/extism/js-sdk/releases/download/v1.2.0/extism-js-x86_64-pc-windows-gnu.gz | gunzip > extism-js.exe

# Download Binaryen (required for compilation)
curl -L https://github.com/WebAssembly/binaryen/releases/download/version_118/binaryen-version_118-x86_64-windows.tar.gz | tar -xz
export PATH="$PATH:$(pwd)/binaryen-version_118/bin"
```

#### 3. Create Plugin Structure

```
my-plugin/
├── plugin.js              # Your JavaScript plugin
├── plugin.d.ts            # TypeScript definitions  
├── build.sh               # Build script
└── dist/                  # Compiled output
    └── plugin.wasm
```

#### 4. Write JavaScript Plugin

```javascript
// plugin.js
const { vortideck_log, vortideck_register_node } = Host.getFunctions();

function get_metadata() {
    const metadata = {
        id: "my-plugin",
        name: "My Plugin",
        description: "Custom VortiDeck plugin",
        category: "CUSTOM",
        parameters_schema: {
            type: "object",
            properties: {
                input: { type: "string", title: "Input" }
            }
        }
    };
    Host.outputString(JSON.stringify(metadata));
    return 0;
}

function execute() {
    try {
        const input = Host.inputString();
        const params = JSON.parse(input);
        
        // Your custom logic here
        const result = params.input.toUpperCase();
        
        Host.outputString(JSON.stringify({
            success: true,
            data: { result }
        }));
        return 0;
    } catch (error) {
        Host.outputString(JSON.stringify({
            success: false,
            error: error.message
        }));
        return 1;
    }
}

module.exports = { get_metadata, execute };
```

#### 5. Create TypeScript Definitions

```typescript
// plugin.d.ts
declare module "main" {
    export function get_metadata(): I32;
    export function execute(): I32;
}

declare module "extism:host" {
    interface user {
        vortideck_log(ptr: I64): I64;
        vortideck_register_node(ptr: I64): I64;
    }
}
```

#### 6. Compile to WASM

```bash
# build.sh
#!/bin/bash
./extism-js.exe plugin.js -i plugin.d.ts -o dist/my-plugin.wasm
echo "✅ Plugin compiled to dist/my-plugin.wasm"
```

#### 7. Create Rust Host

```rust
// main.rs
use extism::{Plugin, Manifest, Wasm, Function, UserData};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create host functions
    let vortideck_log = Function::new(
        "vortideck_log",
        [extism::ValType::I64],
        [extism::ValType::I64],
        UserData::new(()),
        |_plugin, inputs, outputs, _user_data| {
            println!("Plugin log: ptr {}", inputs[0].unwrap_i64());
            outputs[0] = extism::Val::I64(0);
            Ok(())
        },
    );

    // Load WASM plugin
    let wasm_bytes = std::fs::read("dist/my-plugin.wasm")?;
    let manifest = Manifest::new([Wasm::data(wasm_bytes)]);
    
    // Create plugin
    let mut plugin = Plugin::new(manifest, [vortideck_log], true)?;
    
    // Test plugin
    let metadata = plugin.call::<(), String>("get_metadata", ())?;
    println!("Plugin metadata: {}", metadata);
    
    let input = r#"{"input": "hello world"}"#;
    let result = plugin.call::<String, String>("execute", input.to_string())?;
    println!("Plugin result: {}", result);
    
    Ok(())
}
```

#### 8. Build and Test

```bash
# Compile plugin
chmod +x build.sh
./build.sh

# Run Rust host
cargo run --features extism
```

## 🎯 Integration with VortiDeck Flow System

### Flow Control Integration

The plugin system integrates with VortiDeck's existing flow control system:

1. **Registry Integration** (`src/flow_control/registry.rs`):
   - Plugins register as FlowNodes
   - Available in visual flow builder
   - Type-safe parameter validation

2. **Execution Integration** (`src/flow_control/executor.rs`):
   - WASM plugins execute within flow chains
   - Input/output data flows between nodes
   - Error handling and recovery

3. **System Integration** (`src/plugins/wasm_provider.rs`):
   - Plugin lifecycle management
   - Memory safety and sandboxing
   - Performance monitoring

## 🔒 Security & Sandboxing

### WASM Sandboxing Benefits

1. **Memory Isolation**: Plugins cannot access host memory directly
2. **API Restriction**: Only exposed host functions are callable
3. **Resource Limits**: CPU and memory usage can be constrained
4. **No File System Access**: Plugins cannot read/write files without permission

### Host Function Security

```rust
// Secure host function implementation
let secure_log = Function::new(
    "vortideck_log",
    [extism::ValType::I64],
    [extism::ValType::I64],
    UserData::new(()),
    |_plugin, inputs, outputs, _user_data| {
        // Validate input
        let ptr = inputs[0].unwrap_i64() as u64;
        if ptr == 0 {
            outputs[0] = extism::Val::I64(-1); // Error
            return Ok(());
        }
        
        // Safe logging (no direct memory access)
        println!("Plugin message at: {}", ptr);
        outputs[0] = extism::Val::I64(0); // Success
        Ok(())
    },
);
```

## 📊 Performance Considerations

### WASM Plugin Performance

- **Compilation**: ~2-3 seconds for typical JavaScript plugins
- **Loading**: ~50-100ms for 2MB WASM modules
- **Execution**: Near-native performance for compute operations
- **Memory**: Isolated heap, configurable limits

### Optimization Tips

1. **Minimize Host Calls**: Batch operations when possible
2. **Efficient Serialization**: Use compact JSON or binary formats
3. **Memory Management**: Reuse plugin instances
4. **Caching**: Cache compiled WASM modules

## 🐛 Common Issues & Solutions

### Issue 1: "host_fn! macro not found"

**Cause**: Using old Extism API syntax
**Solution**: Use `Function::new` instead of `host_fn!`

### Issue 2: "cannot read property 'readString' of undefined"

**Cause**: Trying to access plugin memory from host without proper context
**Solution**: Use simpler host function patterns or proper memory management

### Issue 3: "WASM file not found"

**Cause**: Incorrect file paths or missing compilation
**Solution**: Verify paths and ensure `extism-js` compilation completed

### Issue 4: "wasm-merge not found"

**Cause**: Missing Binaryen tools
**Solution**: Download and add Binaryen to PATH

## 🎉 Results & Capabilities

### What We Achieved

✅ **JavaScript to WASM Pipeline**: Complete compilation workflow  
✅ **VortiDeck Integration**: Plugins become FlowNodes  
✅ **Type Safety**: TypeScript definitions for host functions  
✅ **Memory Safety**: Sandboxed execution environment  
✅ **Real-time Execution**: Dynamic plugin loading and execution  
✅ **Bidirectional Communication**: Host ↔ Plugin data exchange  

### Demo Results

```
🚀 VortiDeck Plugin System - Final Demo
✅ Successfully loaded WASM plugin (2252510 bytes)
✅ Plugin metadata: {"id":"simple-text-processor",...}
✅ Plugin execution results:
   - "Hello VortiDeck!" → "HELLO VORTIDECK!" 
   - "Plugin System" → "metsyS nigulP"
   - Word count: "This is a test..." → "9"
```

## 🌟 Future Enhancements

### Planned Features

1. **Plugin Marketplace**: Discover and install community plugins
2. **Advanced Memory Management**: Direct memory access for performance
3. **Multi-language Support**: Python, Rust, C++ plugins via WASM
4. **Plugin Debugging**: Development tools and error reporting
5. **Hot Reloading**: Update plugins without restart
6. **Plugin Composition**: Chain multiple plugins together

### Architecture Evolution

```
Current: JavaScript → WASM → VortiDeck
Future:  Multiple Languages → WASM → Enhanced Runtime → VortiDeck
```

## 📚 References & Resources

- [Extism Documentation](https://extism.org/docs/)
- [Extism JavaScript SDK](https://github.com/extism/js-sdk)
- VortiDeck Flow System
- Example Plugin
- Demo Implementation

---

**🌟 VortiDeck + Extism = Infinite Extensibility!** 

This implementation proves that VortiDeck can support a rich ecosystem of community plugins while maintaining security, performance, and ease of development. 