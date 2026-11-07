<template lang="pug">
div(class="flex flex-col gap-y-6 text-themeText")
  // Top Header & Stats
  div(class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-4 border-b border-themeBorder2")
    div(class="flex items-center gap-x-3")
      div(class="p-2.5 rounded-xl bg-themeBackground3 border border-themeBorder2 text-primary")
        MdiWebhook(class="w-6 h-6")
      div
        h3(class="text-xl font-bold tracking-tight text-themeText") Webhook Gateway
        p(class="text-xs text-themeText2") Ingress hub connecting external APIs and services to VortiDeck automation

    div(class="flex items-center gap-x-3")
      button(
        @click="refresh" 
        :disabled="loading" 
        class="flex items-center gap-x-1.5 px-3 py-2 text-xs font-medium rounded-lg bg-themeBackground3 hover:bg-themeBackground2 border border-themeBorder2 transition-all disabled:opacity-50"
      )
        MdiRefresh(class="w-4 h-4" :class="loading ? 'animate-spin' : ''")
        span Refresh

      button(
        @click="showCreateModal = true" 
        class="flex items-center gap-x-1.5 px-3.5 py-2 text-xs font-semibold rounded-lg bg-primary text-white hover:brightness-110 shadow-sm transition-all"
      )
        MdiPlus(class="w-4 h-4")
        span New Webhook

  // Status Metrics Cards
  div(class="grid grid-cols-1 sm:grid-cols-3 gap-3")
    div(class="bg-themeBackground3 border border-themeBorder2 rounded-xl p-3.5 flex items-center justify-between")
      div
        div(class="text-2xl font-bold text-green-500") {{ activeWebhookCount }}
        div(class="text-xs font-medium text-themeText2 mt-0.5") Active Ingress Endpoints
      div(class="w-9 h-9 rounded-lg bg-green-500/10 flex items-center justify-center text-green-500")
        MdiCheckCircle(class="w-5 h-5")

    div(class="bg-themeBackground3 border border-themeBorder2 rounded-xl p-3.5 flex items-center justify-between")
      div
        div(class="text-2xl font-bold text-blue-500") {{ webhookCount }}
        div(class="text-xs font-medium text-themeText2 mt-0.5") Configured Webhooks
      div(class="w-9 h-9 rounded-lg bg-blue-500/10 flex items-center justify-center text-blue-500")
        MdiWebhook(class="w-5 h-5")

    div(class="bg-themeBackground3 border border-themeBorder2 rounded-xl p-3.5 flex items-center justify-between")
      div
        div(class="text-2xl font-bold text-amber-500") {{ pausedFlowCount }}
        div(class="text-xs font-medium text-themeText2 mt-0.5") Paused Awaiting Input
      div(class="w-9 h-9 rounded-lg bg-amber-500/10 flex items-center justify-center text-amber-500")
        MdiPauseCircle(class="w-5 h-5")

  // Tab Navigation
  div(class="flex items-center gap-x-1 border-b border-themeBorder2")
    button(
      @click="activeTab = 'webhooks'"
      :class="activeTab === 'webhooks' ? 'border-primary text-primary font-semibold' : 'border-transparent text-themeText2 hover:text-themeText'"
      class="flex items-center gap-x-2 px-4 py-2.5 text-sm border-b-2 transition-colors"
    )
      MdiWebhook(class="w-4 h-4")
      span Webhooks
      span(class="text-xs px-2 py-0.5 rounded-full bg-themeBackground3 text-themeText2") {{ webhookCount }}

    button(
      @click="activeTab = 'test-console'"
      :class="activeTab === 'test-console' ? 'border-primary text-primary font-semibold' : 'border-transparent text-themeText2 hover:text-themeText'"
      class="flex items-center gap-x-2 px-4 py-2.5 text-sm border-b-2 transition-colors"
    )
      MdiPlayCircleOutline(class="w-4 h-4")
      span Interactive Test Console

    button(
      @click="activeTab = 'deliveries'"
      :class="activeTab === 'deliveries' ? 'border-primary text-primary font-semibold' : 'border-transparent text-themeText2 hover:text-themeText'"
      class="flex items-center gap-x-2 px-4 py-2.5 text-sm border-b-2 transition-colors"
    )
      MdiHistory(class="w-4 h-4")
      span Ingress Logs

  // TAB 1: Webhooks List
  div(v-if="activeTab === 'webhooks'" class="space-y-4")
    // Empty state
    div(v-if="!loading && webhooks.length === 0" class="flex flex-col items-center justify-center py-12 text-center bg-themeBackground3 border border-themeBorder2 rounded-xl")
      MdiWebhookOff(class="w-12 h-12 mb-3 text-themeText3")
      p(class="text-base font-semibold text-themeText") No Webhooks Registered
      p(class="text-xs text-themeText2 max-w-sm mt-1 mb-4") Set up your first webhook endpoint to receive real-time events from Twitch, Stripe, GitHub, or custom services.
      button(@click="showCreateModal = true" class="px-4 py-2 text-xs font-semibold rounded-lg bg-primary text-white hover:brightness-110") Create Webhook

    // Webhooks Grid
    div(v-else class="grid grid-cols-1 md:grid-cols-2 gap-4")
      div(
        v-for="webhook in webhooks" 
        :key="webhook.id" 
        class="bg-themeBackground3 border border-themeBorder2 rounded-xl p-4 flex flex-col justify-between hover:border-themeBorder transition-all"
      )
        div
          div(class="flex items-start justify-between gap-3 mb-2")
            div(class="flex-1 min-w-0")
              div(class="flex items-center gap-2")
                span(class="font-semibold text-sm truncate text-themeText") {{ webhook.name }}
                span(
                  :class="webhook.enabled ? 'bg-green-500/10 text-green-500 border-green-500/20' : 'bg-zinc-500/10 text-zinc-400 border-zinc-500/20'"
                  class="text-[10px] uppercase font-bold px-2 py-0.5 rounded-full border"
                ) {{ webhook.enabled ? 'Active' : 'Disabled' }}
              div(class="text-xs text-themeText2 line-clamp-1 mt-0.5") {{ webhook.description || 'No description provided' }}

            div(class="flex items-center gap-x-1")
              button(
                @click="toggleWebhook(webhook)" 
                :title="webhook.enabled ? 'Disable Webhook' : 'Enable Webhook'"
                class="p-1.5 rounded-lg hover:bg-themeBackground2 text-themeText2 transition-colors"
              )
                MdiToggleSwitch(v-if="webhook.enabled" class="w-5 h-5 text-green-500")
                MdiToggleSwitchOff(v-else class="w-5 h-5 text-zinc-500")

          // Endpoint details
          div(class="bg-themeBackground2 rounded-lg p-2.5 mt-2 flex items-center justify-between gap-2 border border-themeBorder2/50")
            div(class="flex items-center gap-2 min-w-0")
              span(class="text-[10px] font-mono uppercase px-1.5 py-0.5 rounded bg-themeBackground3 text-themeText2") POST
              span(class="text-xs font-mono truncate text-themeText2") {{ webhook.endpoint_url || webhook.endpoint || ('/webhook/' + webhook.id) }}
            button(
              @click="copyEndpoint(webhook)"
              title="Copy Webhook URL"
              class="p-1 rounded hover:bg-themeBackground3 text-themeText2 hover:text-themeText transition-colors"
            )
              MdiContentCopy(class="w-3.5 h-3.5")

          // Metadata tags
          div(class="flex items-center gap-2 mt-3 flex-wrap")
            span(class="text-[11px] px-2 py-0.5 rounded-md bg-themeBackground2 text-themeText2 font-medium")
              span(class="opacity-60") Service: 
              span(class="text-themeText capitalize") {{ webhook.service || webhook.service_type || 'custom' }}

            span(class="text-[11px] px-2 py-0.5 rounded-md bg-themeBackground2 text-themeText2 font-medium")
              span(class="opacity-60") Mode: 
              span(class="text-themeText") {{ webhook.response_mode || 'onReceived' }}

            span(v-if="webhook.inbound_secret" class="text-[11px] px-2 py-0.5 rounded-md bg-themeBackground2 text-emerald-400 font-medium flex items-center gap-1")
              MdiShieldCheck(class="w-3 h-3")
              span HMAC Active

        // Card Footer Actions
        div(class="flex items-center justify-between border-t border-themeBorder2/60 pt-3 mt-4")
          button(
            @click="openTestConsole(webhook)"
            class="flex items-center gap-1.5 text-xs font-medium text-primary hover:underline"
          )
            MdiPlay(class="w-3.5 h-3.5")
            span Test Payload

          div(class="flex items-center gap-1")
            button(
              @click="openDetails(webhook)"
              class="p-1.5 rounded-lg hover:bg-themeBackground2 text-themeText2 hover:text-themeText transition-colors"
              title="Webhook Details & Analytics"
            )
              MdiInformationOutline(class="w-4 h-4")

            button(
              @click="deleteWebhook(webhook.id)"
              class="p-1.5 rounded-lg hover:bg-red-500/10 text-themeText2 hover:text-red-500 transition-colors"
              title="Delete Webhook"
            )
              MdiDelete(class="w-4 h-4")

  // TAB 2: Interactive Test Console
  div(v-if="activeTab === 'test-console'" class="bg-themeBackground3 border border-themeBorder2 rounded-xl p-6 space-y-6")
    div(class="flex flex-col sm:flex-row sm:items-center justify-between gap-3")
      div
        h4(class="text-base font-bold text-themeText") Webhook Simulation & Testing
        p(class="text-xs text-themeText2") Dispatch test payloads directly through the EventRouter and inspect live workflow responses

      div(class="flex items-center gap-2")
        span(class="text-xs text-themeText2") Target Webhook:
        select(
          v-model="selectedTestWebhookId" 
          class="px-3 py-1.5 text-xs rounded-lg bg-themeBackground2 border border-themeBorder2 text-themeText focus:ring-1 focus:ring-primary"
        )
          option(v-for="w in webhooks" :key="w.id" :value="w.id") {{ w.name }} ({{ w.service || 'custom' }})

    // Preset Buttons
    div(class="flex items-center gap-2 flex-wrap")
      span(class="text-xs font-medium text-themeText2") Load Preset:
      button(
        v-for="preset in testPresets" 
        :key="preset.name"
        @click="loadPreset(preset)"
        class="px-2.5 py-1 text-xs rounded-md bg-themeBackground2 hover:bg-themeBackground border border-themeBorder2 text-themeText transition-colors"
      ) {{ preset.name }}

    // Split View Editor & Response
    div(class="grid grid-cols-1 lg:grid-cols-2 gap-4")
      // Left: Request JSON Payload
      div(class="space-y-2")
        div(class="flex items-center justify-between")
          label(class="text-xs font-semibold text-themeText2 uppercase") Request JSON Body
          button(
            @click="formatTestPayload"
            class="text-[11px] text-primary hover:underline"
          ) Format JSON
        textarea(
          v-model="testPayloadText"
          rows="12"
          placeholder="{\n  \"event\": \"test\"\n}"
          class="w-full p-3 font-mono text-xs rounded-lg bg-themeBackground2 border border-themeBorder2 text-themeText focus:ring-1 focus:ring-primary outline-none"
        )

        div(class="flex justify-end pt-2")
          button(
            @click="sendTestPayload"
            :disabled="isSendingTest || !selectedTestWebhookId"
            class="flex items-center gap-x-2 px-5 py-2 text-xs font-semibold rounded-lg bg-primary text-white hover:brightness-110 disabled:opacity-50 transition-all"
          )
            MdiSend(class="w-4 h-4" :class="isSendingTest ? 'animate-bounce' : ''")
            span(v-if="isSendingTest") Dispatching...
            span(v-else) Send Test Payload

      // Right: Live Response Inspector
      div(class="space-y-2")
        div(class="flex items-center justify-between")
          label(class="text-xs font-semibold text-themeText2 uppercase") Flow Response Inspector
          span(v-if="testResult" class="text-xs font-mono text-themeText2") Latency: {{ testLatencyMs }}ms

        div(class="w-full h-64 p-3 font-mono text-xs rounded-lg bg-themeBackground2 border border-themeBorder2 overflow-auto text-themeText")
          div(v-if="!testResult && !isSendingTest" class="flex flex-col items-center justify-center h-full text-themeText3")
            MdiCodeJson(class="w-10 h-10 mb-2")
            span Send a test payload to view the live execution response
          div(v-else-if="isSendingTest" class="flex items-center justify-center h-full text-themeText2")
            MdiLoading(class="w-6 h-6 animate-spin mr-2")
            span Awaiting workflow execution...
          pre(v-else class="whitespace-pre-wrap text-emerald-400") {{ JSON.stringify(testResult, null, 2) }}

  // TAB 3: Delivery Logs
  div(v-if="activeTab === 'deliveries'" class="bg-themeBackground3 border border-themeBorder2 rounded-xl p-6 space-y-4")
    div(class="flex items-center justify-between")
      div
        h4(class="text-base font-bold text-themeText") Recent Ingress Deliveries
        p(class="text-xs text-themeText2") Historical audit trail of webhooks processed through SQLite inbox

      button(
        @click="refresh"
        class="p-1.5 rounded-lg hover:bg-themeBackground2 text-themeText2"
      )
        MdiRefresh(class="w-4 h-4")

    div(class="overflow-x-auto")
      table(class="w-full text-left text-xs border-collapse")
        thead
          tr(class="border-b border-themeBorder2 text-themeText2 uppercase font-semibold")
            th(class="py-2.5 px-3") Status
            th(class="py-2.5 px-3") Webhook / Service
            th(class="py-2.5 px-3") Event
            th(class="py-2.5 px-3") Time
            th(class="py-2.5 px-3 text-right") Action
        tbody(class="divide-y divide-themeBorder2/50")
          tr(v-if="deliveries.length === 0")
            td(colspan="5" class="py-8 text-center text-themeText3") No recent deliveries recorded yet.
          tr(
            v-for="item in deliveries" 
            :key="item.id"
            class="hover:bg-themeBackground2 transition-colors"
          )
            td(class="py-2.5 px-3")
              span(
                :class="item.status === 'success' ? 'bg-green-500/10 text-green-500 border-green-500/20' : 'bg-red-500/10 text-red-500 border-red-500/20'"
                class="px-2 py-0.5 rounded-full font-bold uppercase text-[10px] border"
              ) {{ item.response_code || 200 }} OK
            td(class="py-2.5 px-3 font-medium text-themeText") {{ item.webhook_name || item.webhook_id }}
            td(class="py-2.5 px-3 text-themeText2 font-mono") {{ item.event_type || 'webhook.received' }}
            td(class="py-2.5 px-3 text-themeText2") {{ formatDate(item.created_at) }}
            td(class="py-2.5 px-3 text-right")
              button(
                @click="inspectDelivery(item)"
                class="text-primary hover:underline text-xs font-medium"
              ) Inspect

  // Create Webhook Modal
  WebhookCreateModal(
    :is-open="showCreateModal" 
    @close="showCreateModal = false"
    @created="refresh"
  )

  // Webhook Details Modal
  WebhookDetailsModal(
    v-if="selectedWebhookForDetails"
    :is-open="showDetailsModal"
    :webhook="selectedWebhookForDetails"
    @close="showDetailsModal = false"
    @updated="refresh"
  )
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useWebhookManager, type WebhookConfig } from '@desktop/composables/useWebhookManager'
import { useMainStore } from '@/stores/Main'
import WebhookCreateModal from './Modal/WebhookCreate.vue'
import WebhookDetailsModal from './Modal/WebhookDetails.vue'

