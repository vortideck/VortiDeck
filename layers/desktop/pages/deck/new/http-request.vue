<template lang="pug">
div(class="http-action-page flex flex-col overflow-hidden")

  // Main content area
  div(class="flex-1 min-h-0 w-full max-w-[46rem] mx-auto px-6 pb-6 overflow-y-auto")

    DeckEditorActionPageHeader(
      class="http-action-header"
      eyebrow="Utilities"
      :title="`${requestId ? 'Update' : 'Create'} HTTP request`"
      description="Configure a reusable API call or webhook for buttons and workflows."
    )
      template(v-slot:icon)
        GlobeIcon(class="w-6 h-6")

    // Form with cards
    div(class="http-action-stack")

      // Basic Settings Card
      div(class="bg-themeBackground2 rounded-xl p-5 border border-themeBorder2")
        h3(class="text-lg font-semibold text-themeText mb-4 flex items-center gap-2")
          AdjustmentsHorizontalIcon(class="w-5 h-5 text-themeButton")
          | Basic Settings

        div(class="space-y-4")
          // Name
          div
            label(class="block text-sm font-medium text-themeText2 mb-2") Request Name
            input(
              v-model="requestName"
              class="w-full px-4 py-3 text-base bg-themeBackground border border-themeBorder2 rounded-lg text-themeText focus:outline-none focus:ring-2 focus:ring-themeButton transition-all"
              placeholder="Discord Webhook"
            )

          // Method and URL
          div(class="grid grid-cols-4 gap-4")
            div
              label(class="block text-sm font-medium text-themeText2 mb-2") Method
              select(
                v-model="method"
                class="w-full px-4 py-3 text-base bg-themeBackground border border-themeBorder2 rounded-lg text-themeText focus:outline-none focus:ring-2 focus:ring-themeButton transition-all"
              )
                option(value="GET") GET
                option(value="POST") POST
                option(value="PUT") PUT
                option(value="PATCH") PATCH
                option(value="DELETE") DELETE
                option(value="HEAD") HEAD
                option(value="OPTIONS") OPTIONS

            div(class="col-span-3")
              label(class="block text-sm font-medium text-themeText2 mb-2") URL
              input(
                v-model="url"
                class="w-full px-4 py-3 text-base bg-themeBackground border border-themeBorder2 rounded-lg text-themeText focus:outline-none focus:ring-2 focus:ring-themeButton transition-all"
                placeholder="https://api.example.com/endpoint"
              )

      // Headers Card
      div(class="bg-themeBackground2 rounded-xl p-5 border border-themeBorder2")
        h3(class="text-lg font-semibold text-themeText mb-4 flex items-center gap-2")
          QueueListIcon(class="w-5 h-5 text-themeButton")
          | Headers

        div(class="space-y-3")
          div(v-for="(header, index) in headers" :key="index" class="flex gap-3")
            input(
              v-model="header.key"
              class="flex-1 px-4 py-3 text-base bg-themeBackground border border-themeBorder2 rounded-lg text-themeText focus:outline-none focus:ring-2 focus:ring-themeButton transition-all"
              placeholder="Header Name (e.g., Content-Type)"
            )
            input(
              v-model="header.value"
              :type="header.secret || isProtectedHttpHeader(header.key, header.value) ? 'password' : 'text'"
              class="flex-1 px-4 py-3 text-base bg-themeBackground border border-themeBorder2 rounded-lg text-themeText focus:outline-none focus:ring-2 focus:ring-themeButton transition-all"
              placeholder="Header Value (e.g., application/json)"
            )
            label(class="flex items-center gap-1 text-xs text-themeText2")
              input(v-model="header.secret" type="checkbox" :disabled="isProtectedHttpHeader(header.key, header.value)")
              | Secret
            button(
              @click="removeHeader(index)"
              class="px-4 py-3 bg-themeError hover:brightness-90 text-white rounded-lg transition-all"
            )
              XMarkIcon(class="w-5 h-5")

          button(
            @click="addHeader"
            class="w-full px-4 py-3 bg-themeBackground3 hover:bg-themeBorder2 text-themeText2 rounded-lg transition-all flex items-center justify-center gap-2 border border-themeBorder2"
          )
            PlusIcon(class="w-5 h-5")
            span Add Header
          p(class="text-sm text-themeText3") Authorization, cookies and API keys are stored securely. Mark other private headers as Secret.

      // Body Card (only for POST/PUT/PATCH)
      div(v-if="methodHasBody" class="bg-themeBackground2 rounded-xl p-5 border border-themeBorder2")
        h3(class="text-lg font-semibold text-themeText mb-4 flex items-center gap-2")
          CodeBracketIcon(class="w-5 h-5 text-themeButton")
          | Request Body

        div
          label(class="block text-sm font-medium text-themeText2 mb-2") Body (JSON)
          textarea(
            v-model="body"
            class="w-full px-4 py-3 text-base bg-themeBackground border border-themeBorder2 rounded-lg text-themeText focus:outline-none focus:ring-2 focus:ring-themeButton resize-none transition-all font-mono"
            placeholder='{\n  "key": "value"\n}'
            rows="6"
          )
          p(class="text-sm text-themeText3 mt-1") Enter valid JSON for the request body

      // Advanced Settings Card (Collapsible)
      div(class="bg-themeBackground2 rounded-xl border border-themeBorder2")
        button(
          @click="showAdvanced = !showAdvanced"
          class="w-full p-6 flex items-center justify-between text-left hover:bg-themeBackground3/30 transition-colors rounded-xl"
        )
          div(class="flex items-center gap-2")
            Cog6ToothIcon(class="w-5 h-5 text-themeButton")
            h3(class="text-lg font-semibold text-themeText") Advanced Settings
          ChevronIcon(:class="showAdvanced ? 'rotate-180' : ''" class="w-5 h-5 text-themeText3 transition-transform")

        div(v-if="showAdvanced" class="px-6 pb-6 border-t border-themeBorder2")
          div(class="mt-6")
            label(class="block text-sm font-medium text-themeText2 mb-2") Timeout (milliseconds)
            input(
              v-model.number="timeoutMs"
              type="number"
              class="w-full px-4 py-3 text-base bg-themeBackground border border-themeBorder2 rounded-lg text-themeText focus:outline-none focus:ring-2 focus:ring-themeButton transition-all"
              placeholder="30000"
            )

          div(class="mt-4")
            label(class="block text-sm font-medium text-themeText2 mb-2") Accepted response statuses
            input(v-model="acceptedStatuses" class="w-full px-4 py-3 bg-themeBackground border border-themeBorder2 rounded-lg text-themeText" placeholder="2xx, 404")
            p(class="text-sm text-themeText3 mt-1") Default: 2xx. Add expected codes, or use * to accept any response.

    p(v-if="formError" role="alert" class="text-themeError text-sm mt-3") {{ formError }}

    ActionInputContractEditor(class="mt-4" :reference="requestId ? { kind: 'http_request', id: String(requestId) } : null")

    // Test Results (compact)
    div(v-if="testResponse" class="bg-themeBackground3/30 rounded p-3 text-sm")
      div(class="flex items-center justify-between mb-2")
        div(class="flex items-center gap-2")
          div(:class="testSuccess ? 'bg-themeSuccess' : 'bg-themeError'" class="w-2 h-2 rounded-full")
          span(:class="testSuccess ? 'text-themeSuccess' : 'text-themeError'" class="text-xs font-medium")
            | {{ testSuccess ? 'Success' : 'Failed' }}
        button(@click="testResponse = null" class="text-themeText3 hover:text-themeText")
          XMarkIcon(class="w-4 h-4")

      div
        button(@click="openResponseModal" class="text-themeButton hover:opacity-80 text-sm px-2 py-1 bg-themeButton/10 rounded") View Full Response
        div(
          class="mt-2 p-3 bg-themeBackground border border-themeBorder2 text-themeText rounded text-sm font-mono max-h-20 overflow-hidden"
        )
          | {{ JSON.stringify(testResponse).substring(0, 120) }}{{ JSON.stringify(testResponse).length > 120 ? '...' : '' }}

  // Fixed Footer
  div(class="flex-shrink-0 bg-themeBackground2/95 backdrop-blur-sm border-t border-themeBorder2 p-6")
    div(class="w-full max-w-[46rem] mx-auto flex justify-between items-center")
      div(class="flex items-center gap-3")
        button(
          @click="router.go(-1)"
          class="px-6 py-3 bg-themeBackground2 hover:bg-themeBackground text-themeText2 rounded-lg transition-all border border-themeBorder2"
        ) Cancel

        // Test button
        div(v-if="canTest" class="flex items-center gap-2")
          button(
            @click="testRequest"
            :disabled="isTestRunning"
            :class="isTestRunning ? 'bg-themeButton/70' : 'bg-themeSuccess hover:brightness-110'"
            class="px-4 py-3 text-white rounded-lg text-sm transition-colors flex items-center gap-2"
          )
            PlayIcon(v-if="!isTestRunning" class="w-4 h-4")
            Spinner(v-else class="w-4 h-4")
            span {{ isTestRunning ? 'Testing...' : 'Test Request' }}

          button(
            v-if="testResponse"
            @click="openResponseModal"
            class="px-3 py-3 bg-themeButton hover:brightness-110 text-white rounded-lg text-sm transition-colors flex items-center gap-1"
          )
            DocumentTextIcon(class="w-4 h-4")
            span Response

      div(class="flex gap-3")
        button(
          @click="resetRequest"
          class="px-6 py-3 bg-themeBackground3 hover:bg-themeBorder2 text-themeText2 rounded-lg transition-all border border-themeBorder2"
        ) Reset
        button(
          @click="requestId ? updateRequest() : saveRequest()"
          :disabled="!canSave || isSaving"
          :class="canSave && !isSaving ? 'bg-themeButton shadow-lg hover:shadow-xl hover:brightness-105' : 'bg-themeBorder2 cursor-not-allowed'"
          class="px-8 py-3 text-white rounded-lg transition-all font-semibold flex items-center gap-2"
        )
          div(v-if="isSaving" class="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin")
          span {{ isSaving ? 'Saving...' : (requestId ? 'Update Request' : 'Save Request') }}

  // Response Modal
  BaseModal(:is-open="showResponseModal" :z-index="100000" @close="closeResponseModal")
    div(class="vd-modal-card w-[90vw] h-[90vh] m-4 flex flex-col")
      // Header
      div(class="rounded-t-xl p-6 border-b border-themeBorder2 bg-themeBackground2 flex-shrink-0")
        div(class="flex items-center justify-between")
          div(class="flex items-center gap-4")
            div(class="w-8 h-8 rounded-lg border border-themeBorder2 bg-themeBackground3 flex items-center justify-center")
              DocumentTextIcon(class="w-4 h-4 text-themeText")
            h3(class="text-xl font-bold text-themeText") HTTP Response
          button(@click="closeResponseModal" class="text-themeText3 hover:text-themeText transition-colors")
            XMarkIcon(class="w-6 h-6")

      // Content
      div(class="flex-1 overflow-hidden p-6")
        pre(
          class="p-6 bg-themeBackground border border-themeBorder2 text-themeText rounded-xl text-base font-mono h-full overflow-auto whitespace-pre-wrap leading-relaxed"
        )
          | {{ JSON.stringify(testResponse, null, 2) }}

      // Footer
      div(class="bg-themeBackground3/30 rounded-b-xl p-4 border-t border-themeBorder2 flex justify-between items-center flex-shrink-0")
        div(class="flex items-center gap-4 text-sm text-themeText3")
          span {{ JSON.stringify(testResponse).length }} characters
          span HTTP Response Data
        button(@click="copyResponse" class="px-4 py-2 bg-themeButton hover:brightness-110 text-white rounded-lg transition-all shadow-md hover:shadow-lg") Copy to Clipboard

