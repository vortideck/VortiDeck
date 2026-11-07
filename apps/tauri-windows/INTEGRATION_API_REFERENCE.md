# Universal Integration API Reference

## Frontend Usage Examples (TypeScript)

The universal integration system provides a clean, consistent API with automatic camelCase conversion.

### Basic Usage

```typescript
import { invoke } from '@tauri-apps/api/tauri'

// Get all integrations status
const integrations = await invoke('getIntegrations')
// Returns: { obs: { id: "obs", status: "Connected", ... }, discord: { id: "discord", status: "Disconnected", ... } }

// Check if specific integration is connected
const obsConnected = await invoke('isConnected', { integrationId: 'obs' })
const discordConnected = await invoke('isConnected', { integrationId: 'discord' })

// Execute action on any integration
await invoke('executeAction', {
  integrationId: 'discord',
  actionName: 'sendNotification',
  parameters: {
    title: 'Stream Started!',
    description: 'Your VortiDeck stream is now live',
    color: 0x00ff00
  }
})

// Get available actions for integration
const actions = await invoke('getActions', { integrationId: 'discord' })
// Returns: [{ name: "sendMessage", description: "...", parameters: [...] }, ...]
```

### Convenience Commands (Backward Compatibility)

```typescript
// OBS convenience commands
const obsStatus = await invoke('getObsStatus') // Returns boolean
const obsActions = await invoke('getObsActions')
await invoke('executeObsAction', { actionName: 'startStreaming', parameters: {} })

// Discord convenience commands  
const discordStatus = await invoke('getDiscordStatus') // Returns boolean
const discordActions = await invoke('getDiscordActions')
await invoke('executeDiscordAction', { 
  actionName: 'sendAlert', 
  parameters: { 
    message: 'Recording started!', 
    severity: 'success' 
  } 
})
```

### Vue Store Integration

```typescript
// stores/integrations.ts
export const useIntegrationsStore = defineStore('integrations', {
  state: () => ({
    integrations: new Map<string, IntegrationStatus>(),
    loading: false
  }),
  
  getters: {
    obsConnected: (state) => state.integrations.get('obs')?.status === 'Connected',
    discordConnected: (state) => state.integrations.get('discord')?.status === 'Connected',
    connectedCount: (state) => Array.from(state.integrations.values())
      .filter(integration => integration.status === 'Connected').length
  },
  
  actions: {
    async initializeListener() {
      await listen('integrationStatusChanged', (event: any) => {
        const { integrationId, status } = event.payload
        this.integrations.set(integrationId, status)
      })
    },
    
    async loadIntegrations() {
      this.loading = true
      try {
        const integrations = await invoke('getIntegrations')
        this.integrations = new Map(Object.entries(integrations))
      } finally {
        this.loading = false
      }
    },
    
    async executeAction(integrationId: string, actionName: string, parameters: any = {}) {
      await invoke('executeAction', {
        integrationId,
        actionName, 
        parameters
      })
    },
    
    async refreshStatuses() {
      await invoke('refreshStatuses')
    }
  }
})
```

### Dashboard Component Example

