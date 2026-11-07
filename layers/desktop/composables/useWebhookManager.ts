import { invoke } from '@tauri-apps/api/core'
import { ref, computed } from 'vue'

// 🎯 Webhook Types (aligned with backend Webhook model)
export interface WebhookConfig {
  // Core fields
  id: string
  name: string
  description?: string
  service: string // "github", "slack", "discord", "custom" - maps to service_type
  endpoint_url: string
  target_url?: string
  enabled: boolean
  created_at: string
  updated_at: string

  // Secrets
  inbound_secret?: string // External service secret (GitHub, Twitch, etc.)
  outbound_secret?: string // VortiDeck secret for signing outbound webhooks

  // Phase 1-3 features
  test_url?: string
  production_url?: string
  is_test_active: boolean
  test_expires_at?: string
  response_mode: string // "onReceived" | "lastNode" | "responseNode" | "streaming"
  auth_config?: string // JSON string
  allowed_methods: string // JSON array: ["GET", "POST"]

  // External service integration
  subscription_id?: string

  // Action (UniversalAction JSON - can be FlowId, MacroId, etc.)
  action?: string

  // Deprecated fields (kept for backwards compatibility)
  endpoint?: string // Use endpoint_url instead
  secret?: string // Use inbound_secret/outbound_secret instead
  service_type?: string // Use service instead
  trigger_count?: number // Not in database - remove eventually
  last_triggered?: string | null // Not in database - remove eventually
  requireSignature?: boolean
  rateLimit?: number
  timeout?: number
  maxRetries?: number
  backoffStrategy?: string
  baseDelay?: number
  logs?: WebhookLog[]
}

export interface WebhookLog {
  id: string | number
  timestamp: string
  method: string
  status: number
  responseTime: number
  payload: string
  response: unknown
}

export type WebhookSubmission = Omit<Partial<WebhookConfig>, 'auth_config' | 'action'>
  & Pick<WebhookConfig, 'name' | 'service'>
  & { auth_config?: string | null; action?: string | null }

export type WebhookUpdate = Omit<Partial<WebhookConfig>, 'auth_config' | 'action'>
  & { auth_config?: string | null; action?: string | null }

export interface WebhookTestResult {
  success: boolean
  responseTime: number
  response: unknown
}

export interface ActiveWebhook {
  id: string
  name: string
  endpoint: string
  status: 'active' | 'paused' | 'error'
  created_at: string
  trigger_count: number
  last_triggered?: string
}

export interface PausedFlow {
  flow_id: string
  flow_name: string
  webhook_id: string
  webhook_name: string
  paused_at: string
  paused_node_id: string
  timeout_at?: string
}

export interface WebhookTemplate {
  id: string
  name: string
  service_type: string
  description: string
  endpoint_suffix: string
  requires_secret: boolean
  icon?: string
}

