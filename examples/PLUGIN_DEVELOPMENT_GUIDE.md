# 🔌 VortiDeck Plugin Development Guide

Welcome to VortiDeck plugin development! This guide will teach you how to create powerful, efficient plugins using our modern FlowParams-based architecture.

## 📋 Table of Contents

1. [Quick Start](#quick-start)
2. [Plugin Architecture](#plugin-architecture)
3. [FlowParams Helper](#flowparams-helper)
4. [Plugin Structure](#plugin-structure)
5. [Development Workflow](#development-workflow)
6. [Best Practices](#best-practices)
7. [Advanced Features](#advanced-features)
8. [Troubleshooting](#troubleshooting)

## 🚀 Quick Start

### Prerequisites
- Node.js 18+ installed
- TypeScript knowledge
- Basic understanding of WASM concepts

### Create Your First Plugin

```bash
# Copy the plugin template
cp -r examples/plugin-xtp examples/my-first-plugin
cd examples/my-first-plugin

# Install dependencies
npm install

# Build your plugin
npm run build
```

## 🏗️ Plugin Architecture

VortiDeck plugins use a **direct parameter** approach with **FlowParams helper** for maximum performance:

```
Flow System → Raw Parameters → Plugin → FlowParams Helper → Type-Safe Access
```

**Key Benefits:**
- ⚡ **Zero transformation overhead**
- 🛡️ **Type-safe parameter access**
- 🔄 **Handles both flow format and direct values**
- 📝 **Clean, readable code**

## 🛠️ FlowParams Helper

The `FlowParams` class automatically handles VortiDeck's parameter format:

### Basic Usage

```typescript
import { FlowParams, ExecutionParams, ExecutionResult } from './pdk';

export function executeImpl(input: ExecutionParams): ExecutionResult {
  // Create FlowParams helper
  const flowParams = new FlowParams(input);
  
  // Extract parameters with type safety and defaults
  const inputText = flowParams.getInput(""); // Main input text
  const shouldUppercase = flowParams.getBoolean('uppercase', false);
  const prefix = flowParams.getString('addPrefix', '');
  const count = flowParams.getNumber('repeatCount', 1);
  
  // Your plugin logic here...
  let result = inputText;
  if (shouldUppercase) result = result.toUpperCase();
  if (prefix) result = prefix + result;
  
  return {
    success: true,
    output: result,
    outputs: [result]
  };
}
```

### FlowParams API Reference

| Method | Description | Example |
|--------|-------------|---------|
| `getString(key, default)` | Extract string value | `flowParams.getString('text', '')` |
| `getBoolean(key, default)` | Extract boolean value | `flowParams.getBoolean('enabled', false)` |
| `getNumber(key, default)` | Extract number value | `flowParams.getNumber('count', 0)` |
| `getInput(default)` | Auto-find main input | `flowParams.getInput('')` |
| `isAttached(key)` | Check if connected to another node | `flowParams.isAttached('input')` |
| `getKeys()` | Get all parameter names | `flowParams.getKeys()` |
| `getRaw(key)` | Get raw parameter object | `flowParams.getRaw('debug')` |
| `toSimpleObject()` | Convert to simple key-value | `flowParams.toSimpleObject()` |

### Parameter Format Handling

FlowParams automatically handles both formats:

```typescript
// Flow format (automatic)
{
  "text": {"value": "Hello", "type": "string", "attached": false}
}

// Direct format (backward compatibility)  
{
  "text": "Hello"
}

// Both extract the same way:
const text = flowParams.getString('text', ''); // → "Hello"
```

## 📁 Plugin Structure

### Required Files

```
my-plugin/
├── src/
│   ├── main.ts          # Main plugin logic
│   ├── index.ts         # Plugin exports (auto-generated)
│   └── pdk.ts           # Plugin SDK (copy from template)
├── package.json         # Dependencies and build scripts
├── tsconfig.json        # TypeScript configuration
└── esbuild.js          # Build configuration
```

### main.ts Template

```typescript
import {
  ExecutionParams,
  ExecutionResult, 
  PluginMetadata,
  FlowParams,
} from './pdk';

/**
 * Execute the plugin with given parameters
 */
export function executeImpl(input: ExecutionParams): ExecutionResult {
  try {
    const flowParams = new FlowParams(input);
    
    // Extract your parameters
    const inputText = flowParams.getInput("");
    const config = {
      // Add your configuration options
      option1: flowParams.getString('option1', 'default'),
      option2: flowParams.getBoolean('option2', false),
    };
    
    // Your plugin logic here
    const result = processData(inputText, config);
    
    return {
      success: true,
      output: result,
      outputs: [result]
    };
    
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
      error_code: "EXECUTION_ERROR"
    };
  }
}

/**
 * Plugin metadata for VortiDeck
 */
export function get_metadataImpl(input: string): PluginMetadata {
  return {
    id: "my-awesome-plugin",
    name: "My Awesome Plugin", 
    description: "Does amazing things with text",
    category: "TEXT_PROCESSING",
    category_display_name: "Text Tools",
    type_color: "#4CAF50",
    outputs_count: 1,
    version: "1.0.0",
    author: "Your Name",
    tags: ["text", "processing", "utility"],
    parameters_schema: {
      type: "object",
      properties: {
        input: {
          type: "string",
          title: "Input Text",
          description: "Text to process",
          default: ""
        },
        option1: {
          type: "string", 
          title: "Option 1",
          description: "Your first option",
          default: "default"
        },
        option2: {
          type: "boolean",
          title: "Option 2", 
          description: "Your second option",
          default: false
        }
      }
    }
  };
}

/**
 * Register the plugin
 */
export function registerImpl(input: string): string {
  return "Plugin registered successfully";
}

// Your helper functions
function processData(text: string, config: any): string {
  // Implement your logic here
  return text;
}
```

## 🔄 Development Workflow

### 1. Setup Development Environment

```bash
# Create new plugin from template
cp -r examples/plugin-xtp examples/my-new-plugin
cd examples/my-new-plugin

# Update package.json
nano package.json  # Change name, description

# Install dependencies
npm install
```

### 2. Develop Your Plugin

```bash
# Edit your plugin logic
nano src/main.ts

# Build and test
npm run build

# Check the generated WASM file
ls dist/plugin.wasm
```

### 3. Test in VortiDeck

1. Copy `dist/plugin.wasm` to your VortiDeck plugins directory
2. The hot reloading system will automatically detect it
3. Your plugin appears in the flow builder
4. Test with different parameter combinations

### 4. Debug Issues

```typescript
// Add logging to your plugin
import { vortideck_log } from './pdk';

function logMessage(message: string): void {
  try {
    const encoder = new TextEncoder();
    const buffer = encoder.encode(message);
    vortideck_log(buffer);
  } catch (error) {
    console.log(message); // Fallback
  }
}

export function executeImpl(input: ExecutionParams): ExecutionResult {
  const flowParams = new FlowParams(input);
  
  // Debug parameter extraction
  logMessage(`Plugin received: ${JSON.stringify(flowParams.toSimpleObject())}`);
  
  // Your logic...
}
```

## ✅ Best Practices

### 1. Parameter Validation

```typescript
export function executeImpl(input: ExecutionParams): ExecutionResult {
  const flowParams = new FlowParams(input);
  
  // Validate required parameters
  const inputText = flowParams.getInput("");
  if (!inputText.trim()) {
    return {
      success: false,
      error: "Input text is required",
      error_code: "MISSING_INPUT"
    };
  }
  
  // Validate ranges
  const count = flowParams.getNumber('count', 1);
  if (count < 1 || count > 100) {
    return {
      success: false,
      error: "Count must be between 1 and 100",
      error_code: "INVALID_RANGE"
    };
  }
  
  // Continue with processing...
}
```

### 2. Error Handling

```typescript
export function executeImpl(input: ExecutionParams): ExecutionResult {
  try {
    const flowParams = new FlowParams(input);
    
    // Your logic with proper error handling
    const result = riskyOperation(flowParams);
    
    return {
      success: true,
      output: result,
      outputs: [result]
    };
    
  } catch (error) {
    // Provide helpful error messages
    const errorMessage = error instanceof Error ? error.message : String(error);
    
    return {
      success: false,
      error: `Processing failed: ${errorMessage}`,
      error_code: getErrorCode(error)
    };
  }
}

function getErrorCode(error: any): string {
  if (error instanceof TypeError) return "TYPE_ERROR";
  if (error instanceof RangeError) return "RANGE_ERROR";
  return "UNKNOWN_ERROR";
}
```

### 3. Performance Optimization

```typescript
export function executeImpl(input: ExecutionParams): ExecutionResult {
  const flowParams = new FlowParams(input);
  
  // Only extract parameters you actually use
  const inputText = flowParams.getInput("");
  
  // Check if expensive operations are needed
  if (!flowParams.getBoolean('enableAdvanced', false)) {
    return simpleProcessing(inputText);
  }
  
  // Only do expensive work when required
  return advancedProcessing(inputText, flowParams);
}
```

### 4. Schema Design

```typescript
// Good schema design
parameters_schema: {
  type: "object",
  properties: {
    input: {
      type: "string",
      title: "Input Text",
      description: "The text to process",
      default: ""
    },
    mode: {
      type: "string",
      title: "Processing Mode",
      description: "How to process the text",
      enum: ["simple", "advanced", "custom"],
      default: "simple"
    },
    settings: {
      type: "object",
      title: "Advanced Settings",
      description: "Additional configuration options",
      properties: {
        preserveFormatting: {
          type: "boolean",
          title: "Preserve Formatting",
          default: true
        },
        maxLength: {
          type: "number", 
          title: "Maximum Length",
          minimum: 1,
          maximum: 10000,
          default: 1000
        }
      }
    }
  },
  required: ["input"]
}
```

## 🚀 Advanced Features

### Multiple Outputs

```typescript
export function executeImpl(input: ExecutionParams): ExecutionResult {
  const flowParams = new FlowParams(input);
  const inputText = flowParams.getInput("");
  
  // Process data into multiple outputs
  const uppercase = inputText.toUpperCase();
  const lowercase = inputText.toLowerCase(); 
  const wordCount = inputText.split(' ').length;
  
  return {
    success: true,
    output: uppercase, // Primary output
    outputs: [uppercase, lowercase, wordCount.toString()] // All outputs
  };
}

// In metadata:
outputs_count: 3
```

### Connected Parameter Detection

```typescript
export function executeImpl(input: ExecutionParams): ExecutionResult {
  const flowParams = new FlowParams(input);
  
  // Check if parameter is connected to another node
  if (flowParams.isAttached('input')) {
    // Handle connected input (from another node)
    const inputText = flowParams.getString('input', '');
    return processConnectedInput(inputText);
  } else {
    // Handle manual input
    const inputText = flowParams.getInput('');
    return processManualInput(inputText);
  }
}
```

### Dynamic Configuration

```typescript
export function executeImpl(input: ExecutionParams): ExecutionResult {
  const flowParams = new FlowParams(input);
  
  // Get all available parameters
  const allKeys = flowParams.getKeys();
  const config: Record<string, any> = {};
  
  // Build dynamic configuration
  for (const key of allKeys) {
    if (key.startsWith('custom_')) {
      config[key] = flowParams.getString(key, '');
    }
  }
  
  return processDynamicConfig(flowParams.getInput(''), config);
}
```

## 🐛 Troubleshooting

### Common Issues

**Plugin not appearing in VortiDeck:**
```bash
# Check build output
npm run build
ls dist/plugin.wasm  # Should exist

# Check metadata
grep -r "get_metadata" src/
```

**Parameters not extracting correctly:**
```typescript
// Debug parameter structure
const flowParams = new FlowParams(input);
console.log("Raw params:", flowParams.toSimpleObject());
console.log("Available keys:", flowParams.getKeys());
```

**Build failures:**
```bash
# Check TypeScript errors
npx tsc --noEmit

# Check esbuild configuration
cat esbuild.js
```

**Runtime errors:**
```typescript
// Add comprehensive error handling
try {
  // Your logic
} catch (error) {
  console.error("Plugin error:", error);
  return {
    success: false,
    error: `Detailed error: ${error}`,
    error_code: "DEBUG_ERROR"
  };
}
```

## 📚 Next Steps

1. **Study Examples:** Check `examples/plugin-xtp/` for a working plugin
2. **API Reference:** See [Plugin API Reference](./PLUGIN_API_REFERENCE.md)
3. **Community:** Join our plugin developer community

## 🎯 Plugin Ideas

- **Text Processors:** Formatting, validation, transformation
- **Data Converters:** JSON, CSV, XML processing
- **Integrations:** APIs, databases, external services
- **Utilities:** Math, date/time, string operations
- **AI/ML:** Text analysis, sentiment, classification

Happy plugin development! 🚀 