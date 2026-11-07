# VortiDeck Node Development Guide

## Widget System - Available UI Controls

VortiDeck automatically renders UI controls based on your JSON Schema. No manual widget configuration needed!

---

## Quick Reference

| Widget | Trigger | Example |
|--------|---------|---------|
| **Slider** | `type: number` + `minimum` + `maximum` | Volume, opacity, speed |
| **Color Picker** | `format: "color"` | Background color, text color |
| **Toggle** | `type: boolean` | Enable/disable, on/off |
| **Dropdown** | `enum: [...]` | Mode selection, presets |
| **Textarea** | `type: string` + `maxLength > 100` | Notes, descriptions, code |
| **Text Input** | `type: string` (default) | Names, URLs, paths |
| **Number Input** | `type: number` (no min/max) | Count, index, ID |
| **File Picker** | `format: "file"` | Audio files, images, documents |
| **Multi-Select** | `type: array` + `items.enum` | Tags, categories, permissions |

---

## Optional vs Required Inputs

VortiDeck visually distinguishes between required and optional inputs using port indicators.

### Visual Indicators

| Input Type | Not Connected | Connected |
|------------|---------------|-----------|
| **Required** | ○ Single ring | ● Filled circle |
| **Optional** | ◎ Double ring | ◉ Filled with hollow center |

### Making Inputs Optional

**Method 1: Using `optional` field (Recommended)**

```rust
json!({
    "backgroundColor": {
        "type": "string",
        "format": "color",
        "description": "Background color (optional)",
        "default": "#3B82F6",
        "optional": true    // ← Marks as optional
    }
})
```

**Method 2: Using `required` array (JSON Schema standard)**

```rust
json!({
    "type": "object",
    "properties": {
        "message": {
            "type": "string",
            "description": "Message to send"
        },
        "priority": {
            "type": "string",
            "description": "Priority level",
            "enum": ["low", "medium", "high"],
            "default": "medium"
        }
    },
    "required": ["message"]  // ← Only message is required, priority is optional
})
```

### Behavior Rules

1. **Default**: All inputs are **required** unless explicitly marked optional
2. **Explicit optional**: Add `"optional": true` to make a field optional
3. **Backwards compatible**: If a `required` array exists:
   - Fields in the array → required
   - Fields NOT in the array → optional
4. **Best practice**: Use `"optional": true` for clarity

### Complete Example

```rust
pub struct MyNode;

#[async_trait]
impl FlowNode for MyNode {
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                // REQUIRED inputs (default behavior)
                "title": {
                    "type": "string",
                    "description": "Title (required)",
                    "default": "Untitled"
                },
                "enabled": {
                    "type": "boolean",
                    "description": "Enable feature (required)",
                    "default": true
                },

                // OPTIONAL inputs (explicit)
                "description": {
                    "type": "string",
                    "description": "Optional description",
                    "maxLength": 500,
                    "optional": true    // ← Optional field
                },
                "color": {
                    "type": "string",
                    "format": "color",
                    "description": "Optional color",
                    "default": "#3B82F6",
                    "optional": true    // ← Optional field
                }
            }
        })
    }

    async fn execute(&self, params: Value) -> Result<Value, String> {
        // Required fields - always present
        let title = params["title"].as_str().unwrap_or("Untitled");
        let enabled = params["enabled"].as_bool().unwrap_or(true);

        // Optional fields - may be null/undefined
        let description = params.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let color = params.get("color")
            .and_then(|v| v.as_str())
            .unwrap_or("#3B82F6");

        Ok(json!({ "success": true }))
    }
}
```

### When to Use Optional Inputs

**✅ Use optional for:**
- Configuration settings with sensible defaults
- Cosmetic options (colors, themes)
- Advanced features that aren't always needed
- Fields that may come from connections instead of manual input

**❌ Keep required for:**
- Critical data needed for node execution
- Inputs with no reasonable default
- Core functionality parameters

---

## 1. Slider Widget

**When:** Numeric values with min/max constraints

**Schema:**
```rust
json!({
    "volume": {
        "type": "number",
        "description": "Volume level",
        "default": 50,
        "minimum": 0,        // ← Required for slider
        "maximum": 100,      // ← Required for slider
        "multipleOf": 1      // Optional: step size
    }
})
```