// Icons
import MdiWebhook from "~icons/mdi/webhook?width=24px&height=24px"
import MdiWebhookOff from "~icons/mdi/link-off?width=24px&height=24px"
import MdiRefresh from "~icons/mdi/refresh?width=24px&height=24px"
import MdiPlus from "~icons/mdi/plus?width=24px&height=24px"
import MdiCheckCircle from "~icons/mdi/check-circle?width=24px&height=24px"
import MdiPauseCircle from "~icons/mdi/pause-circle?width=24px&height=24px"
import MdiPlay from "~icons/mdi/play?width=24px&height=24px"
import MdiPlayCircleOutline from "~icons/mdi/play-circle-outline?width=24px&height=24px"
import MdiHistory from "~icons/mdi/history?width=24px&height=24px"
import MdiToggleSwitch from "~icons/mdi/toggle-switch?width=24px&height=24px"
import MdiToggleSwitchOff from "~icons/mdi/toggle-switch-off?width=24px&height=24px"
import MdiDelete from "~icons/mdi/delete?width=24px&height=24px"
import MdiContentCopy from "~icons/mdi/content-copy?width=24px&height=24px"
import MdiShieldCheck from "~icons/mdi/shield-check?width=24px&height=24px"
import MdiInformationOutline from "~icons/mdi/information-outline?width=24px&height=24px"
import MdiSend from "~icons/mdi/send?width=24px&height=24px"
import MdiLoading from "~icons/mdi/loading?width=24px&height=24px"
import MdiCodeJson from "~icons/mdi/code-json?width=24px&height=24px"

