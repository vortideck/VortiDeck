# AI Assistant Architecture

## Overview
The AI Assistant is a context-aware helper that opens as a separate Tauri window with multi-provider support and persistent conversation history.

## Key Features

### 1. Multi-Provider Support
Supports multiple AI providers with easy switching:

```rust
pub enum AIProvider {
    OpenAI { api_key: String, base_url: Option<String> },
    Anthropic { api_key: String, base_url: Option<String> },
    AzureOpenAI { api_key: String, endpoint: String, deployment: String },
    Custom { name: String, base_url: String, api_key: Option<String> },
}
```

**Available Providers:**
- **OpenAI**: GPT-4, GPT-4 Turbo, GPT-3.5 Turbo
- **Anthropic**: Claude 3.5 Sonnet, Claude 3 Opus, Claude 3 Sonnet, Claude 3 Haiku
- **Azure OpenAI**: Azure-hosted models
- **Custom**: Local providers (Ollama, LM Studio, etc.)

### 2. Conversation History

**Two Levels of History:**

#### Per-Window History
Each assistant window maintains its own conversation for context-specific chats:
```rust
pub struct AssistantContext {
    pub conversation_history: Vec<ConversationMessage>,
    // ... other fields
}
```

#### Global History
Shared across all assistant windows for continuity:
```rust
type GlobalConversationHistory = Arc<Mutex<Vec<ConversationMessage>>>;
```

**Benefits:**
- ✅ Context continuity across multiple windows
- ✅ Learn from previous interactions
- ✅ Avoid repeating information
- ✅ Build on previous suggestions

### 3. Provider Selection UI

Users can switch providers and models per window:

```vue
<template>
  <div class="provider-selector">
    <select v-model="selectedProvider">
      <option value="openai">OpenAI</option>
      <option value="anthropic">Anthropic Claude</option>
      <option value="azure">Azure OpenAI</option>
      <option value="custom">Custom</option>
    </select>

    <select v-model="selectedModel">
      <option v-for="model in availableModels" :key="model.id">
        {{ model.name }}
      </option>
    </select>
  </div>
</template>
```

### 4. Integration with Flow Builder

The assistant uses the same AI configuration system as Flow Builder:

**Flow Builder:**
```rust
pub struct AiNodeConfig {
    pub model: Option<String>,
    pub behavior: AiNodeBehavior,
    // ...
}
```

**Assistant:**
```rust
pub struct AssistantAIConfig {
    pub provider: AIProvider,
    pub model: String,
    pub temperature: Option<f32>,
    // ...
}
```

**Shared Benefits:**
- ✅ Consistent model selection
- ✅ Same provider support
- ✅ Unified configuration
- ✅ Easy to add new providers

## Architecture Diagram

```
┌──────────────────────────────────────────────────────────┐
│                   User Interface                         │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────┐ │
│  │  Provider   │  │ Model Select │  │ History Toggle │ │
│  │  Dropdown   │  │   Dropdown   │  │    Button      │ │
│  └─────────────┘  └──────────────┘  └────────────────┘ │
└──────────────────────────────────────────────────────────┘
                           ↓
┌──────────────────────────────────────────────────────────┐
│                 Assistant Window State                   │
│  ┌────────────────────────────────────────────────────┐ │
│  │  AssistantContext                                  │ │
│  │  - provider: AIProvider                            │ │
│  │  - model: String                                   │ │
│  │  - conversation_history: Vec<Message>             │ │
│  │  - context: String                                 │ │
│  └────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────┘
                           ↓
┌──────────────────────────────────────────────────────────┐
│              Global Conversation Store                   │
│  All messages from all windows (chronological)          │
│  ┌────────────────────────────────────────────────────┐ │
│  │  [msg1, msg2, msg3, ...]                          │ │
│  └────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────┘
                           ↓
┌──────────────────────────────────────────────────────────┐
│                  Provider Selection                      │
│                                                          │
│  if OpenAI → OpenAIClient                               │
│  if Anthropic → AnthropicClient (new)                   │
│  if Azure → AzureClient                                 │
│  if Custom → CustomClient                               │
└──────────────────────────────────────────────────────────┘
                           ↓
┌──────────────────────────────────────────────────────────┐
│                    AI Provider API                       │
│  - OpenAI API                                           │
│  - Anthropic API                                        │
│  - Azure OpenAI API                                     │
│  - Custom/Local API                                     │
└──────────────────────────────────────────────────────────┘
```