**Result:** Dual slider + number input

**Use Cases:**
- Volume (0-100)
- Opacity (0-100)
- Speed/Rate (0.1-2.0)
- Percentage (0-100)
- Angle (0-360)

---

## 2. Color Picker Widget

**When:** Color values in hex format

**Schema:**
```rust
json!({
    "backgroundColor": {
        "type": "string",
        "format": "color",   // ← Triggers color picker
        "description": "Background color",
        "default": "#3B82F6"
    }
})
```

**Result:** Color preview box + hex input

**Use Cases:**
- Background colors
- Text colors
- Border colors
- Accent colors

---

## 3. Toggle Widget

**When:** Boolean on/off values

**Schema:**
```rust
json!({
    "enabled": {
        "type": "boolean",  // ← Triggers toggle
        "description": "Enable feature",
        "default": true
    }
})
```

**Result:** Animated toggle switch with On/Off label

**Use Cases:**
- Enable/disable features
- Show/hide elements
- Active/inactive states
- True/false flags

---

## 4. Dropdown (Select) Widget

**When:** Multiple predefined options

**Schema:**
```rust
json!({
    "mode": {
        "type": "string",
        "enum": ["normal", "advanced", "expert"],  // ← Triggers dropdown
        "description": "Operating mode",
        "default": "normal"
    }
})
```

**Result:** Select dropdown with capitalized options

**Use Cases:**
- Mode selection
- Presets
- Quality levels (low/medium/high)
- Format selection (mp3/wav/ogg)

---

## 5. Textarea Widget

**When:** Long text input

**Schema:**
```rust
json!({
    "notes": {
        "type": "string",
        "description": "Multi-line notes",
        "maxLength": 500,   // ← > 100 triggers textarea
        "default": "Enter your notes here..."
    }
})
```

**Options:**
```rust
json!({
    "description": {
        "type": "string",
        "maxLength": 1000,
        "rows": 5           // Optional: initial height (default: 3)
    }
})
```

**Result:** Resizable textarea (min: 60px, max: 300px)

**Use Cases:**
- Descriptions
- Notes
- Code snippets
- Long messages
- JSON/YAML input

---

## 6. Text Input Widget

**When:** Short text (default)

**Schema:**
```rust
json!({
    "name": {
        "type": "string",
        "description": "Node name",
        "default": ""
    }
})
```

**Result:** Single-line text input

**Use Cases:**
- Names
- Labels
- URLs
- File paths
- IDs

---

## 7. Number Input Widget

**When:** Numbers without constraints

**Schema:**
```rust
json!({
    "count": {
        "type": "number",
        "description": "Repeat count",
        "default": 1
    }
})
```

**Result:** Number input (no slider)

**Use Cases:**
- Counters
- Indices
- IDs
- Unlimited ranges

---

## 8. File Picker Widget

**When:** File selection with type filtering

**Schema:**
```rust
json!({
    "audioFile": {
        "type": "string",
        "format": "file",      // ← Triggers file picker
        "description": "Audio file",
        "default": "",
        "typeOptions": {
            "acceptedTypes": [".mp3", ".wav", ".ogg", ".flac"]
        }
    }
})
```

**Result:** File path input + Browse button with file type filter

**Use Cases:**
- Audio files
- Image files
- Document upload
- Configuration files
- Any file input

**Options:**
- `acceptedTypes`: Array of file extensions (e.g., `[".mp3", ".wav"]`)

---

## 9. Multi-Select Widget

**When:** Multiple selections from predefined options

**Schema:**
```rust
json!({
    "tags": {
        "type": "array",
        "description": "Tags",
        "items": {
            "type": "string",
            "enum": ["urgent", "important", "review", "done", "archived"]  // ← Triggers multi-select
        },
        "default": ["important"]
    }
})
```

**Result:** Dropdown with checkboxes + tag display with built-in search

**Use Cases:**
- Tags/labels
- Categories
- Multiple status selection
- Feature flags
- Permission selection

**Features:**
- Checkbox selection (stays open on click)
- Selected items shown as tags
- Built-in search filter
- Select All / Clear All buttons

---

## Advanced: Explicit Widget Override

Force a specific widget even if auto-detection would choose another:

```rust
json!({
    "threshold": {
        "type": "number",
        "ui:widget": "slider",  // ← Explicit override
        "minimum": 0,
        "maximum": 1,
        "default": 0.5
    }
})
```

**Available overrides:**
- `"ui:widget": "slider"`
- `"ui:widget": "color"`
- `"ui:widget": "toggle"`
- `"ui:widget": "select"`
- `"ui:widget": "textarea"`
- `"ui:widget": "text"`
- `"ui:widget": "number"`

---

## Advanced: Connection-Only Inputs (forceInput)

Some inputs should ONLY accept connections, not manual values. Use `forceInput: true` to hide the widget and show only the port.

**When to use:**
- Large data types (images, audio, objects) that don't make sense to type manually
- Data that must come from another node
- Enforcing workflow structure

**Schema:**
```rust
json!({
    "imageData": {
        "type": "object",
        "description": "Image data (must be connected)",
        "forceInput": true,  // ← No widget shown, port only
        "optional": true
    },
    "audioBuffer": {
        "type": "object",
        "description": "Audio buffer (must be connected)",
        "forceInput": true,
        "optional": false  // Required connection
    }
})
```

**Visual Result:**
- Port is shown on the left
- NO widget is rendered inside the node
- User MUST connect another node's output to provide the value

**Use Cases:**
- Image processing nodes (image input)
- Audio processors (audio buffer input)
- Data transformers (structured data input)
- Workflow orchestration (forcing specific connections)

---

## Advanced: Conditional Field Visibility (displayOptions)

Show or hide fields dynamically based on other field values. Based on n8n's `displayOptions`.

**When to use:**
- Configuration modes (show different options for different modes)
- Authentication methods (API key vs OAuth)
- Conditional settings (advanced options only when enabled)

**Schema:**
```rust
json!({
    "authentication": {
        "type": "string",
        "description": "Authentication method",
        "enum": ["none", "apiKey", "oauth"],
        "default": "none"
    },
    "apiKey": {
        "type": "string",
        "description": "API Key",
        "default": "",
        "optional": true,
        "displayOptions": {
            "show": {  // ← Show this field when...
                "authentication": ["apiKey"]  // ← ...authentication is "apiKey"
            }
        }
    },
    "oauthClientId": {
        "type": "string",
        "description": "OAuth Client ID",
        "default": "",
        "optional": true,
        "displayOptions": {
            "show": {
                "authentication": ["oauth"]  // ← Only show when auth is "oauth"
            }
        }
    }
})
```

**Display Options:**
- `show`: Field is visible when conditions match
- `hide`: Field is hidden when conditions match

**Multiple Conditions (AND logic):**
```rust
"displayOptions": {
    "show": {
        "mode": ["advanced"],  // ← AND
        "enabled": [true]      // ← Both must match
    }
}
```

**Multiple Values (OR logic):**
```rust
"displayOptions": {
    "show": {
        "authentication": ["apiKey", "oauth"]  // ← Show if EITHER matches
    }
}
```

**Use Cases:**
- Authentication configuration (different fields per method)
- Mode-specific settings (normal vs advanced)
- Conditional features (only show when enabled)
- Clean node UIs (hide irrelevant options)

---

## Complete Example: All Widgets with Optional/Required

