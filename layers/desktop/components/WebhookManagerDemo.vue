<template lang="pug">
div(class="flex flex-col gap-y-4")
  // Header
  div(class="flex items-center justify-between")
    div(class="flex items-center gap-x-2")
      MdiWebhook(class="w-5 h-5")
      h3(class="text-lg font-semibold") Webhook Manager
      span(class="text-xs bg-blue-100 text-blue-800 px-2 py-1 rounded") DEMO
    div(class="flex items-center gap-x-2")
      button(@click="refresh" :disabled="loading" class="p-1 rounded hover:bg-themeBackground3 transition-colors")
        MdiRefresh(class="w-4 h-4" :class="loading ? 'animate-spin' : ''")
      button(@click="showCreateModal = true" class="bg-black text-white px-3 py-1 rounded text-sm hover:bg-gray-800 transition-colors")
        MdiPlus(class="w-4 h-4 mr-1")
        span Create

  // Status Overview
  div(class="grid grid-cols-3 gap-2 text-sm")
    div(class="bg-themeBackground3 rounded p-2 text-center")
      div(class="text-lg font-bold text-green-600") {{mockData.activeWebhooks.length}}
      div(class="text-themeText2") Active
    div(class="bg-themeBackground3 rounded p-2 text-center")
      div(class="text-lg font-bold text-blue-600") {{mockData.allWebhooks.length}}
      div(class="text-themeText2") Total
    div(class="bg-themeBackground3 rounded p-2 text-center")
      div(class="text-lg font-bold text-yellow-600") {{mockData.pausedFlows.length}}
      div(class="text-themeText2") Paused

  // Content Sections
  div(class="space-y-4")
    // Active Webhooks Section
    div(v-if="mockData.activeWebhooks.length > 0")
      div(class="flex items-center gap-x-2 mb-2")
        MdiCheckCircle(class="w-4 h-4 text-green-600")
        h4(class="font-medium") Active Webhooks
      div(class="space-y-2")
        div(v-for="webhook in mockData.activeWebhooks" :key="webhook.id" class="bg-themeBackground3 rounded p-3")
          div(class="flex items-center justify-between")
            div(class="flex-1 min-w-0")
              div(class="font-medium truncate") {{webhook.name}}
              div(class="text-xs text-themeText2 truncate") {{webhook.endpoint}}
            div(class="flex items-center gap-x-1")
              span(class="text-xs bg-green-100 text-green-800 px-2 py-1 rounded") {{webhook.status}}
              button(@click="testWebhook(webhook)" class="p-1 rounded hover:bg-themeBackground2 transition-colors")
                MdiPlay(class="w-3 h-3")
          div(class="text-xs text-themeText2 mt-1")
            span Triggers: {{webhook.trigger_count}}
            span(v-if="webhook.last_triggered" class="ml-2") Last: {{formatDate(webhook.last_triggered)}}

    // Paused Flows Section
    div(v-if="mockData.pausedFlows.length > 0")
      div(class="flex items-center gap-x-2 mb-2")
        MdiPauseCircle(class="w-4 h-4 text-yellow-600")
        h4(class="font-medium") Paused Flows
      div(class="space-y-2")
        div(v-for="flow in mockData.pausedFlows" :key="flow.flow_id" class="bg-yellow-50 border border-yellow-200 rounded p-3")
          div(class="flex items-center justify-between")
            div(class="flex-1 min-w-0")
              div(class="font-medium truncate") {{flow.flow_name}}
              div(class="text-xs text-themeText2 truncate") Waiting for: {{flow.webhook_name}}
            div(class="flex items-center gap-x-1")
              button(@click="resumeFlow(flow)" class="bg-green-600 text-white px-2 py-1 rounded text-xs hover:bg-green-700 transition-colors")
                MdiPlay(class="w-3 h-3 mr-1")
                Resume
              button(@click="cancelFlow(flow)" class="bg-red-600 text-white px-2 py-1 rounded text-xs hover:bg-red-700 transition-colors")
                MdiStop(class="w-3 h-3 mr-1")
                Cancel
          div(class="text-xs text-themeText2 mt-1")
            span Paused: {{formatDate(flow.paused_at)}}

    // All Webhooks Section
    div
      div(class="flex items-center gap-x-2 mb-2")
        MdiWebhook(class="w-4 h-4")
        h4(class="font-medium") All Webhooks
      div(class="space-y-2 max-h-40 overflow-y-auto")
        div(v-for="webhook in mockData.allWebhooks" :key="webhook.id" class="bg-themeBackground3 rounded p-2 group")
          div(class="flex items-center justify-between")
            div(class="flex-1 min-w-0")
              div(class="font-medium truncate") {{webhook.name}}
              div(class="text-xs text-themeText2 truncate") {{webhook.endpoint}}
            div(class="flex items-center gap-x-1")
              button(@click="toggleWebhook(webhook)" class="p-1 rounded hover:bg-themeBackground2 transition-colors")
                MdiToggleSwitch(v-if="webhook.enabled" class="w-4 h-4 text-green-600")
                MdiToggleSwitchOff(v-else class="w-4 h-4 text-gray-400")
              button(@click="deleteWebhook(webhook)" class="p-1 rounded hover:bg-red-100 transition-colors opacity-0 group-hover:opacity-100")
                MdiDelete(class="w-4 h-4 text-red-600")
          div(class="text-xs text-themeText2 mt-1")
            span(v-if="webhook.service_type" class="bg-blue-100 text-blue-800 px-2 py-1 rounded mr-2") {{webhook.service_type}}
            span Triggers: {{webhook.trigger_count}}

  // Create Webhook Modal
  BaseModal(:is-open="showCreateModal" :z-index="12070" @close="showCreateModal = false")
    div(class="vd-modal-card p-6 w-96 max-w-[90vw]")
      div(class="flex items-center justify-between mb-4")
        h3(class="text-lg font-semibold") Create Webhook (Demo)
        button(@click="showCreateModal = false" class="vd-modal-close-btn")
          MdiClose(class="w-4 h-4")
      
      div(class="p-4 bg-blue-50 rounded-lg")
        p(class="text-sm text-blue-800") This is a demo interface. In the full implementation, this would create a real webhook endpoint.
        
      div(class="flex justify-end gap-2 mt-6")
        button(@click="showCreateModal = false" class="px-4 py-2 text-gray-600 hover:bg-gray-100 rounded")
          Cancel
        button(@click="createDemoWebhook" class="px-4 py-2 bg-black text-white rounded hover:bg-gray-800")
          Create Demo