```vue
<template>
  <div class="integrations-dashboard">
    <h2>Integrations Status</h2>
    
    <div class="summary">
      <div class="stat">
        <span class="label">Connected:</span>
        <span class="value">{{ connectedCount }} / {{ totalCount }}</span>
      </div>
      <div class="stat">
        <span class="label">Connection Rate:</span>
        <span class="value">{{ connectionRate }}%</span>
      </div>
    </div>
    
    <div class="integration-list">
      <div 
        v-for="[id, integration] in integrations" 
        :key="id"
        class="integration-card"
        :class="{ connected: integration.status === 'Connected' }"
      >
        <div class="integration-info">
          <h3>{{ integration.name }}</h3>
          <span class="status" :class="integration.status.toLowerCase()">
            {{ integration.status }}
          </span>
        </div>
        
        <div class="integration-actions">
          <button 
            v-if="integration.status === 'Connected'"
            @click="quickAction(id)"
            class="btn-primary"
          >
            Quick Action
          </button>
          <button @click="openSettings(id)" class="btn-secondary">
            Settings
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useIntegrationsStore } from '@/stores/integrations'

const integrationsStore = useIntegrationsStore()

const { integrations, connectedCount } = storeToRefs(integrationsStore)

const totalCount = computed(() => integrations.value.size)
const connectionRate = computed(() => 
  totalCount.value > 0 ? Math.round((connectedCount.value / totalCount.value) * 100) : 0
)

async function quickAction(integrationId: string) {
  if (integrationId === 'obs') {
    await integrationsStore.executeAction('obs', 'start_streaming')
  } else if (integrationId === 'discord') {
    await integrationsStore.executeAction('discord', 'send_notification', {
      title: 'VortiDeck Alert',
      description: 'Quick action executed!',
      color: 0x0099ff
    })
  }
}

function openSettings(integrationId: string) {
  // Navigate to integration settings
  router.push(`/settings/integrations/${integrationId}`)
}

onMounted(async () => {
  await integrationsStore.initializeListener()
  await integrationsStore.loadIntegrations()
})
</script>
```

## API Reference

### Universal Commands

| Command | Frontend Call | Description |
|---------|---------------|-------------|
| `get_integration_status` | `getIntegrationStatus({ integrationId })` | Get specific integration status |
| `get_integrations` | `getIntegrations()` | Get all integration statuses |
| `execute_action` | `executeAction({ integrationId, actionName, parameters })` | Execute integration action |
| `get_actions` | `getActions({ integrationId })` | Get available actions |
| `is_connected` | `isConnected({ integrationId })` | Check connection status |
| `get_connected` | `getConnected()` | Get connected integrations |
| `refresh_statuses` | `refreshStatuses()` | Refresh all statuses |
| `get_summary` | `getSummary()` | Get dashboard summary |

### Convenience Commands

| Command | Frontend Call | Description |
|---------|---------------|-------------|
| `get_obs_status` | `getObsStatus()` | Get OBS connection status |
| `get_discord_status` | `getDiscordStatus()` | Get Discord connection status |
| `execute_obs_action` | `executeObsAction({ actionName, parameters })` | Execute OBS action |
| `execute_discord_action` | `executeDiscordAction({ actionName, parameters })` | Execute Discord action |
| `get_obs_actions` | `getObsActions()` | Get OBS actions |
| `get_discord_actions` | `getDiscordActions()` | Get Discord actions |

### Event Listeners

```typescript
// Listen for integration status changes
await listen('integrationStatusChanged', (event) => {
  const { integrationId, status, timestamp } = event.payload
  // Handle status change
})

// Listen for new integration registration
await listen('integrationRegistered', (event) => {
  const { integrationId, status } = event.payload
  // Handle new integration
})
```

## Types

```typescript
interface IntegrationStatus {
  id: string
  name: string
  status: 'Connected' | 'Disconnected' | 'Connecting' | 'Error'
  lastUpdated: number
  metadata?: IntegrationMetadata
  actionsAvailable: number
  errorMessage?: string
}

interface IntegrationMetadata {
  id: string
  name: string
  version: string
  description: string
  author: string
  website?: string
}

interface ActionDefinition {
  name: string
  description: string
  parameters: ParameterDefinition[]
}

interface ParameterDefinition {
  name: string
  description: string
  required: boolean
  defaultValue?: string
}
```

## Key Benefits

1. **Automatic Case Conversion**: Rust snake_case automatically converts to frontend camelCase
2. **Simple Function Names**: `getIntegrations()` instead of `getAllIntegrationStatuses()`
3. **Universal API**: Same pattern works for OBS, Discord, and any future integration
4. **Type Safety**: Full TypeScript support with proper interfaces
5. **Backward Compatibility**: Convenience commands maintain existing OBS workflows
6. **Real-time Updates**: Event-driven status updates with no polling needed