```rust
pub struct DemoNode;

#[async_trait]
impl FlowNode for DemoNode {
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                // SLIDER (Required)
                "opacity": {
                    "type": "number",
                    "description": "Opacity level (required)",
                    "minimum": 0,
                    "maximum": 100,
                    "default": 50
                },

                // COLOR PICKER (Optional)
                "backgroundColor": {
                    "type": "string",
                    "format": "color",
                    "description": "Background color (optional)",
                    "default": "#3B82F6",
                    "optional": true
                },

                // TOGGLE (Required)
                "enabled": {
                    "type": "boolean",
                    "description": "Enable/disable (required)",
                    "default": true
                },

                // DROPDOWN (Required)
                "mode": {
                    "type": "string",
                    "description": "Mode selection (required)",
                    "enum": ["normal", "advanced", "expert"],
                    "default": "normal"
                },

                // TEXTAREA (Optional)
                "notes": {
                    "type": "string",
                    "description": "Notes (optional)",
                    "maxLength": 500,
                    "default": "Enter your notes here...",
                    "optional": true
                },

                // FILE PICKER (Optional)
                "audioFile": {
                    "type": "string",
                    "format": "file",
                    "description": "Audio file (optional)",
                    "default": "",
                    "optional": true,
                    "typeOptions": {
                        "acceptedTypes": [".mp3", ".wav", ".ogg", ".flac"]
                    }
                },

                // MULTI-SELECT (Optional)
                "tags": {
                    "type": "array",
                    "description": "Tags (optional)",
                    "optional": true,
                    "items": {
                        "type": "string",
                        "enum": ["urgent", "important", "review", "done", "archived"]
                    },
                    "default": ["important"]
                }
            }
        })
    }

    async fn execute(&self, params: Value) -> Result<Value, String> {
        // Required fields - always present
        let opacity = params["opacity"].as_f64().unwrap_or(50.0);
        let enabled = params["enabled"].as_bool().unwrap_or(true);
        let mode = params["mode"].as_str().unwrap_or("normal");

        // Optional fields - use .get() for safer access
        let bg_color = params.get("backgroundColor")
            .and_then(|v| v.as_str())
            .unwrap_or("#3B82F6");
        let notes = params.get("notes")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let audio_file = params.get("audioFile")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        Ok(json!({
            "success": true,
            "opacity": opacity,
            "mode": mode
        }))
    }
}
```

---

## Advanced: Type Color Coding

VortiDeck uses a visual type system with color-coded ports and connections, inspired by ComfyUI. Each data type gets a distinct color for better visual clarity when connecting nodes.

### Available Type Colors

| Type | Color | Hex | Use Case |
|------|-------|-----|----------|
| `string` | Green | `#4CAF50` | Text, names, URLs |
| `number` | Blue | `#2196F3` | Numbers, counts, IDs |
| `boolean` | Orange | `#FF9800` | True/false values |
| `array` | Purple | `#9C27B0` | Lists, collections |
| `object` | Grey | `#607D8B` | Complex data structures |
| `audio` | Amber | `#F59E0B` | Audio buffers, files |
| `image` | Violet | `#8B5CF6` | Image data |
| `video` | Pink | `#EC4899` | Video data |
| `text` | Emerald | `#10B981` | Long text (similar to string) |
| `json` | Indigo | `#6366F1` | JSON data |
| `any` | Slate | `#94A3B8` | Universal type |
| `flow` | Red | `#EF4444` | Flow control |

### How to Use

Define output types in your node's `outputs_schema()`:

```rust
pub struct MyNode;

#[async_trait]
impl FlowNode for MyNode {
    fn outputs_schema(&self) -> Vec<crate::services::flow::traits::OutputDefinition> {
        vec![
            OutputDefinition {
                id: "text".to_string(),
                name: "Text Output".to_string(),
                output_type: "string".to_string(),  // ← Green port
                description: Some("Processed text".to_string()),
            },
            OutputDefinition {
                id: "count".to_string(),
                name: "Count".to_string(),
                output_type: "number".to_string(),  // ← Blue port
                description: Some("Number of items".to_string()),
            },
            OutputDefinition {
                id: "data".to_string(),
                name: "Data".to_string(),
                output_type: "object".to_string(),  // ← Grey port
                description: Some("Structured data".to_string()),
            },
        ]
    }

    // ... rest of implementation
}
```

### Visual Behavior

- **Ports**: Display a colored ring based on the data type
- **Connections**: Line color matches the source port type
- **Type Mismatches**: Visually obvious when connecting incompatible types

---

## Advanced: Dynamic Type Inference (Variable Nodes)

VortiDeck features an intelligent variable system where **Set Variable** and **Get Variable** nodes automatically infer and propagate types based on connections.

### How It Works

1. **Set Variable** node receives data from any output
2. System detects the source output type (e.g., `string`, `number`)
3. Set Variable's output type updates to match the input type
4. All **Get Variable** nodes with matching variable names update their output types
5. Type changes propagate through the entire flow automatically

### Example Flow

```
[Text Node] --string--> [Set Variable: "message"] --string--> [Get Variable: "message"] --string--> [Log Node]
                              ↓
                        (Type: any → string)
                              ↓
                    [Get Variable: "message"]
                              ↓
                        (Type: any → string)
```