</template>

<script setup lang="ts">
import BaseModal from "../../../components/Modal/BaseModal.vue";
import ActionInputContractEditor from '../../../components/ActionInputContractEditor.vue';
import GlobeIcon from "~icons/heroicons/globe-alt";
import PlayIcon from "~icons/heroicons/play";
import DocumentTextIcon from "~icons/heroicons/document-text";
import XMarkIcon from "~icons/heroicons/x-mark";
import ChevronIcon from "~icons/heroicons/chevron-down";
import PlusIcon from "~icons/heroicons/plus";
import Spinner from "~icons/svg-spinners/ring-resize";
import AdjustmentsHorizontalIcon from "~icons/heroicons/adjustments-horizontal";
import QueueListIcon from "~icons/heroicons/queue-list";
import CodeBracketIcon from "~icons/heroicons/code-bracket";
import Cog6ToothIcon from "~icons/heroicons/cog-6-tooth";

import { invoke } from "@tauri-apps/api/core";
import { acceptsHttpStatus, isProtectedHttpHeader, parseAcceptedHttpStatuses } from "@base/utils/httpRequestSettings";

interface HeaderEntry {
  key: string;
  value: string;
  secret?: boolean;
}

interface StoredHttpRequest {
  id: number;
  name: string;
  method: string;
  url: string;
  headers: string | null;
  body: string | null;
  timeout_ms: number | null;
  created_at: string;
  accepted_status_codes: string | null;
}