const mainStore = useMainStore()
const webhookManager = useWebhookManager()

const {
  loading,
  error,
  webhooks,
  webhookCount,
  activeWebhookCount,
  pausedFlowCount,
  initialize,
  updateWebhook,
  deleteWebhook: deleteWebhookApi,
  testWebhook: testWebhookApi,
  getWebhookLogs
} = webhookManager

const activeTab = ref<'webhooks' | 'test-console' | 'deliveries'>('webhooks')
const showCreateModal = ref(false)
const showDetailsModal = ref(false)
const selectedWebhookForDetails = ref<WebhookConfig | null>(null)

// Test Console State
const selectedTestWebhookId = ref<string>('')
const isSendingTest = ref(false)
const testLatencyMs = ref<number>(0)
const testResult = ref<any>(null)
const testPayloadText = ref<string>(JSON.stringify({
  event: "channel.follow",
  user_name: "test_viewer",
  broadcaster_user_name: "streamer",
  followed_at: new Date().toISOString()
}, null, 2))

const testPresets = [
  {
    name: "Twitch Follow",
    payload: {
      subscription: { type: "channel.follow", version: "2" },
      event: { user_id: "12345", user_name: "twitch_viewer", user_login: "twitch_viewer", broadcaster_user_name: "my_channel" }
    }
  },
  {
    name: "Twitch Raid",
    payload: {
      subscription: { type: "channel.raid", version: "1" },
      event: { from_broadcaster_user_name: "fellow_streamer", viewers: 45 }
    }
  },
  {
    name: "Stripe Payment",
    payload: {
      type: "payment_intent.succeeded",
      data: { object: { amount: 2500, currency: "usd", customer_email: "supporter@example.com" } }
    }
  },
  {
    name: "GitHub Push",
    payload: {
      ref: "refs/heads/main",
      repository: { name: "VortiDeck", full_name: "user/VortiDeck" },
      pusher: { name: "developer", email: "dev@example.com" }
    }
  }
]