## Usage Examples

### Example 1: OpenAI with History
```typescript
const assistant = useAIAssistant({
  context: 'command_input',
  targetInput: commandInput,
  aiConfig: {
    provider: 'openai',
    model: 'gpt-4',
  },
  useGlobalHistory: true, // Include all previous conversations
});
```

### Example 2: Claude for Complex Analysis
```typescript
const assistant = useAIAssistant({
  context: 'script_editor',
  targetInput: scriptEditor,
  aiConfig: {
    provider: 'anthropic',
    model: 'claude-3-5-sonnet-20241022',
  },
  useGlobalHistory: true,
});
```

### Example 3: Fast GPT-3.5 for Simple Tasks
```typescript
const assistant = useAIAssistant({
  context: 'program_args',
  targetInput: argsInput,
  aiConfig: {
    provider: 'openai',
    model: 'gpt-3.5-turbo',
  },
  useGlobalHistory: false, // Fresh conversation
});
```

## Backend Commands

### Get Available Providers
```rust
#[command]
pub async fn get_ai_providers() -> Result<Vec<ProviderInfo>, String>
```

### Get Models for Provider
```rust
#[command]
pub async fn get_provider_models(provider: String) -> Result<Vec<ModelInfo>, String>
```

### Switch Provider
```rust
#[command]
pub async fn switch_ai_provider(
    window_label: String,
    provider: AIProvider,
    model: String,
) -> Result<(), String>
```

### Get Global History
```rust
#[command]
pub async fn get_global_conversation_history(
    limit: Option<usize>
) -> Result<Vec<ConversationMessage>, String>
```

### Clear History
```rust
#[command]
pub async fn clear_conversation_history(
    scope: HistoryScope // Global, Window, or All
) -> Result<(), String>
```

## Configuration

### Settings UI
```vue
<template>
  <div class="assistant-settings">
    <h3>Default AI Provider</h3>
    <select v-model="defaultProvider">
      <option value="openai">OpenAI</option>
      <option value="anthropic">Anthropic Claude</option>
    </select>

    <h3>API Keys</h3>
    <input v-model="openaiKey" placeholder="OpenAI API Key" type="password" />
    <input v-model="anthropicKey" placeholder="Anthropic API Key" type="password" />

    <h3>History Settings</h3>
    <checkbox v-model="enableGlobalHistory">
      Enable global conversation history
    </checkbox>
    <number v-model="historyLimit">
      Max history messages: {{ historyLimit }}
    </number>
  </div>
</template>
```

## Benefits

### For Users:
- ✅ **Choice**: Use preferred AI provider
- ✅ **Cost**: Mix expensive/cheap models based on task
- ✅ **Privacy**: Use local providers for sensitive data
- ✅ **Continuity**: Conversation history across windows
- ✅ **Context**: AI remembers previous interactions

### For Developers:
- ✅ **Extensible**: Easy to add new providers
- ✅ **Consistent**: Same patterns as Flow Builder
- ✅ **Type-safe**: Rust enums for providers
- ✅ **Testable**: Mock providers for testing

## Future Enhancements

1. **Conversation Export**: Save history to file
2. **Conversation Search**: Find past interactions
3. **Provider Fallback**: Auto-switch on failure
4. **Cost Tracking**: Monitor API usage per provider
5. **Custom Prompts**: User-defined system prompts per provider
6. **Conversation Branches**: Fork conversations for experimentation