const queryNumber = (value: unknown): number | null => {
  const raw = Array.isArray(value) ? value[0] : value;
  if (typeof raw !== "string" || raw.trim() === "") return null;
  const parsed = Number(raw);
  return Number.isInteger(parsed) ? parsed : null;
};

const route = useRoute();
const router = useRouter();

// Route parameters - using actionId to match Submenu component pattern
const requestId = ref(queryNumber(route.query.actionId));
const copyFromId = ref(queryNumber(route.query.copyFrom));

// Form data
const requestName = ref("New HTTP Request");
const method = ref("GET");
const url = ref("");
const headers = ref<HeaderEntry[]>([
  { key: "Content-Type", value: "application/json" }
]);
const body = ref("");
const timeoutMs = ref(30000);
const acceptedStatuses = ref("2xx");
const formError = ref("");

// UI state
const showAdvanced = ref(false);
const isTestRunning = ref(false);
const isSaving = ref(false);
const testResponse = ref<unknown>(null);
const testSuccess = ref(false);
const showResponseModal = ref(false);

// Computed
const methodHasBody = computed(() => {
  return !["GET", "HEAD"].includes(method.value);
});

const canSave = computed(() => {
  return Boolean(requestName.value.trim() && url.value.trim());
});

const canTest = computed(() => {
  return canSave.value;
});