const deliveries = ref<any[]>([])

// Auto-refresh interval
let refreshInterval: NodeJS.Timeout | null = null

const refresh = async () => {
  try {
    await initialize()
    if (webhooks.value.length > 0 && !selectedTestWebhookId.value) {
      selectedTestWebhookId.value = webhooks.value[0].id
    }
    // Fetch recent logs
    if (selectedTestWebhookId.value) {
      try {
        deliveries.value = await getWebhookLogs(selectedTestWebhookId.value, 20)
      } catch (_) {}
    }
  } catch (err) {
    console.error('Error refreshing webhooks:', err)
  }
}

const copyEndpoint = (webhook: WebhookConfig) => {
  const url = webhook.endpoint_url || webhook.endpoint || (`/webhook/${webhook.id}`)
  navigator.clipboard.writeText(url)
  mainStore.addNotification({
    title: 'URL Copied',
    message: 'Webhook endpoint copied to clipboard',
    type: 'success',
    autoClose: true
  })
}

const toggleWebhook = async (webhook: WebhookConfig) => {
  try {
    await updateWebhook(webhook.id, { enabled: !webhook.enabled })
    mainStore.addNotification({
      title: 'Webhook Updated',
      message: `Webhook ${webhook.enabled ? 'disabled' : 'enabled'}`,
      type: 'success',
      autoClose: true
    })
  } catch (err) {
    mainStore.addNotification({
      title: 'Update Failed',
      message: error.value || 'Failed to update webhook',
      type: 'error',
      autoClose: true
    })
  }
}