</template>

<script setup lang="ts">
import BaseModal from "./Modal/BaseModal.vue";
import { ref } from 'vue'
import { useMainStore } from '@/stores/Main'

// Icons
import MdiWebhook from "~icons/mdi/webhook?width=24px&height=24px"
import MdiRefresh from "~icons/mdi/refresh?width=24px&height=24px"
import MdiPlus from "~icons/mdi/plus?width=24px&height=24px"
import MdiCheckCircle from "~icons/mdi/check-circle?width=24px&height=24px"
import MdiPauseCircle from "~icons/mdi/pause-circle?width=24px&height=24px"
import MdiPlay from "~icons/mdi/play?width=24px&height=24px"
import MdiStop from "~icons/mdi/stop?width=24px&height=24px"
import MdiToggleSwitch from "~icons/mdi/toggle-switch?width=24px&height=24px"
import MdiToggleSwitchOff from "~icons/mdi/toggle-switch-off?width=24px&height=24px"
import MdiDelete from "~icons/mdi/delete?width=24px&height=24px"
import MdiClose from "~icons/mdi/close?width=24px&height=24px"

const mainStore = useMainStore()

// Mock data
const mockData = ref({
  activeWebhooks: [
    {
      id: '1',
      name: 'GitHub Push Events',
      endpoint: 'http://localhost:3000/webhook/github-push',
      status: 'active',
      trigger_count: 15,
      last_triggered: new Date().toISOString()
    },
    {
      id: '2',
      name: 'Discord Bot Events',
      endpoint: 'http://localhost:3000/webhook/discord-bot',
      status: 'active',
      trigger_count: 7,
      last_triggered: new Date(Date.now() - 3600000).toISOString()
    }
  ],
  pausedFlows: [
    {
      flow_id: 'flow-1',
      flow_name: 'Auto Deploy on Push',
      webhook_name: 'GitHub Push Events',
      paused_at: new Date(Date.now() - 1800000).toISOString()
    }
  ],
  allWebhooks: [
    {
      id: '1',
      name: 'GitHub Push Events',
      endpoint: 'http://localhost:3000/webhook/github-push',
      enabled: true,
      service_type: 'github',
      trigger_count: 15
    },
    {
      id: '2',
      name: 'Discord Bot Events',
      endpoint: 'http://localhost:3000/webhook/discord-bot',
      enabled: true,
      service_type: 'discord',
      trigger_count: 7
    },
    {
      id: '3',
      name: 'Slack Notifications',
      endpoint: 'http://localhost:3000/webhook/slack-notify',
      enabled: false,
      service_type: 'slack',
      trigger_count: 0
    }
  ]
})

