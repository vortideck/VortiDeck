<template lang="pug">
div(class="w-full h-full flex flex-col bg-gray-50 dark:bg-gray-900 min-h-screen")
  // Professional Header
  div(class="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4")
    div(class="flex items-center justify-between")
      div
        h1(class="text-2xl font-semibold text-gray-900 dark:text-gray-100") API & Integrations
        p(class="text-sm text-gray-600 dark:text-gray-400 mt-1") Manage webhooks, API endpoints, and external integrations
      
      div(class="flex items-center gap-3")
        button(@click="refresh" :disabled="loading" class="flex items-center gap-2 px-4 py-2 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors")
          MdiRefresh(class="w-4 h-4" :class="loading ? 'animate-spin' : ''")
          span Refresh
        
        button(@click="openCreateModal" class="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors")
          MdiPlus(class="w-4 h-4")
          span Create Integration

  // Tab Navigation
  div(class="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700")
    nav(class="flex space-x-8 px-6" aria-label="Tabs")
      button(
        v-for="tab in tabs" 
        :key="tab.id"
        @click="activeTab = tab.id"
        :class="activeTab === tab.id ? 'border-blue-500 text-blue-600 dark:text-blue-400' : 'border-transparent text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 hover:border-gray-300'"
        class="flex items-center gap-2 py-4 px-1 border-b-2 font-medium text-sm transition-colors"
      )
        component(:is="tab.icon" class="w-4 h-4")
        span {{ tab.name }}
        span(v-if="tab.count" class="bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300 rounded-full px-2 py-0.5 text-xs font-medium") {{ tab.count }}

  // Tab Content
  div(class="flex-1 p-6")
    // Webhooks Tab
    div(v-if="activeTab === 'webhooks'" class="space-y-6")
      // Server Info Banner
      div(v-if="serverInfo" class="bg-gradient-to-r from-blue-50 to-indigo-50 dark:from-blue-900/20 dark:to-indigo-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4 mb-6")
        div(class="flex items-center justify-between")
          div(class="flex items-center gap-4")
            div(class="w-12 h-12 bg-blue-600 rounded-lg flex items-center justify-center")
              MdiServer(class="w-6 h-6 text-white")
            div
              div(class="text-sm font-medium text-blue-900 dark:text-blue-100 mb-1") Server Address
              div(class="flex items-center gap-3")
                span(class="font-mono text-lg font-bold text-blue-700 dark:text-blue-300") http://{{ serverInfo.ip }}:{{ serverInfo.port }}
                button(@click="copyServerAddress" class="p-1.5 rounded-md bg-blue-100 dark:bg-blue-800 hover:bg-blue-200 dark:hover:bg-blue-700 transition-colors" title="Copy base URL")
                  MdiContentCopy(class="w-4 h-4 text-blue-600 dark:text-blue-300")
          div(class="flex items-center gap-2 px-3 py-1.5 bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400 rounded-full text-sm font-medium")
            div(class="w-2 h-2 bg-green-600 rounded-full animate-pulse")
            span Online

      div(v-else class="bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg p-4 mb-6")
        div(class="flex items-center gap-3")
          MdiAlertCircle(class="w-5 h-5 text-yellow-600 dark:text-yellow-400")
          div
            div(class="text-sm font-medium text-yellow-900 dark:text-yellow-100") Server address not available
            div(class="text-xs text-yellow-700 dark:text-yellow-300 mt-1") Make sure the VortiDeck server is running to receive webhooks

      // Statistics Cards (existing professional design)
      div(class="grid grid-cols-1 sm:grid-cols-4 gap-4 mb-6")
        div(class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-6 shadow-sm hover:shadow-md transition-shadow")
          div(class="flex items-center justify-between")
            div
              div(class="text-2xl font-semibold text-gray-900 dark:text-gray-100") {{getFilteredHealthyWebhooksCount()}}
              div(class="text-sm text-gray-600 dark:text-gray-400 mt-1") Healthy Webhooks
            div(class="w-10 h-10 bg-green-100 dark:bg-green-900/30 rounded-lg flex items-center justify-center")
              MdiCheckCircle(class="w-5 h-5 text-green-600 dark:text-green-400")
          div(class="mt-4 w-full bg-gray-200 dark:bg-gray-700 rounded-full h-1.5")
            div(
              class="bg-green-600 dark:bg-green-400 rounded-full h-1.5 transition-all duration-700 ease-out" 
              :style="`width: ${getFilteredHealthyPercentage}%`"
            )
        
        div(class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-6 shadow-sm hover:shadow-md transition-shadow")
          div(class="flex items-center justify-between")
            div
              div(class="text-2xl font-semibold text-gray-900 dark:text-gray-100") {{getFilteredWarningWebhooksCount()}}
              div(class="text-sm text-gray-600 dark:text-gray-400 mt-1") Need Attention
            div(class="w-10 h-10 bg-yellow-100 dark:bg-yellow-900/30 rounded-lg flex items-center justify-center")
              MdiAlertCircle(class="w-5 h-5 text-yellow-600 dark:text-yellow-400")
        
        div(class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-6 shadow-sm hover:shadow-md transition-shadow")
          div(class="flex items-center justify-between")
            div
              div(class="text-2xl font-semibold text-gray-900 dark:text-gray-100") {{filteredWebhooks.length}}
              div(class="text-sm text-gray-600 dark:text-gray-400 mt-1") Total Webhooks
            div(class="w-10 h-10 bg-blue-100 dark:bg-blue-900/30 rounded-lg flex items-center justify-center")
              MdiWebhook(class="w-5 h-5 text-blue-600 dark:text-blue-400")
        
        div(class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-6 shadow-sm hover:shadow-md transition-shadow")
          div(class="flex items-center justify-between")
            div
              div(class="text-2xl font-semibold text-gray-900 dark:text-gray-100") {{getFilteredTotalRequestsToday()}}
              div(class="text-sm text-gray-600 dark:text-gray-400 mt-1") Requests Today
            div(class="w-10 h-10 bg-purple-100 dark:bg-purple-900/30 rounded-lg flex items-center justify-center")
              MdiChartLine(class="w-5 h-5 text-purple-600 dark:text-purple-400")

      // Filters Section
      div(class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6")
        div(class="flex items-center justify-between mb-4")
          h3(class="text-lg font-medium text-gray-900 dark:text-gray-100") Filters & Search
          button(@click="showFilters = !showFilters" class="flex items-center gap-2 px-3 py-2 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-md hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors")
            MdiFilter(class="w-4 h-4")
            span Filters
            MdiBadge(v-if="activeFiltersCount > 0" class="w-4 h-4 text-blue-600")
            button(v-if="hasActiveFilters" @click.stop="clearAllFilters" class=" rounded-full p-1 text-white bg-red-400 hover:bg-red-500 dark:hover:bg-red-500 transition-colors")
              MdiClose(class="w-3 h-3")
        
        Transition(name="slide-down")
          div(v-if="showFilters" class="grid grid-cols-1 md:grid-cols-4 gap-4")
            div
              label(class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2") Search
              input(v-model="searchQuery" type="text" placeholder="Name, URL, or payload..." class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:text-white")
            div
              label(class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2") Status
              select(v-model="selectedStatus" class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:text-white")
                option(value="") All Status
                option(value="healthy") Healthy
                option(value="warning") Warning
                option(value="critical") Critical
                option(value="offline") Offline
            div
              label(class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2") Service Type
              select(v-model="selectedServiceType" class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:text-white")
                option(value="") All Services
                option(value="github") GitHub
                option(value="discord") Discord
                option(value="slack") Slack
                option(value="stripe") Stripe
                option(value="custom") Custom
            div
              label(class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2") Activity
              select(v-model="selectedActivity" class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:text-white")
                option(value="") All Activity
                option(value="active") Active (24h)
                option(value="idle") Idle (7d+)
                option(value="never") Never used

      // Webhooks List
      div(class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700")
        div(class="p-6 border-b border-gray-200 dark:border-gray-700")
          div(class="flex items-center justify-between")
            h3(class="text-lg font-medium text-gray-900 dark:text-gray-200") Webhooks
            div(class="text-sm text-gray-500 dark:text-gray-400")
              span {{filteredWebhooks.length}} of {{mockData.allWebhooks.length}} webhooks
        
        div(v-if="filteredWebhooks.length === 0" class="text-center py-16")
          div(class="w-16 h-16 bg-gray-100 dark:bg-gray-700 rounded-full flex items-center justify-center mx-auto mb-4")
            MdiWebhookOff(class="w-8 h-8 text-gray-400")
          h4(class="text-lg font-medium text-gray-700 dark:text-gray-300 mb-2") No webhooks found
          p(class="text-gray-500 dark:text-gray-400 mb-4") {{hasActiveFilters ? 'Try adjusting your filters' : 'Create your first webhook to get started'}}
        
        div(v-else class="divide-y divide-gray-200 dark:divide-gray-700")
          div(v-for="webhook in filteredWebhooks" :key="webhook.id" class="p-6 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors cursor-pointer")
            div(class="flex items-start justify-between")
              div(class="flex items-start gap-4 flex-1 min-w-0")
                div(class="w-10 h-10 rounded-lg flex items-center justify-center" :class="getWebhookStatusClass(webhook)")
                  component(:is="getWebhookStatusIcon(webhook)" class="w-5 h-5")
                
                div(class="flex-1 min-w-0")
                  div(class="flex items-center gap-3 mb-2")
                    h4(class="text-lg font-medium text-gray-900 dark:text-gray-200 truncate") {{webhook.name}}
                    span(v-if="webhook.service_type" class="inline-flex items-center px-2 py-1 rounded-md text-xs font-medium" :class="getServiceTypeClass(webhook.service_type)")
                      span {{webhook.service_type}}
                  
                  div(class="space-y-1")
                    div(class="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400")
                      MdiWebhook(class="w-4 h-4")
                      span(class="font-mono text-sm font-medium truncate") {{ getFullWebhookUrl(webhook) }}
                    
                    div(class="flex items-center gap-6 text-xs text-gray-500 dark:text-gray-500")
                      div(class="flex items-center gap-1")
                        MdiChartBox(class="w-3 h-3")
                        span {{ webhook.trigger_count || 0 }} requests
                      div(class="flex items-center gap-1")
                        MdiCheckCircle(class="w-3 h-3 text-green-500")
                        span {{ getSuccessRate(webhook) }}% success
                      div(class="flex items-center gap-1")
                        MdiSpeedometer(class="w-3 h-3")
                        span {{ getAverageResponseTime(webhook) }}ms avg
                      div(v-if="webhook.last_triggered" class="flex items-center gap-1")
                        MdiClock(class="w-3 h-3")
                        span Last: {{ formatDate(webhook.last_triggered) }}
              
              div(class="flex items-center gap-2")
                button(@click="testWebhook(webhook)" class="p-2 rounded-lg bg-blue-50 text-blue-600 hover:bg-blue-100 dark:bg-blue-900/50 dark:text-blue-400 dark:hover:bg-blue-900 transition-colors")
                  MdiTestTube(class="w-4 h-4")
                button(@click="editWebhook(webhook)" class="p-2 rounded-lg bg-gray-50 text-gray-600 hover:bg-gray-100 dark:bg-gray-700 dark:text-gray-400 dark:hover:bg-gray-600 transition-colors")
                  MdiPencil(class="w-4 h-4")
                button(@click="openDetailsModal(webhook)" class="p-2 rounded-lg bg-gray-50 text-gray-600 hover:bg-gray-100 dark:bg-gray-700 dark:text-gray-400 dark:hover:bg-gray-600 transition-colors")
                  MdiInformation(class="w-4 h-4")

    // API Endpoints Tab
    div(v-else-if="activeTab === 'api-endpoints'" class="space-y-6")
      div(class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-8 text-center")
        MdiApi(class="w-16 h-16 text-gray-400 mx-auto mb-4")
        h3(class="text-xl font-medium text-gray-900 dark:text-gray-100 mb-2") REST API Endpoints
        p(class="text-gray-600 dark:text-gray-400 mb-6") Manage and monitor your REST API endpoints, rate limiting, and authentication.
        button(class="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors") Configure API Endpoints

    // Authentication & Security Tab
    div(v-else-if="activeTab === 'auth'" class="space-y-6")
      div(class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-8 text-center")
        MdiSecurity(class="w-16 h-16 text-gray-400 mx-auto mb-4")
        h3(class="text-xl font-medium text-gray-900 dark:text-gray-100 mb-2") Authentication & Security
        p(class="text-gray-600 dark:text-gray-400 mb-6") Configure API keys, HMAC signatures, IP whitelisting, and security policies.
        button(class="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors") Manage Security

    // Analytics & Monitoring Tab
    div(v-else-if="activeTab === 'analytics'" class="space-y-6")
      div(class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-8 text-center")
        MdiChartLine(class="w-16 h-16 text-gray-400 mx-auto mb-4")
        h3(class="text-xl font-medium text-gray-900 dark:text-gray-100 mb-2") Analytics & Monitoring
        p(class="text-gray-600 dark:text-gray-400 mb-6") View detailed analytics, performance metrics, and real-time monitoring dashboards.
        button(class="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors") View Analytics

    // Settings Tab
    div(v-else-if="activeTab === 'settings'" class="space-y-6")
      div(class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-8 text-center")
        MdiCog(class="w-16 h-16 text-gray-400 mx-auto mb-4")
        h3(class="text-xl font-medium text-gray-900 dark:text-gray-100 mb-2") Integration Settings
        p(class="text-gray-600 dark:text-gray-400 mb-6") Configure global settings, notification preferences, and integration defaults.
        button(class="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors") Configure Settings

  // Modal Components
  WebhookCreate(
    :isOpen="showCreateModal"
    :isEdit="isEditMode"
    :webhook="editingWebhook"
    @close="closeCreateModal"
    @submit="handleWebhookSubmit"
  )
  
  WebhookDetails(
    :isOpen="showDetailsModal"
    :webhook="selectedWebhook"
    :initialTab="detailsInitialTab"
    @close="closeDetailsModal"
    @edit="editWebhookFromDetails"
  )
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, type Component } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useMainStore } from "@/stores/Main";
import {
  useWebhookManager,
  type WebhookConfig,
  type WebhookLog,
  type WebhookSubmission,
  type WebhookTestResult,
} from "../composables/useWebhookManager";
import WebhookCreate from "../components/Modal/WebhookCreate.vue";
import WebhookDetails from "../components/Modal/WebhookDetails.vue";

// Icons
import MdiWebhook from "~icons/mdi/webhook?width=24px&height=24px";
import MdiRefresh from "~icons/mdi/refresh?width=24px&height=24px";
import MdiPlus from "~icons/mdi/plus?width=24px&height=24px";
import MdiFilter from "~icons/mdi/filter?width=24px&height=24px";
import MdiFilterOff from "~icons/mdi/filter-off?width=24px&height=24px";
import MdiPlay from "~icons/mdi/play?width=24px&height=24px";
import MdiPause from "~icons/mdi/pause?width=24px&height=24px";
import MdiStop from "~icons/mdi/stop?width=24px&height=24px";
import MdiToggleSwitch from "~icons/mdi/toggle-switch?width=24px&height=24px";
import MdiToggleSwitchOff from "~icons/mdi/toggle-switch-off?width=24px&height=24px";
import MdiDelete from "~icons/mdi/delete?width=24px&height=24px";
import MdiInformation from "~icons/mdi/information?width=24px&height=24px";
import MdiPencil from "~icons/mdi/pencil?width=24px&height=24px";
import MdiCheckboxMultipleMarked from "~icons/mdi/checkbox-multiple-marked?width=24px&height=24px";
import MdiBadge from "~icons/mdi/badge?width=24px&height=24px";
import MdiAlertCircle from "~icons/mdi/alert-circle?width=24px&height=24px";
import MdiCheckCircle from "~icons/mdi/check-circle?width=24px&height=24px";
import MdiChartLine from "~icons/mdi/chart-line?width=24px&height=24px";
import MdiChartBox from "~icons/mdi/chart-box?width=24px&height=24px";
import MdiSpeedometer from "~icons/mdi/speedometer?width=24px&height=24px";
import MdiClock from "~icons/mdi/clock?width=24px&height=24px";
import MdiTestTube from "~icons/mdi/test-tube?width=24px&height=24px";
import MdiTextBoxSearchOutline from "~icons/mdi/text-box-search-outline?width=24px&height=24px";
import MdiDownload from "~icons/mdi/download?width=24px&height=24px";
import MdiContentCopy from "~icons/mdi/content-copy?width=24px&height=24px";
import MdiEye from "~icons/mdi/eye?width=24px&height=24px";
import MdiEyeOff from "~icons/mdi/eye-off?width=24px&height=24px";
import MdiRocket from "~icons/mdi/rocket?width=24px&height=24px";
import MdiLoading from "~icons/mdi/loading?width=24px&height=24px";
import MdiClose from "~icons/mdi/close?width=24px&height=24px";
import MdiFileDocumentOutline from "~icons/mdi/file-document-outline?width=24px&height=24px";
import MdiGithub from "~icons/mdi/github?width=24px&height=24px";
import MdiDiscord from "~icons/mdi/discord?width=24px&height=24px";
import MdiSlack from "~icons/mdi/slack?width=24px&height=24px";
import MdiCreditCard from "~icons/mdi/credit-card?width=24px&height=24px";
import MdiApi from "~icons/mdi/api?width=24px&height=24px";
import MdiSecurity from "~icons/mdi/security?width=24px&height=24px";
import MdiCog from "~icons/mdi/cog?width=24px&height=24px";
import MdiServer from "~icons/mdi/server?width=24px&height=24px";
import MdiWebhookOff from "~icons/mdi/link-off?width=24px&height=24px";

const mainStore = useMainStore();

interface WebhookDisplay {
  id: string;
  enabled: boolean;
  last_triggered?: string | null;
  service?: string;
  service_type?: string;
}

// Initialize webhook manager
const webhookManager = useWebhookManager();

// Server info for constructing full webhook URLs
const serverInfo = ref<{ ip: string; port: number } | null>(null);

// Tab Management
const activeTab = ref("webhooks");
const tabs = ref([
  {
    id: "webhooks",
    name: "Webhooks",
    icon: MdiWebhook,
    count: computed(() => webhookManager.webhooks.value.length),
  },
  {
    id: "api-endpoints",
    name: "API Endpoints",
    icon: MdiApi,
    count: null,
  },
  {
    id: "auth",
    name: "Security",
    icon: MdiSecurity,
    count: null,
  },
  {
    id: "analytics",
    name: "Analytics",
    icon: MdiChartLine,
    count: null,
  },
  {
    id: "settings",
    name: "Settings",
    icon: MdiCog,
    count: null,
  },
]);

// Mock data
const mockData = ref({
  pausedFlows: [
    {
      flow_id: "flow-1",
      flow_name: "Auto Deploy Pipeline",
      webhook_name: "GitHub Push Events",
      paused_at: new Date(Date.now() - 1800000).toISOString(),
    },
    {
      flow_id: "flow-2",
      flow_name: "Notification System",
      webhook_name: "Discord Bot Commands",
      paused_at: new Date(Date.now() - 900000).toISOString(),
    },
  ],
  allWebhooks: [
    {
      id: "1",
      name: "GitHub Push Events",
      endpoint: "http://localhost:3000/webhook/github-push",
      enabled: true,
      service_type: "github",
      trigger_count: 23,
      last_triggered: new Date().toISOString(),
      secret: "my-secret-key",
      requireSignature: false,
      rateLimit: 100,
      timeout: 30,
      maxRetries: 3,
      backoffStrategy: "Exponential",
      baseDelay: 1000,
      updated_at: new Date().toISOString(),
    },
    {
      id: "2",
      name: "Discord Bot Commands",
      endpoint: "http://localhost:3000/webhook/discord-bot",
      enabled: true,
      service_type: "discord",
      trigger_count: 12,
      last_triggered: new Date(Date.now() - 3600000).toISOString(),
      secret: "discord-secret",
      requireSignature: true,
      rateLimit: 50,
      timeout: 10,
      maxRetries: 1,
      backoffStrategy: "Linear",
      baseDelay: 500,
      updated_at: new Date().toISOString(),
    },
    {
      id: "3",
      name: "Slack Notifications",
      endpoint: "http://localhost:3000/webhook/slack-notify",
      enabled: false,
      service_type: "slack",
      trigger_count: 0,
      last_triggered: null,
      secret: "slack-secret",
      requireSignature: false,
      rateLimit: 1000,
      timeout: 60,
      maxRetries: 5,
      backoffStrategy: "Exponential",
      baseDelay: 1000,
      updated_at: new Date().toISOString(),
    },
    {
      id: "4",
      name: "Stripe Payments",
      endpoint: "http://localhost:3000/webhook/stripe-payments",
      enabled: true,
      service_type: "stripe",
      trigger_count: 5,
      last_triggered: new Date(Date.now() - 7200000).toISOString(),
      secret: "stripe-secret",
      requireSignature: true,
      rateLimit: 200,
      timeout: 15,
      maxRetries: 2,
      backoffStrategy: "Exponential",
      baseDelay: 2000,
      updated_at: new Date().toISOString(),
    },
    {
      id: "5",
      name: "Custom API Webhook",
      endpoint: "http://localhost:3000/webhook/custom-api",
      enabled: false,
      service_type: "custom",
      trigger_count: 0,
      last_triggered: null,
      secret: "custom-secret",
      requireSignature: false,
      rateLimit: 500,
      timeout: 30,
      maxRetries: 3,
      backoffStrategy: "Linear",
      baseDelay: 1000,
      updated_at: new Date().toISOString(),
    },
  ],
});

// State
const loading = ref(false);
const showCreateModal = ref(false);
const showFilters = ref(false);
const showDetailsModal = ref(false);
const selectedWebhook = ref<WebhookConfig | null>(null);
const selectedWebhooks = ref<string[]>([]);
const showBulkActions = ref(false);
const selectAll = ref(false);
const isEditMode = ref(false);
const editingWebhook = ref<WebhookConfig | null>(null);

// Filter state
const searchQuery = ref("");
const selectedServiceType = ref("");
const selectedStatus = ref("");
const selectedActivity = ref("");

// Computed properties
const filteredWebhooks = computed(() => {
  let filtered = webhookManager.webhooks.value;

  if (searchQuery.value) {
    filtered = filtered.filter(
      (webhook) =>
        webhook.name.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
        (webhook.endpoint_url || webhook.endpoint || "")
          .toLowerCase()
          .includes(searchQuery.value.toLowerCase())
    );
  }

  if (selectedServiceType.value) {
    filtered = filtered.filter(
      (webhook) => webhook.service_type === selectedServiceType.value
    );
  }

  if (selectedStatus.value) {
    filtered = filtered.filter((webhook) =>
      selectedStatus.value === "healthy"
        ? getWebhookHealth(webhook) === "healthy"
        : selectedStatus.value === "warning"
        ? getWebhookHealth(webhook) === "warning"
        : selectedStatus.value === "critical"
        ? getWebhookHealth(webhook) === "critical"
        : selectedStatus.value === "offline"
        ? getWebhookHealth(webhook) === "offline"
        : true
    );
  }

  if (selectedActivity.value) {
    filtered = filtered.filter((webhook) => {
      if (!webhook.last_triggered && selectedActivity.value === "never") {
        return true;
      }
      if (!webhook.last_triggered) {
        return false;
      }

      const lastTriggered = new Date(webhook.last_triggered);
      const now = new Date();
      const diffInHours = (now.getTime() - lastTriggered.getTime()) / (1000 * 60 * 60);

      if (selectedActivity.value === "active") {
        return diffInHours < 24;
      } else if (selectedActivity.value === "idle") {
        return diffInHours >= 168; // 7 days
      } else if (selectedActivity.value === "never") {
        return !webhook.last_triggered;
      }
      return true;
    });
  }

  return filtered;
});

const activeFiltersCount = computed(() => {
  let count = 0;
  if (searchQuery.value) count++;
  if (selectedServiceType.value) count++;
  if (selectedStatus.value) count++;
  if (selectedActivity.value) count++;
  return count;
});

const hasActiveFilters = computed(() => {
  return activeFiltersCount.value > 0;
});

const getFilteredHealthyPercentage = computed(() => {
  const total = filteredWebhooks.value.length;
  if (total === 0) return 0;
  const healthy = getFilteredHealthyWebhooksCount();
  return Math.round((healthy / total) * 100);
});

const getFilteredHealthyWebhooksCount = () => {
  return filteredWebhooks.value.filter((w) => getWebhookHealth(w) === "healthy").length;
};

const getFilteredWarningWebhooksCount = () => {
  return filteredWebhooks.value.filter((w) =>
    ["warning", "critical"].includes(getWebhookHealth(w))
  ).length;
};

const getFilteredTotalRequestsToday = () => {
  return filteredWebhooks.value.reduce(
    (sum, webhook) => sum + (webhook.trigger_count || 0),
    0
  );
};

// Helper methods
const getServiceTypeClass = (serviceType: string) => {
  const classes: Record<string, string> = {
    github: "bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-300",
    discord: "bg-indigo-100 text-indigo-800 dark:bg-indigo-900/50 dark:text-indigo-300",
    slack: "bg-green-100 text-green-800 dark:bg-green-900/50 dark:text-green-300",
    stripe: "bg-purple-100 text-purple-800 dark:bg-purple-900/50 dark:text-purple-300",
    custom: "bg-blue-100 text-blue-800 dark:bg-blue-900/50 dark:text-blue-300",
  };
  return classes[serviceType] || classes.custom;
};

const getWebhookStatusClass = (webhook: WebhookDisplay) => {
  if (!webhook.enabled) {
    return "bg-gray-500 text-white";
  }

  const health = getWebhookHealth(webhook);
  if (health === "healthy") {
    return "bg-green-500 text-white";
  } else if (health === "warning") {
    return "bg-yellow-500 text-white";
  } else if (health === "critical") {
    return "bg-red-500 text-white";
  } else {
    return "bg-gray-500 text-white";
  }
};

const getWebhookStatusIcon = (webhook: WebhookDisplay) => {
  if (!webhook.enabled) {
    return MdiWebhook;
  }

  const health = getWebhookHealth(webhook);
  if (health === "healthy") {
    return MdiCheckCircle;
  } else if (health === "warning") {
    return MdiAlertCircle;
  } else if (health === "critical") {
    return MdiAlertCircle;
  } else {
    return MdiWebhook;
  }
};

const getWebhookHealth = (webhook: WebhookDisplay) => {
  if (!webhook.enabled) {
    return "offline";
  }

  if (!webhook.last_triggered) {
    return "offline";
  }

  const lastTriggered = new Date(webhook.last_triggered);
  const now = new Date();
  const diffInHours = (now.getTime() - lastTriggered.getTime()) / (1000 * 60 * 60);

  // Mock health logic - in real implementation this would check error rates, response times, etc.
  if (diffInHours < 1) {
    return "healthy";
  } else if (diffInHours < 24) {
    // Use webhook ID for consistent pseudo-random health (not truly random)
    const hash = webhook.id.split("").reduce((a, b) => a + b.charCodeAt(0), 0);
    return hash % 5 === 0 ? "warning" : "healthy"; // 20% chance of warning, but consistent
  } else if (diffInHours < 168) {
    // 7 days
    return "warning";
  } else {
    return "critical";
  }
};

const getSuccessRate = (webhook: WebhookDisplay) => {
  // Mock success rate based on webhook health
  const health = getWebhookHealth(webhook);
  if (health === "healthy") return 98 + Math.floor(Math.random() * 2); // 98-99%
  if (health === "warning") return 85 + Math.floor(Math.random() * 10); // 85-94%
  if (health === "critical") return 60 + Math.floor(Math.random() * 20); // 60-79%
  return 0;
};

const getAverageResponseTime = (webhook: WebhookDisplay) => {
  // Mock average response time based on service type
  const baseTime: Record<string, number> = {
    github: 45,
    discord: 120,
    slack: 80,
    stripe: 200,
    custom: 100,
  };
  const base = baseTime[webhook.service_type ?? webhook.service ?? "custom"] || 100;
  return base + Math.floor(Math.random() * 50); // Add some variance
};

// Methods
const refresh = async () => {
  loading.value = true;
  try {
    await webhookManager.fetchWebhooks();
    mainStore.addNotification({
      title: "Webhook Data Refreshed",
      message: "All webhook data has been updated",
      type: "success",
      autoClose: true,
    });
  } finally {
    loading.value = false;
  }
};

const openCreateModal = () => {
  isEditMode.value = false;
  editingWebhook.value = null;
  showCreateModal.value = true;
};

const closeCreateModal = () => {
  showCreateModal.value = false;
  isEditMode.value = false;
  editingWebhook.value = null;
};

const detailsInitialTab = ref<"analytics" | "logs" | "test" | "config">("analytics");

const openDetailsModal = (
  webhook: WebhookConfig,
  initialTab: "analytics" | "logs" | "test" | "config" = "analytics",
) => {
  selectedWebhook.value = webhook;
  detailsInitialTab.value = initialTab;
  showDetailsModal.value = true;
};

const closeDetailsModal = () => {
  showDetailsModal.value = false;
  selectedWebhook.value = null;
};

const editWebhook = (webhook: WebhookConfig) => {
  isEditMode.value = true;
  editingWebhook.value = webhook;
  showCreateModal.value = true;
};

const editWebhookFromDetails = (webhook: WebhookConfig) => {
  showDetailsModal.value = false;
  setTimeout(() => {
    editWebhook(webhook);
  }, 100);
};

const handleWebhookSubmit = async (webhookData: WebhookSubmission) => {
  try {
    if (isEditMode.value) {
      // Update existing webhook
      const { id, ...updates } = webhookData;
      if (!id) throw new Error("Cannot update a webhook without an id");
      await webhookManager.updateWebhook(id, updates);
      mainStore.addNotification({
        title: "Webhook Updated",
        message: `${webhookData.name} has been updated`,
        type: "success",
        autoClose: true,
      });
    } else {
      // Create new webhook
      await webhookManager.createWebhook(webhookData);
      mainStore.addNotification({
        title: "Webhook Created",
        message: `${webhookData.name} has been created successfully`,
        type: "success",
        autoClose: true,
      });
    }

    closeCreateModal();

    // Refresh the webhook list
    await webhookManager.fetchWebhooks();
  } catch (error: any) {
    mainStore.addNotification({
      title: isEditMode.value ? "Update Failed" : "Creation Failed",
      message:
        error?.message || `Failed to ${isEditMode.value ? "update" : "create"} webhook`,
      type: "error",
      autoClose: true,
    });
  }
};

const testWebhook = (webhook: WebhookConfig) => {
  openDetailsModal(webhook, "test");
};

const clearAllFilters = () => {
  searchQuery.value = "";
  selectedServiceType.value = "";
  selectedStatus.value = "";
  selectedActivity.value = "";

  mainStore.addNotification({
    title: "Filters Cleared",
    message: "All filters have been reset",
    type: "success",
    autoClose: true,
  });
};

const toggleWebhook = (webhook: any) => {
  webhook.enabled = !webhook.enabled;
  mainStore.addNotification({
    title: "Webhook Updated",
    message: `${webhook.name} ${webhook.enabled ? "enabled" : "disabled"}`,
    type: "success",
    autoClose: true,
  });
};

const deleteWebhook = (webhook: any) => {
  if (!confirm(`Are you sure you want to delete "${webhook.name}"?`)) return;

  mockData.value.allWebhooks = mockData.value.allWebhooks.filter(
    (w) => w.id !== webhook.id
  );

  mainStore.addNotification({
    title: "Webhook Deleted",
    message: `${webhook.name} has been deleted`,
    type: "success",
    autoClose: true,
  });
};

const formatDate = (dateString: string) => {
  if (!dateString) return "Never";
  return new Date(dateString).toLocaleString();
};

const toggleSelectAll = () => {
  if (selectedWebhooks.value.length === filteredWebhooks.value.length) {
    selectedWebhooks.value = [];
    selectAll.value = false;
  } else {
    selectedWebhooks.value = filteredWebhooks.value.map((w) => w.id);
    selectAll.value = true;
  }
};

const bulkEnable = () => {
  if (
    !confirm(`Are you sure you want to enable ${selectedWebhooks.value.length} webhooks?`)
  )
    return;

  const count = selectedWebhooks.value.length;
  mockData.value.allWebhooks.forEach((webhook) => {
    if (selectedWebhooks.value.includes(webhook.id)) {
      webhook.enabled = true;
    }
  });
  selectedWebhooks.value = [];
  selectAll.value = false;

  mainStore.addNotification({
    title: "Bulk Operation",
    message: `${count} webhooks enabled`,
    type: "success",
    autoClose: true,
  });
};

const bulkDisable = () => {
  if (
    !confirm(
      `Are you sure you want to disable ${selectedWebhooks.value.length} webhooks?`
    )
  )
    return;

  const count = selectedWebhooks.value.length;
  mockData.value.allWebhooks.forEach((webhook) => {
    if (selectedWebhooks.value.includes(webhook.id)) {
      webhook.enabled = false;
    }
  });
  selectedWebhooks.value = [];
  selectAll.value = false;

  mainStore.addNotification({
    title: "Bulk Operation",
    message: `${count} webhooks disabled`,
    type: "success",
    autoClose: true,
  });
};

const bulkDelete = () => {
  if (
    !confirm(`Are you sure you want to delete ${selectedWebhooks.value.length} webhooks?`)
  )
    return;

  const count = selectedWebhooks.value.length;
  mockData.value.allWebhooks = mockData.value.allWebhooks.filter(
    (w) => !selectedWebhooks.value.includes(w.id)
  );
  selectedWebhooks.value = [];
  selectAll.value = false;

  mainStore.addNotification({
    title: "Bulk Operation",
    message: `${count} webhooks deleted`,
    type: "success",
    autoClose: true,
  });
};

const exportSelected = () => {
  const data = mockData.value.allWebhooks.filter((webhook) =>
    selectedWebhooks.value.includes(webhook.id)
  );
  const csvContent = [
    [
      "ID",
      "Name",
      "Endpoint",
      "Service Type",
      "Status",
      "Last Triggered",
      "Trigger Count",
    ],
    ...data.map((webhook) => [
      webhook.id,
      webhook.name,
      webhook.endpoint,
      webhook.service_type,
      webhook.enabled ? getWebhookHealth(webhook) : "Disabled",
      webhook.last_triggered ? formatDate(webhook.last_triggered) : "Never",
      webhook.trigger_count || 0,
    ]),
  ];

  const csvString = csvContent.map((row) => row.join(",")).join("\n");
  const blob = new Blob([csvString], { type: "text/csv" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `webhooks_${new Date().toISOString().split("T")[0]}.csv`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);

  mainStore.addNotification({
    title: "Export Complete",
    message: `${data.length} webhooks exported to CSV`,
    type: "success",
    autoClose: true,
  });
};

const viewLogs = (webhook: any) => {
  mainStore.addNotification({
    title: "Webhook Logs",
    message: `Opening logs for ${webhook.name}... (Feature coming soon)`,
    type: "info",
    autoClose: true,
  });
};

// Webhook Details Modal
const detailTabs = ref([
  { id: "analytics", label: "Analytics", icon: MdiChartLine },
  { id: "logs", label: "Logs", icon: MdiTextBoxSearchOutline },
  { id: "test", label: "Test", icon: MdiTestTube },
  { id: "config", label: "Configuration", icon: MdiPencil },
]);
const activeDetailTab = ref("analytics");
const logFilter = ref("all");
const testConfig = ref({
  method: "POST",
  contentType: "application/json",
  headers: [] as Array<{ key: string; value: string }>,
  payload: '{"message": "Test webhook payload"}',
});
const testLoading = ref(false);
const testResult = ref<WebhookTestResult | null>(null);
const showSecret = ref(false);

const webhookTemplates = ref({
  github: [
    {
      name: "Basic Push Event",
      payload:
        '{"action": "push", "repository": {"full_name": "owner/repo"}, "ref": "refs/heads/branch", "head_commit": {"id": "commit-id", "message": "commit message"}}',
    },
    {
      name: "Issue Event",
      payload:
        '{"action": "opened", "issue": {"title": "Issue Title", "body": "Issue Description", "number": 123}}',
    },
  ],
  discord: [
    { name: "Message Event", payload: '{"content": "Hello from Discord!"}' },
    {
      name: "Reaction Event",
      payload: '{"reaction": "👍", "user": {"id": "user-id", "username": "username"}}',
    },
  ],
  slack: [
    { name: "Message Event", payload: '{"text": "Hello from Slack!"}' },
    {
      name: "Interactive Message",
      payload:
        '{"type": "interactive_message", "callback_id": "callback-id", "actions": [{"name": "action_name", "value": "action_value"}]}',
    },
  ],
  stripe: [
    {
      name: "Charge Succeeded",
      payload:
        '{"object": "charge", "amount": 1000, "currency": "usd", "status": "succeeded"}',
    },
    {
      name: "Customer Created",
      payload: '{"object": "customer", "id": "customer-id", "email": "user@example.com"}',
    },
  ],
  custom: [
    { name: "Simple GET", payload: "{}" },
    { name: "Simple POST", payload: '{"message": "Hello from Custom!"}' },
  ],
});

const getFilteredLogs = () => {
  let logs = [...(selectedWebhook.value?.logs ?? [])];

  if (logFilter.value === "success") {
    logs = logs.filter((log) => log.status >= 200 && log.status < 300);
  } else if (logFilter.value === "error") {
    logs = logs.filter((log) => log.status >= 400);
  }

  return logs.sort(
    (a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()
  );
};

const refreshLogs = () => {
  if (selectedWebhook.value) {
    selectedWebhook.value.logs = []; // Clear existing logs for refresh
    mainStore.addNotification({
      title: "Logs Refreshed",
      message: `Logs for ${selectedWebhook.value.name} have been refreshed.`,
      type: "success",
      autoClose: true,
    });
  }
};

const formatJSON = (json: string) => {
  try {
    return JSON.stringify(JSON.parse(json), null, 2);
  } catch (e) {
    return json;
  }
};

const copyToClipboard = (text: string) => {
  navigator.clipboard
    .writeText(text)
    .then(() => {
      mainStore.addNotification({
        title: "Copied",
        message: `Webhook URL copied to clipboard!`,
        type: "success",
        autoClose: true,
      });
    })
    .catch(() => {
      mainStore.addNotification({
        title: "Error",
        message: `Failed to copy webhook URL.`,
        type: "error",
        autoClose: true,
      });
    });
};

const addHeader = () => {
  testConfig.value.headers.push({ key: "", value: "" });
};

const removeHeader = (index: number) => {
  testConfig.value.headers.splice(index, 1);
};

const sendTestWebhook = async () => {
  if (!selectedWebhook.value) return;

  testLoading.value = true;
  testResult.value = null;
  const startTime = performance.now();

  try {
    const endpoint = selectedWebhook.value.endpoint_url || selectedWebhook.value.endpoint;
    if (!endpoint) throw new Error("Webhook endpoint is unavailable");
    const response = await fetch(endpoint, {
      method: testConfig.value.method,
      headers: testConfig.value.headers.map((h) => [h.key, h.value] as [string, string]),
      body: testConfig.value.payload,
      signal: AbortSignal.timeout((selectedWebhook.value.timeout ?? 30) * 1000),
    });
    const endTime = performance.now();

    const responseTime = endTime - startTime;
    const status = response.status;
    const responseBody = await response.json();

    const log: WebhookLog = {
      id: Date.now(),
      timestamp: new Date().toISOString(),
      method: testConfig.value.method,
      status: status,
      responseTime: responseTime,
      payload: testConfig.value.payload,
      response: responseBody,
    };

    if (selectedWebhook.value.logs) {
      selectedWebhook.value.logs.unshift(log);
    } else {
      selectedWebhook.value.logs = [log];
    }

    testResult.value = {
      success: status >= 200 && status < 300,
      responseTime: responseTime,
      response: responseBody,
    };

    mainStore.addNotification({
      title: "Test Successful",
      message: `Webhook test successful! Status: ${status}, Response: ${JSON.stringify(
        responseBody
      )}`,
      type: "success",
      autoClose: true,
    });
  } catch (error: any) {
    const endTime = performance.now();
    const responseTime = endTime - startTime;

    const log: WebhookLog = {
      id: Date.now(),
      timestamp: new Date().toISOString(),
      method: testConfig.value.method,
      status: error.code || 500,
      responseTime: responseTime,
      payload: testConfig.value.payload,
      response: error.message || "Unknown error",
    };

    if (selectedWebhook.value.logs) {
      selectedWebhook.value.logs.unshift(log);
    } else {
      selectedWebhook.value.logs = [log];
    }

    testResult.value = {
      success: false,
      responseTime: responseTime,
      response: error.message || "Unknown error",
    };

    mainStore.addNotification({
      title: "Test Failed",
      message: `Webhook test failed: ${error.message || error.code || "Unknown error"}`,
      type: "error",
      autoClose: true,
    });
  } finally {
    testLoading.value = false;
  }
};

const clearTestPayload = () => {
  testConfig.value.payload = '{"message": "Test webhook payload"}';
  testResult.value = null;
};

const loadTemplate = (template: any) => {
  testConfig.value.payload = template.payload;
  testResult.value = null;
};

// Create Webhook Modal
const newWebhook = ref({
  name: "",
  description: "",
  endpoint: "",
  service_type: "",
  enabled: true,
  secret: "",
  requireSignature: false,
  rateLimit: 100,
  timeout: 30,
  maxRetries: 3,
  backoffStrategy: "exponential",
  baseDelay: 1000,
  eventFilters: "",
});
const showNewSecret = ref(false);
const selectedTemplate = ref<{
  type: string;
  name: string;
  description: string;
  icon: Component;
  color: string;
  examplePayload: string;
} | null>(null);
const isFormValid = ref(false);

const serviceTemplates = ref([
  {
    type: "github",
    name: "GitHub",
    description: "Handles GitHub push, pull request, and issue events",
    icon: MdiGithub,
    color: "text-gray-800 dark:text-gray-200",
    examplePayload:
      '{"action": "push", "repository": {"full_name": "owner/repo"}, "ref": "refs/heads/main", "head_commit": {"id": "abc123", "message": "Update README"}}',
  },
  {
    type: "discord",
    name: "Discord",
    description: "Sends messages and reactions to Discord channels",
    icon: MdiDiscord,
    color: "text-blue-500 dark:text-blue-400",
    examplePayload:
      '{"content": "Hello from VortiDeck!", "username": "VortiDeck Bot", "avatar_url": "https://example.com/avatar.png"}',
  },
  {
    type: "slack",
    name: "Slack",
    description: "Sends messages and interactive messages to Slack channels",
    icon: MdiSlack,
    color: "text-green-500 dark:text-green-400",
    examplePayload:
      '{"text": "Hello from VortiDeck!", "channel": "#general", "username": "VortiDeck", "icon_emoji": ":robot_face:"}',
  },
  {
    type: "stripe",
    name: "Stripe",
    description: "Handles Stripe charge, customer, and event notifications",
    icon: MdiCreditCard,
    color: "text-purple-500 dark:text-purple-400",
    examplePayload:
      '{"id": "evt_1A2B3C", "object": "event", "type": "charge.succeeded", "data": {"object": {"id": "ch_1A2B3C", "amount": 2000, "currency": "usd"}}}',
  },
  {
    type: "custom",
    name: "Custom",
    description: "Any HTTP endpoint you can call",
    icon: MdiWebhook,
    color: "text-gray-500 dark:text-gray-400",
    examplePayload:
      '{"message": "Custom webhook payload", "timestamp": "2024-01-15T10:30:00Z", "data": {"key": "value"}}',
  },
]);

const selectServiceTemplate = (service: any) => {
  newWebhook.value.service_type = service.type;
  newWebhook.value.name = service.name + " Webhook";
  newWebhook.value.endpoint =
    service.type + "-webhook-" + Math.random().toString(36).substring(7);
  selectedTemplate.value = service;

  // Set service-specific defaults
  if (service.type === "github") {
    newWebhook.value.requireSignature = true;
    newWebhook.value.eventFilters = "push, pull_request, issues";
  } else if (service.type === "stripe") {
    newWebhook.value.requireSignature = true;
    newWebhook.value.eventFilters = "charge.succeeded, customer.created";
  } else if (service.type === "discord") {
    newWebhook.value.rateLimit = 50;
    newWebhook.value.timeout = 10;
  }

  checkFormValidity();
};

const checkFormValidity = () => {
  isFormValid.value = !!(
    newWebhook.value.name &&
    newWebhook.value.endpoint &&
    newWebhook.value.service_type
  );
};

const copyExamplePayload = () => {
  if (selectedTemplate.value) {
    navigator.clipboard.writeText(selectedTemplate.value.examplePayload).then(() => {
      mainStore.addNotification({
        title: "Copied",
        message: "Example payload copied to clipboard!",
        type: "success",
        autoClose: true,
      });
    });
  }
};

const generateRandomEndpoint = () => {
  const randomString = Math.random().toString(36).substring(7);
  newWebhook.value.endpoint = randomString;
};

const generateSecretKey = () => {
  newWebhook.value.secret = Math.random().toString(36).substring(7);
};

const createWebhook = () => {
  if (
    !newWebhook.value.name ||
    !newWebhook.value.endpoint ||
    !newWebhook.value.service_type
  ) {
    mainStore.addNotification({
      title: "Validation Error",
      message: "Webhook name, endpoint, and service type are required.",
      type: "error",
      autoClose: true,
    });
    return;
  }

  const newId = Date.now().toString();
  const newWebhookData = {
    ...newWebhook.value,
    id: newId,
    enabled: newWebhook.value.enabled,
    trigger_count: 0,
    last_triggered: null,
    logs: [],
    updated_at: new Date().toISOString(),
  };

  mockData.value.allWebhooks.push(newWebhookData);
  showCreateModal.value = false;
  newWebhook.value = {
    name: "",
    description: "",
    endpoint: "",
    service_type: "",
    enabled: true,
    secret: "",
    requireSignature: false,
    rateLimit: 100,
    timeout: 30,
    maxRetries: 3,
    backoffStrategy: "exponential",
    baseDelay: 1000,
    eventFilters: "",
  };
  selectedTemplate.value = null;
  isFormValid.value = false;

  mainStore.addNotification({
    title: "Webhook Created",
    message: `Webhook "${newWebhookData.name}" has been created.`,
    type: "success",
    autoClose: true,
  });
};

// Helper to get full webhook URL
const getFullWebhookUrl = (webhook: any) => {
  if (!serverInfo.value) return webhook.endpoint_url || webhook.endpoint || "";

  const endpoint = webhook.endpoint_url || webhook.endpoint || "";
  return `http://${serverInfo.value.ip}:${serverInfo.value.port}${endpoint}`;
};

// Copy server base URL to clipboard
const copyServerAddress = () => {
  if (!serverInfo.value) return;

  const baseUrl = `http://${serverInfo.value.ip}:${serverInfo.value.port}`;
  navigator.clipboard
    .writeText(baseUrl)
    .then(() => {
      mainStore.addNotification({
        title: "Server Address Copied",
        message: `${baseUrl} copied to clipboard`,
        type: "success",
        autoClose: true,
      });
    })
    .catch(() => {
      mainStore.addNotification({
        title: "Copy Failed",
        message: "Failed to copy server address",
        type: "error",
        autoClose: true,
      });
    });
};

// Load webhooks on mount
onMounted(async () => {
  // Fetch server info first
  try {
    serverInfo.value = await invoke<{ ip: string; port: number } | null>(
      "get_server_info_new"
    );
    console.log("🌐 Server info loaded:", serverInfo.value);
  } catch (error) {
    console.error("❌ Failed to fetch server info:", error);
  }

  await webhookManager.fetchWebhooks();

  // Listen for webhook_event to update counts in real-time
  const unlisten = await listen("webhook_event", async (event) => {
    console.log("🔔 Webhook event received:", event.payload);

    // Refresh webhooks to get updated counts
    await webhookManager.fetchWebhooks();
  });

  // Cleanup listener on unmount
  onUnmounted(() => {
    unlisten();
  });
});
</script>

<style scoped>
/* Slide down transition for filters - ultra smooth */
.slide-down-enter-active,
.slide-down-leave-active {
  transition: all 0.4s cubic-bezier(0.23, 1, 0.32, 1);
  transform-origin: top;
}

.slide-down-enter-from,
.slide-down-leave-to {
  opacity: 0;
  transform: scaleY(0.8);
}

.slide-down-enter-to,
.slide-down-leave-from {
  opacity: 1;
  transform: scaleY(1);
}

/* Animation delays for background elements */
.animation-delay-1000 {
  animation-delay: 1s;
}

.animation-delay-2000 {
  animation-delay: 2s;
}

/* Gradient text effect */
.bg-clip-text {
  background-clip: text;
  -webkit-background-clip: text;
}

/* Custom scrollbar */
::-webkit-scrollbar {
  width: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: linear-gradient(180deg, #3b82f6, #8b5cf6);
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: linear-gradient(180deg, #2563eb, #7c3aed);
}

/* Enhanced hover effects */
.group:hover .group-hover\:scale-110 {
  transform: scale(1.1);
}

/* Backdrop blur support */
.backdrop-blur-sm {
  backdrop-filter: blur(4px);
}

/* Progress bar animation */
@keyframes progress {
  from {
    width: 0%;
  }
}

.animate-progress {
  animation: progress 0.7s ease-out;
}

/* Fade transition for modal backdrop */
.fade-enter-active {
  animation: fade-in 0.3s;
}

.fade-leave-active {
  animation: fade-out 0.3s;
}

@keyframes fade-in {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

@keyframes fade-out {
  from {
    opacity: 1;
  }
  to {
    opacity: 0;
  }
}

/* Jump transition for modal content */
.jump-enter-active {
  animation: jump-in 0.3s;
}

.jump-leave-active {
  animation: jump-out 0.3s;
}

@keyframes jump-in {
  0% {
    transform: translateY(-100vh);
    opacity: 0;
  }
  100% {
    transform: translateY(0);
    opacity: 1;
  }
}

@keyframes jump-out {
  0% {
    transform: translateY(0);
    opacity: 1;
  }
  100% {
    transform: translateY(100vh);
    opacity: 0;
  }
}
</style>