// 🎯 Webhook Manager Composable
export const useWebhookManager = () => {
  const loading = ref(false)
  const error = ref<string | null>(null)
  const webhooks = ref<WebhookConfig[]>([])
  const activeWebhooks = ref<ActiveWebhook[]>([])
  const pausedFlows = ref<PausedFlow[]>([])
  const templates = ref<WebhookTemplate[]>([])

  /**
   * Fetch all webhooks
   */
  const fetchWebhooks = async (): Promise<void> => {
    loading.value = true
    error.value = null

    try {
      console.log('🎯 Fetching webhooks')

      const response: WebhookConfig[] = await invoke('get_webhooks')
      webhooks.value = response

      console.log('✅ Webhooks fetched:', response.length)
    } catch (err) {
      console.error('❌ Error fetching webhooks:', err)
      error.value = err instanceof Error ? err.message : 'Failed to fetch webhooks'
    } finally {
      loading.value = false
    }
  }

  /**
   * Fetch active webhooks
   */
  const fetchActiveWebhooks = async (): Promise<void> => {
    try {
      console.log('🎯 Fetching active webhooks')
      
      const response: ActiveWebhook[] = await invoke('get_active_webhooks')
      activeWebhooks.value = response
      
      console.log('✅ Active webhooks fetched:', response.length)
    } catch (err) {
      console.error('❌ Error fetching active webhooks:', err)
      error.value = err instanceof Error ? err.message : 'Failed to fetch active webhooks'
    }
  }

  /**
   * Fetch paused flows
   */
  const fetchPausedFlows = async (): Promise<void> => {
    try {
      console.log('🎯 Fetching paused flows')
      
      const response: PausedFlow[] = await invoke('get_paused_flows')
      pausedFlows.value = response
      
      console.log('✅ Paused flows fetched:', response.length)
    } catch (err) {
      console.error('❌ Error fetching paused flows:', err)
      error.value = err instanceof Error ? err.message : 'Failed to fetch paused flows'
    }
  }

  /**
   * Fetch webhook templates
   */
  const fetchTemplates = async (): Promise<void> => {
    try {
      console.log('🎯 Fetching webhook templates')
      
      const response: WebhookTemplate[] = await invoke('get_webhook_templates')
      templates.value = response
      
      console.log('✅ Webhook templates fetched:', response.length)
    } catch (err) {
      console.error('❌ Error fetching webhook templates:', err)
      error.value = err instanceof Error ? err.message : 'Failed to fetch webhook templates'
    }
  }

  /**
   * Create a new webhook
   */
  const createWebhook = async (config: WebhookSubmission): Promise<WebhookConfig> => {
    loading.value = true
    error.value = null

    try {
      console.log('🎯 Creating webhook:', config)

      // Ensure allowed_methods is a JSON string (handle both string and array)
      const allowedMethodsJson = (() => {
        if (!config.allowed_methods) return null
        if (typeof config.allowed_methods === 'string') return config.allowed_methods
        if (Array.isArray(config.allowed_methods)) return JSON.stringify(config.allowed_methods)
        return null
      })()

      const response: WebhookConfig = await invoke('create_webhook', {
        name: config.name,
        description: config.description || null,
        service: config.service || 'custom',
        targetUrl: config.target_url || null,
        action: config.action || null,
        responseMode: config.response_mode || null,
        authConfig: config.auth_config || null,
        allowedMethods: allowedMethodsJson
      })

      // Add to local state
      webhooks.value.push(response)

      console.log('✅ Webhook created:', response)
      return response
    } catch (err) {
      console.error('❌ Error creating webhook:', err)
      error.value = err instanceof Error ? err.message : 'Failed to create webhook'
      throw err
    } finally {
      loading.value = false
    }
  }

  /**
   * Update webhook
   */
  const updateWebhook = async (id: string, updates: WebhookUpdate): Promise<WebhookConfig> => {
    loading.value = true
    error.value = null

    try {
      console.log('🎯 Updating webhook:', id, updates)

      const response: WebhookConfig = await invoke('update_webhook', {
        webhookId: id,
        name: updates.name || null,
        description: updates.description || null,
        service: updates.service || null,
        targetUrl: updates.target_url || null,
        inboundSecret: updates.inbound_secret || null,
        action: updates.action || null,
        enabled: updates.enabled !== undefined ? updates.enabled : null,
        subscriptionId: updates.subscription_id || null
      })

      // Update local state
      const index = webhooks.value.findIndex(w => w.id === id)
      if (index !== -1) {
        webhooks.value[index] = response
      }

      console.log('✅ Webhook updated:', response)
      return response
    } catch (err) {
      console.error('❌ Error updating webhook:', err)
      error.value = err instanceof Error ? err.message : 'Failed to update webhook'
      throw err
    } finally {
      loading.value = false
    }
  }

  /**
   * Delete webhook
   */
  const deleteWebhook = async (id: string): Promise<void> => {
    loading.value = true
    error.value = null

    try {
      console.log('🎯 Deleting webhook:', id)

      await invoke('delete_webhook', { webhookId: id })

      // Remove from local state
      webhooks.value = webhooks.value.filter(w => w.id !== id)
      activeWebhooks.value = activeWebhooks.value.filter(w => w.id !== id)

      console.log('✅ Webhook deleted:', id)
    } catch (err) {
      console.error('❌ Error deleting webhook:', err)
      error.value = err instanceof Error ? err.message : 'Failed to delete webhook'
      throw err
    } finally {
      loading.value = false
    }
  }

  /**
   * Get webhook statistics
   */
  const getStatistics = async (): Promise<any> => {
    try {
      console.log('🎯 Getting webhook statistics')

      const response = await invoke('get_webhook_statistics')

      console.log('✅ Statistics fetched:', response)
      return response
    } catch (err) {
      console.error('❌ Error fetching statistics:', err)
      error.value = err instanceof Error ? err.message : 'Failed to fetch statistics'
      throw err
    }
  }

  /**
   * Resume a paused flow
   */
  const resumeFlow = async (flowId: string): Promise<void> => {
    try {
      console.log('🎯 Resuming flow:', flowId)
      
      await invoke('resume_flow', { flowId })
      
      // Remove from paused flows
      pausedFlows.value = pausedFlows.value.filter(f => f.flow_id !== flowId)
      
      console.log('✅ Flow resumed:', flowId)
    } catch (err) {
      console.error('❌ Error resuming flow:', err)
      error.value = err instanceof Error ? err.message : 'Failed to resume flow'
      throw err
    }
  }

  /**
   * Cancel a paused flow
   */
  const cancelFlow = async (flowId: string): Promise<void> => {
    try {
      console.log('🎯 Canceling flow:', flowId)
      
      await invoke('cancel_flow', { flowId })
      
      // Remove from paused flows
      pausedFlows.value = pausedFlows.value.filter(f => f.flow_id !== flowId)
      
      console.log('✅ Flow canceled:', flowId)
    } catch (err) {
      console.error('❌ Error canceling flow:', err)
      error.value = err instanceof Error ? err.message : 'Failed to cancel flow'
      throw err
    }
  }

  /**
   * Test webhook endpoint
   */
  const testWebhook = async (id: string): Promise<void> => {
    try {
      console.log('🎯 Testing webhook:', id)
      
      await invoke('test_webhook', { id })
      
      console.log('✅ Webhook test sent:', id)
    } catch (err) {
      console.error('❌ Error testing webhook:', err)
      error.value = err instanceof Error ? err.message : 'Failed to test webhook'
      throw err
    }
  }

  /**
   * Get webhook logs
   */
  const getWebhookLogs = async (id: string, limit = 50): Promise<WebhookLog[]> => {
    try {
      console.log('🎯 Getting webhook logs:', id)
      
      const response = await invoke<WebhookLog[]>('get_webhook_logs', { id, limit })
      
      console.log('✅ Webhook logs fetched:', response.length)
      return response
    } catch (err) {
      console.error('❌ Error fetching webhook logs:', err)
      error.value = err instanceof Error ? err.message : 'Failed to fetch webhook logs'
      throw err
    }
  }

  // Computed properties
  const enabledWebhooks = computed(() => webhooks.value.filter(w => w.enabled))
  const disabledWebhooks = computed(() => webhooks.value.filter(w => !w.enabled))
  const webhookCount = computed(() => webhooks.value.length)
  const activeWebhookCount = computed(() => activeWebhooks.value.length)
  const pausedFlowCount = computed(() => pausedFlows.value.length)

  /**
   * Initialize webhook manager
   */
  const initialize = async (): Promise<void> => {
    await Promise.all([
      fetchWebhooks(),
      fetchActiveWebhooks(),
      fetchPausedFlows(),
      fetchTemplates()
    ])
  }

  return {
    // State
    loading,
    error,
    webhooks,
    activeWebhooks,
    pausedFlows,
    templates,

    // Computed
    enabledWebhooks,
    disabledWebhooks,
    webhookCount,
    activeWebhookCount,
    pausedFlowCount,

    // Methods
    initialize,
    fetchWebhooks,
    fetchActiveWebhooks,
    fetchPausedFlows,
    fetchTemplates,
    createWebhook,
    updateWebhook,
    deleteWebhook,
    getStatistics,
    resumeFlow,
    cancelFlow,
    testWebhook,
    getWebhookLogs
  }
}