// Header management
const addHeader = () => {
  headers.value.push({ key: "", value: "" });
};

const removeHeader = (index: number) => {
  headers.value.splice(index, 1);
};

const buildHeaders = (): Record<string, string> => {
  const result: Record<string, string> = {};
  for (const header of headers.value) {
    const key = header.key.trim();
    const value = header.value;
    if (!key && !value) continue;
    if (!key) throw new Error("Every header value needs a header name.");
    if (Object.keys(result).some(existing => existing.toLowerCase() === key.toLowerCase())) {
      throw new Error(`Duplicate header: ${key}`);
    }
    result[key] = value;
  }
  return result;
};

// Test functionality
const testRequest = async () => {
  if (isTestRunning.value) return;

  isTestRunning.value = true;
  testResponse.value = null;
  testSuccess.value = false;

  try {
    // Build headers object
    const headersObj = buildHeaders();

    // Build request data
    const requestData = {
      method: method.value,
      url: url.value,
      headers: JSON.stringify(headersObj),
      body: methodHasBody.value && body.value ? body.value : null,
      timeoutMs: timeoutMs.value,
      secretHeaders: headers.value.filter(header => header.secret).map(header => header.key.trim()),
    };

    const statusPolicy = parseAcceptedHttpStatuses(acceptedStatuses.value);
    const response = await invoke<{ status: number }>("test_http_request", requestData);

    testResponse.value = response;
    testSuccess.value = acceptsHttpStatus(response.status, statusPolicy);
  } catch (error) {
    console.error("Test request failed:", error);
    testResponse.value = { error: error instanceof Error ? error.message : String(error) };
    testSuccess.value = false;
  } finally {
    isTestRunning.value = false;
  }
};

// Save/Update
const saveRequest = async () => {
  isSaving.value = true;
  formError.value = "";

  try {
    // Build headers object
    const headersObj = buildHeaders();

    await invoke("save_http_request", {
      name: requestName.value,
      method: method.value,
      url: url.value,
      headers: JSON.stringify(headersObj),
      body: methodHasBody.value && body.value ? body.value : null,
      timeoutMs: timeoutMs.value,
      acceptedStatusCodes: parseAcceptedHttpStatuses(acceptedStatuses.value),
      secretHeaders: headers.value.filter(header => header.secret).map(header => header.key.trim()),
    });

    await router.push("/deck");
  } catch (error) {
    formError.value = String(error);
  } finally {
    isSaving.value = false;
  }
};