// State
const loading = ref(false)
const showCreateModal = ref(false)

// Methods
const refresh = () => {
  loading.value = true
  setTimeout(() => {
    loading.value = false
    mainStore.addNotification({
      title: 'Webhook Data Refreshed',
      message: 'Mock data refreshed successfully',
      type: 'success',
      autoClose: true
    })
  }, 1000)
}

const testWebhook = (webhook: any) => {
  mainStore.addNotification({
    title: 'Webhook Test',
    message: `Test sent to ${webhook.name}`,
    type: 'success',
    autoClose: true
  })
}

const resumeFlow = (flow: any) => {
  mockData.value.pausedFlows = mockData.value.pausedFlows.filter(f => f.flow_id !== flow.flow_id)
  mainStore.addNotification({
    title: 'Flow Resumed',
    message: `${flow.flow_name} resumed successfully`,
    type: 'success',
    autoClose: true
  })
}

const cancelFlow = (flow: any) => {
  mockData.value.pausedFlows = mockData.value.pausedFlows.filter(f => f.flow_id !== flow.flow_id)
  mainStore.addNotification({
    title: 'Flow Cancelled',
    message: `${flow.flow_name} cancelled successfully`,
    type: 'success',
    autoClose: true
  })
}

const toggleWebhook = (webhook: any) => {
  webhook.enabled = !webhook.enabled
  mainStore.addNotification({
    title: 'Webhook Updated',
    message: `${webhook.name} ${webhook.enabled ? 'enabled' : 'disabled'}`,
    type: 'success',
    autoClose: true
  })
}

const deleteWebhook = (webhook: any) => {
  if (!confirm(`Are you sure you want to delete ${webhook.name}?`)) return
  
  mockData.value.allWebhooks = mockData.value.allWebhooks.filter(w => w.id !== webhook.id)
  mockData.value.activeWebhooks = mockData.value.activeWebhooks.filter(w => w.id !== webhook.id)
  
  mainStore.addNotification({
    title: 'Webhook Deleted',
    message: `${webhook.name} deleted successfully`,
    type: 'success',
    autoClose: true
  })
}

const createDemoWebhook = () => {
  const newWebhook = {
    id: String(Date.now()),
    name: 'Demo Webhook',
    endpoint: 'http://localhost:3000/webhook/demo',
    enabled: true,
    service_type: 'custom',
    trigger_count: 0
  }
  
  mockData.value.allWebhooks.push(newWebhook)
  showCreateModal.value = false
  
  mainStore.addNotification({
    title: 'Demo Webhook Created',
    message: 'Demo webhook created successfully',
    type: 'success',
    autoClose: true
  })
}

const formatDate = (dateString: string) => {
  return new Date(dateString).toLocaleString()
}
</script>

<style scoped lang="scss">
@use "./Modal/modalTheme.scss" as *;
</style>
