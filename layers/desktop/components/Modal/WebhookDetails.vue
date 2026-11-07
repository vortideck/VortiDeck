<template lang="pug">
BaseModal(:is-open="isOpen" @close="requestClose")
  div(class="vd-modal-card relative w-full max-w-[50rem] max-h-[90vh] overflow-y-auto")
        // Header
        div(class="flex items-center gap-x-3 p-6 border-b border-themeBorder2")
          div(class="w-10 h-10 rounded-lg flex items-center justify-center" :class="getWebhookStatusClass(webhook)")
            component(:is="getWebhookStatusIcon(webhook)" class="w-5 h-5")
          div(class="flex-1")
            h3(class="text-2xl font-semibold text-themeText") {{ webhook?.name }}
            p(class="text-sm text-themeText2") {{ webhook?.endpoint }}
          button(@click="requestClose" class="vd-modal-close-btn")
            span(class="sr-only") Close
            svg(class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")

        // Tab Navigation
        div(class="border-b border-themeBorder2")
          nav(class="flex space-x-8 px-6" aria-label="Tabs")
            button(
              v-for="tab in detailTabs" 
              :key="tab.id"
              @click="activeTab = tab.id"
              :class="activeTab === tab.id ? 'webhook-tab-btn--active' : 'webhook-tab-btn--idle'"
              class="webhook-tab-btn flex items-center gap-2 py-4 px-1 border-b-2 font-medium text-sm transition-colors"
            )
              component(:is="tab.icon" class="w-4 h-4")
              span {{ tab.label }}

        // Tab Content
        div(class="p-6")
          // Analytics Tab
          div(v-if="activeTab === 'analytics'" class="space-y-6")
            div(class="grid grid-cols-1 md:grid-cols-3 gap-6")
              div(class="bg-themeBackground3 rounded-lg p-6 border border-themeBorder2")
                div(class="flex items-center justify-between")
                  div
                    div(class="text-2xl font-semibold text-themeText") {{ webhook?.trigger_count || 0 }}
                    div(class="text-sm text-themeText2") Total Requests
                  div(class="w-10 h-10 rounded-lg flex items-center justify-center bg-themeBackground2 border border-themeBorder2")
                    MdiChartBox(class="w-5 h-5 text-themeText2")
              
              div(class="bg-themeBackground3 rounded-lg p-6 border border-themeBorder2")
                div(class="flex items-center justify-between")
                  div
                    div(class="text-2xl font-semibold text-themeText") {{ getSuccessRate(webhook) }}%
                    div(class="text-sm text-themeText2") Success Rate
                  div(class="w-10 h-10 rounded-lg flex items-center justify-center bg-themeBackground2 border border-themeBorder2")
                    MdiCheckCircle(class="w-5 h-5 text-themeText2")
              
              div(class="bg-themeBackground3 rounded-lg p-6 border border-themeBorder2")
                div(class="flex items-center justify-between")
                  div
                    div(class="text-2xl font-semibold text-themeText") {{ getAverageResponseTime(webhook) }}ms
                    div(class="text-sm text-themeText2") Avg Response Time
                  div(class="w-10 h-10 rounded-lg flex items-center justify-center bg-themeBackground2 border border-themeBorder2")
                    MdiSpeedometer(class="w-5 h-5 text-themeText2")

            div(class="bg-themeBackground3 rounded-lg p-6 border border-themeBorder2")
              h4(class="text-lg font-medium text-themeText mb-4") Recent Activity
              div(class="space-y-2")
                div(class="text-sm text-themeText2")
                  span Last triggered: {{ formatDate(webhook?.last_triggered) }}
                div(class="text-sm text-themeText2")
                  span Status: {{ getWebhookHealth(webhook) }}
                div(class="text-sm text-themeText2")
                  span Service: {{ webhook?.service_type }}

          // Logs Tab
          div(v-if="activeTab === 'logs'" class="space-y-6")
            div(class="flex items-center justify-between")
              h4(class="text-lg font-medium text-themeText") Request Logs
              div(class="flex items-center gap-3")
                select(v-model="logFilter" class="vd-modal-select px-3 py-2")
                  option(value="all") All Requests
                  option(value="success") Success Only
                  option(value="error") Errors Only
                button(@click="refreshLogs" class="vd-modal-primary-btn px-4 py-2")
                  MdiRefresh(class="w-4 h-4")
            
            div(class="bg-themeBackground3 rounded-lg border border-themeBorder2 max-h-96 overflow-y-auto")
              div(v-if="!getFilteredLogs().length" class="text-center py-8 text-themeText3")
                MdiFileDocumentOutline(class="w-12 h-12 mx-auto mb-4 text-themeText2")
                p No logs available for this webhook yet.
              
              div(v-else class="divide-y divide-themeBorder2")
                div(v-for="log in getFilteredLogs()" :key="log.id" class="p-4 hover:bg-themeBackground2 transition-colors")
                  div(class="flex items-start justify-between")
                    div(class="flex-1")
                      div(class="flex items-center gap-3 mb-2")
                        span(class="text-sm font-medium text-themeText2") {{ log.method }}
                        span(
                          :class="log.status >= 200 && log.status < 300 ? 'text-green-600 bg-green-100 dark:bg-green-900/30' : 'text-red-600 bg-red-100 dark:bg-red-900/30'"
                          class="px-2 py-1 rounded-full text-xs font-medium"
                        ) {{ log.status }}
                        span(class="text-xs text-themeText3") {{ log.responseTime }}ms
                      div(class="text-xs text-themeText3") {{ formatDate(log.timestamp) }}
                    div(class="flex items-center gap-2")
                      button(@click="viewLogDetails(log)" class="text-themeText2 hover:text-themeText text-sm transition-colors")
                        MdiEye(class="w-4 h-4")

          // Test Tab
          div(v-if="activeTab === 'test'" class="space-y-6")
            div(class="rounded-lg border border-themeWarningBorder bg-themeWarningBgSubtle px-4 py-3 text-sm text-themeText")
              strong Real effects are enabled.
              span(class="ml-1 text-themeText2") A test request runs the configured flow and can control connected services or the system. Access is limited to this editor session and expires automatically.
            div(class="grid grid-cols-1 md:grid-cols-2 gap-6")
              div(class="space-y-4")
                h4(class="text-lg font-medium text-themeText") Test Configuration
                
                div(class="space-y-3")
                  div
                    label(class="block text-sm font-medium text-themeText mb-2") HTTP Method
                    select(v-model="testConfig.method" class="vd-modal-select w-full px-3 py-2")
                      option(value="GET") GET
                      option(value="POST") POST
                      option(value="PUT") PUT
                      option(value="DELETE") DELETE
                  
                  div
                    label(class="block text-sm font-medium text-themeText mb-2") Content Type
                    select(v-model="testConfig.contentType" class="vd-modal-select w-full px-3 py-2")
                      option(value="application/json") application/json
                      option(value="application/x-www-form-urlencoded") application/x-www-form-urlencoded
                      option(value="text/plain") text/plain
                  
                  div
                    label(class="block text-sm font-medium text-themeText mb-2") Custom Headers
                    div(class="space-y-2")
                      div(v-for="(header, index) in testConfig.headers" :key="index" class="flex gap-2")
                        input(v-model="header.key" placeholder="Header name" class="vd-modal-input flex-1 px-3 py-2")
                        input(v-model="header.value" placeholder="Header value" class="vd-modal-input flex-1 px-3 py-2")
                        button(@click="removeHeader(index)" class="px-2 py-2 text-themeText2 hover:text-themeText transition-colors")
                          MdiClose(class="w-4 h-4")
                      button(@click="addHeader" class="webhook-link-btn")
                        MdiPlus(class="w-4 h-4 inline mr-1")
                        span Add Header

                div
                  label(class="block text-sm font-medium text-themeText mb-2") Request Payload
                  div(class="relative")
                    textarea(
                      v-model="testConfig.payload" 
                      rows="8"
                      placeholder="Enter JSON payload..."
                      class="vd-modal-textarea w-full px-3 py-2 font-mono text-sm"
                    )
                    button(@click="formatPayload" class="absolute top-2 right-2 px-2 py-1 text-xs rounded-md border border-themeBorder2 bg-themeBackground2 text-themeText2 hover:bg-themeBackground3 hover:text-themeText transition-colors")
                      | Format JSON

                div(class="flex gap-2")
                  button(@click="sendTestWebhook" :disabled="testLoading" class="vd-modal-primary-btn flex-1 px-4 py-2 disabled:opacity-50")
                    MdiRocket(v-if="!testLoading" class="w-4 h-4 mr-2")
                    MdiLoading(v-if="testLoading" class="w-4 h-4 mr-2 animate-spin")
                    span {{ testLoading ? 'Testing...' : 'Send Test' }}
                  button(@click="clearTestPayload" class="px-4 py-2 rounded-lg border border-themeBorder2 bg-themeBackground2 text-themeText2 hover:bg-themeBackground3 hover:text-themeText transition-colors")
                    | Clear

                // Test mode timer
                div(v-if="testModeExpiresIn > 0" class="webhook-timer-note flex items-center gap-2 px-3 py-2 rounded-lg text-sm")
                  MdiTimer(class="w-4 h-4")
                  span Test mode active: {{ testModeExpiresIn }}s remaining

              div(class="space-y-4")
                h4(class="text-lg font-medium text-themeText") Test Result
                
                div(v-if="testResult" class="bg-themeBackground3 rounded-lg p-4 border border-themeBorder2")
                  div(class="flex items-center gap-3 mb-3")
                    MdiCheckCircle(v-if="testResult.success" class="w-6 h-6 text-green-600")
                    MdiAlertCircle(v-if="!testResult.success" class="w-6 h-6 text-red-600")
                    span(class="font-medium text-themeText") {{ testResult.success ? 'Success' : 'Failed' }}
                    span(class="text-sm text-themeText2") {{ testResult.responseTime }}ms
                  
                  div(class="space-y-3")
                    div
                      label(class="block text-sm font-medium text-themeText mb-1") Response
                      pre(class="bg-themeBackground2 p-3 rounded-lg text-sm text-themeText overflow-x-auto border border-themeBorder2") {{ formatJSON(testResult.response) }}
                
                div(v-else class="bg-themeBackground3 rounded-lg p-8 border border-themeBorder2 text-center")
                  MdiTestTube(class="w-12 h-12 mx-auto mb-4 text-themeText2")
                  p(class="text-themeText2") No test results yet. Send a test request to see the response here.

          // Configuration Tab
          div(v-if="activeTab === 'config'" class="space-y-6")
            div(class="bg-themeBackground3 rounded-lg p-6 border border-themeBorder2")
              h4(class="text-lg font-medium text-themeText mb-4") Webhook Configuration
              
              div(class="grid grid-cols-1 md:grid-cols-2 gap-6")
                div(class="space-y-4")
                  div
                    label(class="block text-sm font-medium text-themeText mb-2") Endpoint URL
                    div(class="flex gap-2")
                      input(:value="getFullWebhookUrl(webhook)" readonly class="vd-modal-input flex-1 px-3 py-2 bg-themeBackground2/75 text-themeText")
                      button(@click="copyToClipboard(getFullWebhookUrl(webhook))" class="vd-modal-primary-btn px-3 py-2")
                        MdiContentCopy(class="w-4 h-4")
                  
                  div
                    label(class="block text-sm font-medium text-themeText mb-2") Inbound signing secret
                    div(class="vd-modal-input flex items-center gap-2 px-3 py-2 bg-themeBackground2/75 text-themeText2")
                      MdiShieldCheck(class="w-4 h-4 text-themeSuccess")
                      span Stored securely and never returned by read operations
                  
                  div
                    label(class="block text-sm font-medium text-themeText mb-2") Service Type
                    input(v-model="webhook.service_type" readonly class="vd-modal-input w-full px-3 py-2 bg-themeBackground2/75 text-themeText")
                
                div(class="space-y-4")
                  div
                    label(class="block text-sm font-medium text-themeText mb-2") Rate Limit
                    input(v-model="webhook.rateLimit" readonly class="vd-modal-input w-full px-3 py-2 bg-themeBackground2/75 text-themeText")
                  
                  div
                    label(class="block text-sm font-medium text-themeText mb-2") Timeout
                    input(v-model="webhook.timeout" readonly class="vd-modal-input w-full px-3 py-2 bg-themeBackground2/75 text-themeText")
                  
                  div
                    label(class="block text-sm font-medium text-themeText mb-2") Max Retries
                    input(v-model="webhook.maxRetries" readonly class="vd-modal-input w-full px-3 py-2 bg-themeBackground2/75 text-themeText")

              div(class="flex justify-end mt-6")
                button(@click="$emit('edit', webhook)" class="vd-modal-primary-btn px-6 py-2")
                  MdiPencil(class="w-4 h-4 mr-2")
                  span Edit Configuration

        // Footer
        div(class="vd-modal-footer flex items-center justify-end gap-3 p-6")
          button(@click="requestClose" class="px-4 py-2 text-themeText2 hover:text-themeText transition-colors")
            Close
</template>

<script setup lang="ts">
import BaseModal from "./BaseModal.vue";
import { invoke } from "@tauri-apps/api/core";
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import MdiCheckCircle from "~icons/mdi/check-circle";
import MdiAlertCircle from "~icons/mdi/alert-circle";
import MdiChartBox from "~icons/mdi/chart-box";
import MdiSpeedometer from "~icons/mdi/speedometer";
import MdiRefresh from "~icons/mdi/refresh";
import MdiEye from "~icons/mdi/eye";
import MdiShieldCheck from "~icons/mdi/shield-check";
import MdiContentCopy from "~icons/mdi/content-copy";
import MdiPencil from "~icons/mdi/pencil";
import MdiPlus from "~icons/mdi/plus";
import MdiClose from "~icons/mdi/close";
import MdiRocket from "~icons/mdi/rocket";
import MdiLoading from "~icons/mdi/loading";
import MdiTestTube from "~icons/mdi/test-tube";
import MdiFileDocumentOutline from "~icons/mdi/file-document-outline";
import MdiWebhook from "~icons/mdi/webhook";
import MdiChartLine from "~icons/mdi/chart-line";
import MdiTimer from "~icons/mdi/timer";
import MdiTextBoxSearchOutline from "~icons/mdi/text-box-search-outline";
import MdiCog from "~icons/mdi/cog";
import type {
  WebhookConfig,
  WebhookLog,
  WebhookTestResult,
} from "@desktop/composables/useWebhookManager";

interface Props {
  isOpen: boolean;
  webhook: WebhookConfig | null;
  initialTab?: "analytics" | "logs" | "test" | "config";
}

interface Emits {
  (e: "close"): void;
  (e: "edit", webhook: WebhookConfig): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

const activeTab = ref("analytics");
const logFilter = ref("all");
const testLoading = ref(false);
const testResult = ref<WebhookTestResult | null>(null);
const testModeExpiresIn = ref(0);
let testModeTimer: ReturnType<typeof setInterval> | null = null;

const endTestSession = async () => {
  const webhookId = props.webhook?.id;
  testModeExpiresIn.value = 0;
  if (testModeTimer) {
    clearInterval(testModeTimer);
    testModeTimer = null;
  }
  if (!webhookId) return;
  try {
    await invoke("deactivate_test_mode", { webhookId });
  } catch (error) {
    console.warn("Failed to close webhook test session", error);
  }
};

const requestClose = () => {
  void endTestSession();
  emit("close");
};

// Server info for constructing full webhook URLs
const serverInfo = ref<{ ip: string; port: number } | null>(null);

const testConfig = ref({
  method: "POST",
  contentType: "application/json",
  headers: [] as Array<{ key: string; value: string }>,
  payload:
    '{\n  "message": "Test webhook payload",\n  "timestamp": "' +
    new Date().toISOString() +
    '"\n}',
});

const detailTabs = ref([
  { id: "analytics", label: "Analytics", icon: MdiChartLine },
  { id: "logs", label: "Logs", icon: MdiTextBoxSearchOutline },
  { id: "test", label: "Test", icon: MdiTestTube },
  { id: "config", label: "Configuration", icon: MdiCog },
]);

const getWebhookHealth = (webhook: WebhookConfig | null) => {
  if (!webhook?.enabled) return "offline";
  if (!webhook?.last_triggered) return "offline";

  const lastTriggered = new Date(webhook.last_triggered);
  const now = new Date();
  const diffInHours = (now.getTime() - lastTriggered.getTime()) / (1000 * 60 * 60);

  if (diffInHours < 1) return "healthy";
  if (diffInHours < 24) return "healthy";
  if (diffInHours < 168) return "warning";
  return "critical";
};

const getWebhookStatusClass = (webhook: WebhookConfig | null) => {
  if (!webhook?.enabled) return "bg-gray-500 text-white";

  const health = getWebhookHealth(webhook);
  if (health === "healthy") return "bg-green-500 text-white";
  if (health === "warning") return "bg-yellow-500 text-white";
  if (health === "critical") return "bg-red-500 text-white";
  return "bg-gray-500 text-white";
};

const getWebhookStatusIcon = (webhook: WebhookConfig | null) => {
  if (!webhook?.enabled) return MdiWebhook;

  const health = getWebhookHealth(webhook);
  if (health === "healthy") return MdiCheckCircle;
  if (health === "warning") return MdiAlertCircle;
  if (health === "critical") return MdiAlertCircle;
  return MdiWebhook;
};

const getSuccessRate = (webhook: WebhookConfig | null) => {
  const health = getWebhookHealth(webhook);
  if (health === "healthy") return 98 + Math.floor(Math.random() * 2);
  if (health === "warning") return 85 + Math.floor(Math.random() * 10);
  if (health === "critical") return 60 + Math.floor(Math.random() * 20);
  return 0;
};

const getAverageResponseTime = (webhook: WebhookConfig | null) => {
  const baseTime: Record<string, number> = {
    github: 45,
    discord: 120,
    slack: 80,
    stripe: 200,
    custom: 100,
  };
  const base = baseTime[webhook?.service_type ?? webhook?.service ?? "custom"] || 100;
  return base + Math.floor(Math.random() * 50);
};

const formatDate = (dateString?: string) => {
  if (!dateString) return "Never";
  return new Date(dateString).toLocaleString();
};

const getFilteredLogs = () => {
  const logs = [...(props.webhook?.logs || [])];

  if (logFilter.value === "success") {
    return logs.filter((log) => log.status >= 200 && log.status < 300);
  }
  if (logFilter.value === "error") {
    return logs.filter((log) => log.status >= 400);
  }

  return logs.sort(
    (a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()
  );
};

const refreshLogs = () => {
  // Simulate log refresh
  console.log("Refreshing logs for webhook:", props.webhook?.name);
};

const viewLogDetails = (log: WebhookLog) => {
  console.log("View log details:", log);
};

const formatPayload = () => {
  try {
    const parsed = JSON.parse(testConfig.value.payload);
    testConfig.value.payload = JSON.stringify(parsed, null, 2);
  } catch (e) {
    console.error("Invalid JSON format");
  }
};

const formatJSON = (json: unknown): string => {
  try {
    if (typeof json === "string") {
      return JSON.stringify(JSON.parse(json), null, 2);
    }
    return JSON.stringify(json, null, 2);
  } catch (e) {
    return String(json);
  }
};

const addHeader = () => {
  testConfig.value.headers.push({ key: "", value: "" });
};

const removeHeader = (index: number) => {
  testConfig.value.headers.splice(index, 1);
};

const clearTestPayload = () => {
  testConfig.value.payload =
    '{\n  "message": "Test webhook payload",\n  "timestamp": "' +
    new Date().toISOString() +
    '"\n}';
  testResult.value = null;
};

const sendTestWebhook = async () => {
  if (!props.webhook) return;

  testLoading.value = true;
  testResult.value = null;

  try {
    // Use the real send_test_webhook command (auto-activates test mode)
    const result = await invoke<{
      success: boolean;
      responseTime: number;
      response?: unknown;
      error?: string;
      testModeActive?: boolean;
      testModeExpiresIn?: number;
    }>("send_test_webhook", {
      webhookId: props.webhook.id,
      payload: testConfig.value.payload,
      method: testConfig.value.method,
      contentType: testConfig.value.contentType,
      headers: Object.fromEntries(
        testConfig.value.headers
          .map(({ key, value }) => [key.trim(), value] as const)
          .filter(([key]) => key.length > 0),
      ),
    });

    testResult.value = {
      success: result.success,
      responseTime: result.responseTime,
      response: result.response || result.error || "No response",
    };

    // Start countdown timer if test mode is active
    if (result.testModeActive && result.testModeExpiresIn) {
      startTestModeCountdown(result.testModeExpiresIn);
    }
  } catch (error) {
    testResult.value = {
      success: false,
      responseTime: 0,
      response: error instanceof Error ? error.message : "Network error",
    };
  } finally {
    testLoading.value = false;
  }
};

const startTestModeCountdown = (seconds: number) => {
  // Clear existing timer
  if (testModeTimer) {
    clearInterval(testModeTimer);
  }

  testModeExpiresIn.value = seconds;

  testModeTimer = setInterval(() => {
    testModeExpiresIn.value--;
    if (testModeExpiresIn.value <= 0) {
      if (testModeTimer !== null) clearInterval(testModeTimer);
      testModeTimer = null;
    }
  }, 1000);
};

// Helper to get full webhook URL
const getFullWebhookUrl = (webhook: any) => {
  if (!serverInfo.value) return webhook?.endpoint_url || webhook?.endpoint || '';

  const endpoint = webhook?.endpoint_url || webhook?.endpoint || '';
  return `http://${serverInfo.value.ip}:${serverInfo.value.port}${endpoint}`;
};

// Fetch server info on mount
onMounted(async () => {
  try {
    serverInfo.value = await invoke<{ ip: string; port: number } | null>('get_server_info_new');
    console.log('🌐 Server info loaded in webhook details:', serverInfo.value);
  } catch (error) {
    console.error('❌ Failed to fetch server info:', error);
  }
});

// Cleanup timer on unmount
onUnmounted(() => {
  if (testModeTimer) {
    clearInterval(testModeTimer);
  }
  void endTestSession();
});

const copyToClipboard = (text: string) => {
  navigator.clipboard.writeText(text).then(() => {
    console.log("Copied to clipboard:", text);
  });
};

// Reset when webhook changes
watch(
  () => props.webhook,
  (newWebhook) => {
    if (newWebhook) {
      activeTab.value = props.initialTab || "analytics";
      testResult.value = null;
    }
  }
);

watch(
  () => props.isOpen,
  (isOpen, wasOpen) => {
    if (isOpen) activeTab.value = props.initialTab || "analytics";
    else if (wasOpen) void endTestSession();
  },
);
</script>

<style scoped lang="scss">
@use "./modalTheme.scss" as *;

.webhook-tab-btn {
  color: hsl(var(--text2) / 1);
  border-bottom-color: transparent;
}

.webhook-tab-btn--idle:hover {
  color: hsl(var(--text) / 1);
  border-bottom-color: hsl(var(--border2) / 0.9);
}

.webhook-tab-btn--active {
  color: hsl(var(--editor-accent, 20 96% 55%) / 1);
  border-bottom-color: hsl(var(--editor-accent, 20 96% 55%) / 1);
}

.webhook-link-btn {
  color: hsl(var(--editor-accent, 20 96% 55%) / 1);
  font-size: 0.8rem;
  font-weight: 600;
  transition: color 0.2s ease;
}

.webhook-link-btn:hover {
  color: hsl(var(--editor-accent-2, 20 95% 62%) / 1);
}

.webhook-timer-note {
  border: 1px solid hsl(var(--editor-accent, 20 96% 55%) / 0.3);
  background: hsl(var(--editor-accent, 20 96% 55%) / 0.08);
  color: hsl(var(--editor-accent, 20 96% 55%) / 1);
}
</style>