const updateRequest = async () => {
  isSaving.value = true;
  formError.value = "";

  try {
    // Build headers object
    const headersObj = buildHeaders();

    await invoke("update_http_request", {
      requestId: requestId.value,
      name: requestName.value,
      method: method.value,
      url: url.value,
      headers: JSON.stringify(headersObj),
      body: methodHasBody.value && body.value ? body.value : null,
      timeoutMs: timeoutMs.value,
      acceptedStatusCodes: parseAcceptedHttpStatuses(acceptedStatuses.value),
      secretHeaders: headers.value.filter(header => header.secret).map(header => header.key.trim()),
    });

    await router.push("/deck");
  } catch (error) {
    formError.value = String(error);
  } finally {
    isSaving.value = false;
  }
};

const resetRequest = () => {
  requestName.value = "New HTTP Request";
  method.value = "GET";
  url.value = "";
  headers.value = [{ key: "Content-Type", value: "application/json" }];
  body.value = "";
  timeoutMs.value = 30000;
  acceptedStatuses.value = "2xx";
  formError.value = "";
  testResponse.value = null;
  testSuccess.value = false;
};

// Modal functions
const openResponseModal = () => {
  showResponseModal.value = true;
};

const closeResponseModal = () => {
  showResponseModal.value = false;
};

const copyResponse = async () => {
  try {
    await navigator.clipboard.writeText(JSON.stringify(testResponse.value, null, 2));
    console.log("Response copied to clipboard");
  } catch (error) {
    console.error("Failed to copy:", error);
  }
};

// Load data on mount (if editing)
onMounted(async () => {
  // Check if we're in copy mode or edit mode
  const idToLoad = copyFromId.value || requestId.value;
  if (idToLoad) {
    console.log("🔍 Loading HTTP request with ID:", idToLoad, copyFromId.value ? "(copy mode)" : "(edit mode)");
    try {
      const requestData = await invoke<StoredHttpRequest>("get_http_request_by_id", {
        requestId: idToLoad,
      });



      if (requestData) {
        // If copying, append " (Copy)" to the name
        requestName.value = copyFromId.value ? `${requestData.name} (Copy)` : (requestData.name || "HTTP Request");
        method.value = requestData.method || "GET";
        url.value = requestData.url || "";
        timeoutMs.value = requestData.timeout_ms || 30000;

        acceptedStatuses.value = requestData.accepted_status_codes ? JSON.parse(requestData.accepted_status_codes).join(", ") : "2xx";

        // Parse headers
        if (requestData.headers) {
          try {
            const headersObj = JSON.parse(requestData.headers);
            headers.value = Object.entries(headersObj).map(([key, value]) => ({
              key,
              value: String(value),
              secret: isProtectedHttpHeader(key, String(value))
            }));
          } catch (e) {
            headers.value = [{ key: "Content-Type", value: "application/json" }];
          }
        }

        // Parse body
        if (requestData.body) {
          try {
            // If it's a JSON string, pretty print it
            const parsed = JSON.parse(requestData.body);
            body.value = JSON.stringify(parsed, null, 2);
          } catch (e) {
            // If not JSON, use as-is
            body.value = requestData.body;
          }
        }

        // If copying, clear the requestId so it creates a new entry
        if (copyFromId.value) {
          requestId.value = null;
        }
      }
    } catch (error) {
      console.error("Error loading HTTP request:", error);
    }
  }
});
</script>

<style scoped lang="scss">
@use "../../../components/Modal/modalTheme.scss" as *;

:deep(.http-action-header) {
  border-radius: 0.9rem 0.9rem 0 0;
  box-shadow: none;
}

/*
 * The shared deck-item layout owns an 8px vertical and 32px horizontal gutter.
 * This full-surface editor consumes that gutter itself so its footer reaches the
 * window edges, while retaining the layout's intentional title-bar clearance.
 */
.http-action-page {
  width: calc(100% + 4rem);
  height: calc(100% + 1rem);
  min-height: 0;
  margin: -0.5rem -2rem;
}

.http-action-stack > :deep(*) {
  margin-top: 0;
  border-top: 0;
  border-radius: 0;
}

.http-action-stack > :deep(*:last-child) {
  border-radius: 0 0 0.9rem 0.9rem;
}

input[type="number"]::-webkit-outer-spin-button,
input[type="number"]::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}

input[type="number"] {
  -moz-appearance: textfield;
}

</style>