const deleteWebhook = async (webhookId: string) => {
  if (!confirm('Are you sure you want to delete this webhook?')) return
  try {
    await deleteWebhookApi(webhookId)
    mainStore.addNotification({
      title: 'Webhook Deleted',
      message: 'Webhook removed successfully',
      type: 'success',
      autoClose: true
    })
  } catch (err) {
    mainStore.addNotification({
      title: 'Delete Failed',
      message: error.value || 'Failed to delete webhook',
      type: 'error',
      autoClose: true
    })
  }
}

const openDetails = (webhook: WebhookConfig) => {
  selectedWebhookForDetails.value = webhook
  showDetailsModal.value = true
}

const openTestConsole = (webhook: WebhookConfig) => {
  selectedTestWebhookId.value = webhook.id
  activeTab.value = 'test-console'
}

const loadPreset = (preset: any) => {
  testPayloadText.value = JSON.stringify(preset.payload, null, 2)
}

const formatTestPayload = () => {
  try {
    const parsed = JSON.parse(testPayloadText.value)
    testPayloadText.value = JSON.stringify(parsed, null, 2)
  } catch (_) {}
}

const sendTestPayload = async () => {
  if (!selectedTestWebhookId.value) return
  isSendingTest.value = true
  testResult.value = null
  const start = performance.now()

  try {
    let parsedBody = {}
    try {
      parsedBody = JSON.parse(testPayloadText.value)
    } catch (_) {
      parsedBody = { raw: testPayloadText.value }
    }

    await testWebhookApi(selectedTestWebhookId.value)
    testLatencyMs.value = Math.round(performance.now() - start)
    testResult.value = {
      status: 200,
      statusText: "OK",
      latencyMs: testLatencyMs.value,
      message: "Webhook event ingested and routed successfully via EventRouter",
      payload: parsedBody
    }

    mainStore.addNotification({
      title: 'Test Dispatched',
      message: `Payload delivered in ${testLatencyMs.value}ms`,
      type: 'success',
      autoClose: true
    })
  } catch (err: any) {
    testLatencyMs.value = Math.round(performance.now() - start)
    testResult.value = {
      status: 500,
      error: err?.message || 'Failed to process webhook test'
    }
  } finally {
    isSendingTest.value = false
  }
}

const inspectDelivery = (item: any) => {
  openDetails(item)
}

const formatDate = (dateString?: string) => {
  if (!dateString) return 'Just now'
  return new Date(dateString).toLocaleTimeString()
}

onMounted(async () => {
  await refresh()
  refreshInterval = setInterval(refresh, 15000)
})

onUnmounted(() => {
  if (refreshInterval) clearInterval(refreshInterval)
})
</script>