**Before Connection:**
- Set Variable output: `any` (grey port)
- Get Variable output: `any` (grey port)

**After Connection:**
- Set Variable output: `string` (green port)
- Get Variable output: `string` (green port)

### Implementation Details

The type synchronization is **event-driven** and highly performant:

- Uses Map-based caching for O(1) lookups
- Only updates directly affected nodes
- No deep watchers on the entire node graph
- Triggers on:
  - Connection creation/removal
  - Variable name changes
  - Node addition/removal

### Building Variable-Aware Nodes

No special implementation needed! The system automatically handles type propagation. Just ensure your nodes:

1. Declare proper output types in `outputs_schema()`
2. Use standard VortiDeck connection system
3. Types will propagate automatically

---

## MCP Tools (Same System!)

MCP tools use the same schema format:

```json
{
  "name": "my_tool",
  "description": "My custom tool",
  "inputSchema": {
    "type": "object",
    "properties": {
      "volume": {
        "type": "number",
        "minimum": 0,
        "maximum": 100,
        "default": 50
      },
      "color": {
        "type": "string",
        "format": "color",
        "default": "#3B82F6"
      }
    }
  }
}
```

---

## Best Practices

### ✅ DO:
- Use descriptive descriptions
- Provide sensible defaults for all inputs
- Add min/max for sliders
- Use enums for fixed options
- Use color format for colors
- **Declare proper output types** in `outputs_schema()` for type color coding
- **Use semantic type names** (`string`, `number`, `audio`, etc.) for visual clarity
- Mark non-critical inputs as `optional: true`
- Use `.get()` when accessing optional fields in execute()
- Include helpful hints in descriptions (e.g., "optional", "required")
- Use `forceInput: true` for large data types (images, audio, complex objects)
- Use `displayOptions` to keep node UIs clean and context-aware
- Test displayOptions with all possible combinations

### ❌ DON'T:
- Mix slider (min/max) with unrestricted numbers
- Use textarea for short text
- Forget default values (especially for optional fields)
- Use vague descriptions
- Mark critical inputs as optional
- Access optional fields directly with `params["field"]` (use `params.get("field")` instead)
- Overuse `forceInput` - only use when manual input truly doesn't make sense
- Create complex displayOptions chains - keep visibility logic simple
- Forget to mark conditionally-shown fields as optional

---

## Widget Inference Logic

```
1. Explicit `ui:widget` specified? → Use it
2. `format: "color"`? → Color picker
3. `format: "file"`? → File picker
4. `type: "array"` + items.enum? → Multi-select dropdown
5. Has `enum` array? → Dropdown
6. `type: "boolean"`? → Toggle
7. `type: "number"` + min/max? → Slider
8. `type: "number"`? → Number input
9. `type: "string"` + maxLength > 100? → Textarea
10. `type: "string"`? → Text input
11. Default → Text input
```

See: [widgetInference.ts](../../layers/flow/utils/widgetInference.ts)

---

## Future Widgets (From Research)

Based on ComfyUI/n8n research, planned features:

### Range Slider (Min/Max Pair)
```rust
json!({
    "range": {
        "type": "object",
        "ui:widget": "range",
        "properties": {
            "min": { "type": "number" },
            "max": { "type": "number" }
        }
    }
})
```

### Date/Time Picker
```rust
json!({
    "scheduledTime": {
        "type": "string",
        "format": "date-time",
        "ui:widget": "datetime",
        "default": "2025-01-01T00:00:00Z"
    }
})
```

### Code Editor Widget
```rust
json!({
    "code": {
        "type": "string",
        "ui:widget": "code-editor",
        "language": "javascript",
        "default": "// Enter code here"
    }
})
```

---

## Testing Your Node

1. Create your node with schema
2. Register it in the provider
3. Restart app
4. Open Flow Editor
5. Add your node to canvas
6. Verify all widgets render correctly

---

## Need Help?

- Example node: [WidgetDemoNode](../../apps/tauri-windows/libs/vortideck-core/src/services/flow/standard_nodes.rs#L694-L780)
- Widget inference: [widgetInference.ts](../../layers/flow/utils/widgetInference.ts)
- Research notes: research-comfyui-n8n-node-systems.md
- Implementation guide: widget-system-implementation.md
