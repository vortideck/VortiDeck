<template lang="pug">
BaseModal(:is-open="isOpen" @close="$emit('close')")
  div(class="vd-modal-card relative w-full max-w-[40rem] max-h-[90vh] overflow-y-auto")
        // Header
        div(class="flex items-center gap-x-3 p-6 border-b border-themeBorder2")
          MdiWebhook(class="w-7 h-7 text-themeText")
          h3(class="text-2xl font-semibold text-themeText") {{ isEdit ? 'Edit Webhook' : 'Create New Webhook' }}
          button(@click="$emit('close')" class="vd-modal-close-btn ml-auto")
            span(class="sr-only") Close
            svg(class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")

        // Body
        div(class="p-6 space-y-6")
          // Basic Information
          div(class="space-y-4")
            div(class="grid grid-cols-2 gap-4")
              div(class="space-y-2")
                label(class="block text-sm font-medium text-themeText") Webhook Name *
                input(
                  v-model="formData.name"
                  placeholder="Enter webhook name"
                  class="vd-modal-input w-full px-3 py-2"
                  @input="validateForm"
                )

              div(class="space-y-2")
                label(class="block text-sm font-medium text-themeText") Service Type *
                Multiselect(
                  v-model="selectedTemplate"
                  :options="serviceTemplates"
                  :searchable="true"
                  :allow-empty="false"
                  :show-labels="false"
                  track-by="type"
                  label="name"
                  placeholder="Select service type"
                  class="custom-multiselect"
                  @update:modelValue="selectServiceTemplate"
                )
                  template(#singleLabel="{ option }")
                    div(class="flex items-center gap-2")
                      component(:is="option.icon" :class="option.color" class="w-5 h-5")
                      span {{ option.name }}
                  template(#option="{ option }")
                    div(class="flex items-center gap-3 py-1")
                      component(:is="option.icon" :class="option.color" class="w-5 h-5")
                      div
                        div(class="font-medium") {{ option.name }}
                        div(class="text-xs text-themeText2") {{ option.description }}

            div(class="space-y-2")
              label(class="block text-sm font-medium text-themeText") Description
              textarea(
                v-model="formData.description" 
                placeholder="Optional description"
                rows="3"
                class="vd-modal-textarea w-full px-3 py-2"
              )

          // Security Settings
          div(class="space-y-4")
            h4(class="text-lg font-medium text-themeText") Security Settings
            
            div(class="space-y-2")
              label(class="block text-sm font-medium text-themeText") Secret Key
              div(class="flex gap-2")
                input(
                  v-model="formData.secret" 
                  :type="showSecret ? 'text' : 'password'"
                  placeholder="Enter secret key"
                  class="vd-modal-input flex-1 px-3 py-2"
                )
                button(
                  @click="showSecret = !showSecret"
                  class="px-3 py-2 bg-themeBackground3 border border-themeBorder2 text-themeText2 rounded-lg hover:bg-themeBackground2 transition-colors"
                )
                  MdiEye(v-if="!showSecret" class="w-4 h-4")
                  MdiEyeOff(v-if="showSecret" class="w-4 h-4")
                button(
                  @click="generateSecretKey"
                  class="px-3 py-2 bg-themeBackground3 border border-themeBorder2 text-themeText rounded-lg hover:bg-themeBackground2 transition-colors"
                )
                  MdiKey(class="w-4 h-4")

            div(class="flex items-center gap-2")
              input(
                id="require-signature"
                type="checkbox" 
                v-model="formData.requireSignature"
                class="w-4 h-4 rounded border-themeBorder2 bg-themeBackground3 text-themeText focus:ring-2 focus:ring-blue-500/50"
              )
              label(for="require-signature" class="text-sm text-themeText2") Require HMAC signature validation

          // Configuration
          div(class="space-y-4")
            h4(class="text-lg font-medium text-themeText") Configuration
            
            div(class="grid grid-cols-2 gap-4")
              div(class="space-y-2")
                label(class="block text-sm font-medium text-themeText") Rate Limit (per minute)
                input(
                  v-model.number="formData.rateLimit" 
                  type="number"
                  min="1"
                  max="1000"
                  class="vd-modal-input w-full px-3 py-2"
                )

              div(class="space-y-2")
                label(class="block text-sm font-medium text-themeText") Timeout (seconds)
                input(
                  v-model.number="formData.timeout" 
                  type="number"
                  min="1"
                  max="300"
                  class="vd-modal-input w-full px-3 py-2"
                )

            div(class="grid grid-cols-2 gap-4")
              div(class="space-y-2")
                label(class="block text-sm font-medium text-themeText") Max Retries
                input(
                  v-model.number="formData.maxRetries" 
                  type="number"
                  min="0"
                  max="10"
                  class="vd-modal-input w-full px-3 py-2"
                )

              div(class="space-y-2")
                label(class="block text-sm font-medium text-themeText") Backoff Strategy
                select(
                  v-model="formData.backoffStrategy" 
                  class="vd-modal-select w-full px-3 py-2"
                )
                  option(value="linear") Linear
                  option(value="exponential") Exponential

            div(class="flex items-center gap-2")
              input(
                id="webhook-enabled"
                type="checkbox"
                v-model="formData.enabled"
                class="w-4 h-4 rounded border-themeBorder2 bg-themeBackground3 text-themeText focus:ring-2 focus:ring-blue-500/50"
              )
              label(for="webhook-enabled" class="text-sm text-themeText2") Enable webhook immediately

          // Phase 1: Test & Production URLs
          div(class="space-y-4")
            h4(class="text-lg font-medium text-themeText") Webhook URLs (Test & Production)

            div(class="space-y-2")
              label(class="block text-sm font-medium text-themeText")
                | Test URL
                span(class="text-xs text-themeText2 ml-2") (Active for 120s when testing)
              input(
                v-model="formData.test_url"
                placeholder="/webhook-test/{id} (auto-generated)"
                readonly
                class="vd-modal-input w-full px-3 py-2 bg-themeBackground3/70 text-themeText2 cursor-not-allowed opacity-90"
              )

            div(class="space-y-2")
              label(class="block text-sm font-medium text-themeText") Production URL
              input(
                v-model="formData.production_url"
                placeholder="/webhook/{id} (auto-generated)"
                readonly
                class="vd-modal-input w-full px-3 py-2 bg-themeBackground3/70 text-themeText2 cursor-not-allowed opacity-90"
              )

          // Phase 2: Response Mode
          div(class="space-y-4")
            h4(class="text-lg font-medium text-themeText") Response Behavior

            div(class="space-y-2")
              label(class="block text-sm font-medium text-themeText") Response Mode
              select(
                v-model="formData.response_mode"
                class="vd-modal-select w-full px-3 py-2"
              )
                option(value="onReceived") Immediate (200 OK when received)
                option(value="lastNode") Last Node Output (flow result)
                option(value="responseNode") Custom Response (RespondToWebhook node)

              div(class="text-xs text-themeText2 mt-1")
                span(v-if="formData.response_mode === 'onReceived'") Returns 200 OK immediately when webhook is received
                span(v-if="formData.response_mode === 'lastNode'") Returns the output of the flow's last executed node
                span(v-if="formData.response_mode === 'responseNode'") Returns custom response from RespondToWebhook node in flow

          // Phase 3: HTTP Methods
          div(class="space-y-4")
            h4(class="text-lg font-medium text-themeText") Allowed HTTP Methods

            div(class="flex flex-wrap gap-3")
              div(
                v-for="method in ['GET', 'POST', 'PUT', 'PATCH', 'DELETE']"
                :key="method"
                class="flex items-center gap-2"
              )
                input(
                  :id="`method-${method}`"
                  type="checkbox"
                  :value="method"
                  v-model="formData.allowed_methods"
                  class="w-4 h-4 rounded border-themeBorder2 bg-themeBackground3 text-themeText focus:ring-2 focus:ring-blue-500/50"
                )
                label(:for="`method-${method}`" class="text-sm text-themeText2") {{ method }}

          // Phase 3: Authentication Config (Optional)
          div(class="space-y-4")
            h4(class="text-lg font-medium text-themeText2")
              | Authentication
              span(class="text-xs text-themeText2 ml-2 font-normal") (Optional)

            div(class="space-y-2")
              label(class="block text-sm font-medium text-themeText") Auth Config (JSON)
              textarea(
                v-model="formData.auth_config"
                placeholder='{"type": "bearer", "token": "your-token"}'
                rows="4"
                class="vd-modal-textarea w-full px-3 py-2 font-mono text-sm"
              )
              div(class="text-xs text-themeText2") JSON configuration for authenticating outbound requests

          // Flow Assignment
          div(class="space-y-4")
            h4(class="text-lg font-medium text-themeText")
              | Flow Assignment
              span(class="text-xs text-themeText2 ml-2 font-normal") (Optional)

            div(class="space-y-2")
              label(class="block text-sm font-medium text-themeText") Execute Flow
              select(
                v-model="formData.assigned_flow_id"
                class="vd-modal-select w-full px-3 py-2"
                @change="onFlowSelected"
              )
                option(value="") No flow assigned
                option(
                  v-for="flow in availableFlows"
                  :key="flow.id"
                  :value="flow.id"
                ) {{ flow.name }}

              div(class="text-xs text-themeText2 mt-1")
                span(v-if="!formData.assigned_flow_id") Webhook will trigger without executing a flow
                span(v-else) This flow will execute when webhook receives a request

            // Flow trigger details (if flow is selected)
            div(v-if="formData.assigned_flow_id" class="p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg")
              div(class="flex items-start gap-2")
                MdiInformation(class="w-5 h-5 text-blue-600 dark:text-blue-400 flex-shrink-0 mt-0.5")
                div(class="text-sm text-blue-800 dark:text-blue-200")
                  div(class="font-medium mb-1") Flow Execution
                  div Webhook payload will be passed to the flow as input data
                  div(class="mt-1") Use the response_mode setting to control what the webhook returns

          // External Service Registration (Future: Auto-register webhooks with external platforms)
          div(v-if="formData.service_type && formData.service_type !== 'custom'" class="space-y-4")
            h4(class="text-lg font-medium text-themeText")
              | External Service Registration
              span(class="text-xs text-themeText2 ml-2 font-normal") (Coming Soon)

            div(class="p-4 bg-themeBackground3/50 border border-themeBorder2 rounded-lg")
              div(class="flex items-start gap-3")
                MdiInformation(class="w-6 h-6 text-themeText2 flex-shrink-0 mt-0.5")
                div(class="flex-1")
                  div(class="font-medium text-themeText mb-2") Automatic Webhook Registration
                  div(class="text-sm text-themeText2")
                    | In a future update, VortiDeck will be able to automatically register your webhook URL with external services like GitHub, Discord, and Slack, saving you the manual setup step.

        // Footer
        div(class="vd-modal-footer flex items-center justify-end gap-3 p-6")
          button(
            @click="$emit('close')" 
            class="px-4 py-2 text-themeText2 hover:text-themeText transition-colors"
          ) Cancel
          button(
            @click="submitForm"
            :disabled="!isFormValid"
            class="vd-modal-primary-btn px-6 py-2 disabled:opacity-45 disabled:cursor-not-allowed"
          ) {{ isEdit ? 'Update' : 'Create' }} Webhook
</template>

<script setup lang="ts">
import BaseModal from "./BaseModal.vue";
import { ref, watch, type Component } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { WebhookConfig, WebhookSubmission } from "@desktop/composables/useWebhookManager";
import Multiselect from "vue-multiselect";
import "vue-multiselect/dist/vue-multiselect.css";
import MdiWebhook from "~icons/mdi/webhook";
import MdiEye from "~icons/mdi/eye";
import MdiEyeOff from "~icons/mdi/eye-off";
import MdiKey from "~icons/mdi/key";
import MdiGithub from "~icons/mdi/github";
import MdiDiscord from "~icons/mdi/discord";
import MdiSlack from "~icons/mdi/slack";
import MdiCreditCard from "~icons/mdi/credit-card";
import MdiInformation from "~icons/mdi/information";

interface Props {
  isOpen: boolean;
  isEdit?: boolean;
  webhook?: WebhookConfig | null;
}

interface Emits {
  (e: "close"): void;
  (e: "submit", webhook: WebhookSubmission): void;
}

interface WebhookFormData {
  name: string;
  description: string;
  endpoint: string;
  service_type: string;
  enabled: boolean;
  secret: string;
  test_url: string;
  production_url: string;
  response_mode: string;
  allowed_methods: string[];
  auth_config: string;
  assigned_flow_id: string;
  requireSignature: boolean;
  rateLimit: number;
  timeout: number;
}

interface ServiceTemplate {
  type: string;
  name: string;
  description: string;
  icon: Component;
  color: string;
}

interface FlowSummary {
  id: string;
  name: string;
}

const emptyForm = (): WebhookFormData => ({
  name: "",
  description: "",
  endpoint: "",
  service_type: "",
  enabled: true,
  secret: "",
  test_url: "",
  production_url: "",
  response_mode: "onReceived",
  allowed_methods: ["POST"],
  auth_config: "",
  assigned_flow_id: "",
  requireSignature: false,
  rateLimit: 100,
  timeout: 30,
});

const props = withDefaults(defineProps<Props>(), {
  isEdit: false,
  webhook: null,
});

const emit = defineEmits<Emits>();

const showSecret = ref(false);
const selectedTemplate = ref<ServiceTemplate | null>(null);
const isFormValid = ref(false);

const formData = ref<WebhookFormData>(emptyForm());

// Available flows for assignment
const availableFlows = ref<FlowSummary[]>([]);

const serviceTemplates = ref<ServiceTemplate[]>([
  {
    type: "github",
    name: "GitHub",
    description: "GitHub webhooks for repos",
    icon: MdiGithub,
    color: "text-gray-800 dark:text-gray-200",
  },
  {
    type: "discord",
    name: "Discord",
    description: "Discord bot webhooks",
    icon: MdiDiscord,
    color: "text-blue-500 dark:text-blue-400",
  },
  {
    type: "slack",
    name: "Slack",
    description: "Slack integration webhooks",
    icon: MdiSlack,
    color: "text-green-500 dark:text-green-400",
  },
  {
    type: "stripe",
    name: "Stripe",
    description: "Payment webhooks",
    icon: MdiCreditCard,
    color: "text-purple-500 dark:text-purple-400",
  },
  {
    type: "custom",
    name: "Custom",
    description: "Custom HTTP webhooks",
    icon: MdiWebhook,
    color: "text-gray-500 dark:text-gray-400",
  },
]);

// Watch for webhook prop changes (for edit mode)
watch(
  () => props.webhook,
  (newWebhook) => {
    if (newWebhook && props.isEdit) {
      // Map backend field names to form field names
      formData.value.name = newWebhook.name || "";
      formData.value.description = newWebhook.description || "";
      formData.value.endpoint = newWebhook.target_url || ""; // Backend: target_url -> Form: endpoint
      formData.value.service_type = newWebhook.service || ""; // Backend: service -> Form: service_type
      formData.value.enabled =
        newWebhook.enabled !== undefined ? newWebhook.enabled : true;
      formData.value.secret = newWebhook.inbound_secret || ""; // Backend: inbound_secret -> Form: secret

      // Phase 1-3 fields
      formData.value.test_url = newWebhook.test_url || "";
      formData.value.production_url = newWebhook.production_url || "";
      formData.value.response_mode = newWebhook.response_mode || "onReceived";

      // Parse JSON string to array for allowed_methods
      try {
        formData.value.allowed_methods = newWebhook.allowed_methods
          ? JSON.parse(newWebhook.allowed_methods)
          : ["POST"];
      } catch {
        formData.value.allowed_methods = ["POST"];
      }

      formData.value.auth_config = newWebhook.auth_config || "";

      // Parse Action enum back to flow_id
      if (newWebhook.action) {
        try {
          const action = JSON.parse(newWebhook.action);
          formData.value.assigned_flow_id = action.FlowId || "";
        } catch {
          formData.value.assigned_flow_id = "";
        }
      } else {
        formData.value.assigned_flow_id = "";
      }

      selectedTemplate.value = serviceTemplates.value.find(
        (t) => t.type === newWebhook.service
      ) ?? null;
      validateForm();
    }
  },
  { immediate: true }
);

// Reset form when modal opens/closes
watch(
  () => props.isOpen,
  (isOpen) => {
    if (!isOpen) {
      resetForm();
    } else {
      // Load flows when modal opens
      loadFlows();
      if (!props.isEdit) {
        resetForm();
      }
    }
  }
);

const resetForm = () => {
  formData.value = emptyForm();
  selectedTemplate.value = null;
  isFormValid.value = false;
  showSecret.value = false;
};

const selectServiceTemplate = (service: ServiceTemplate | null) => {
  if (!service) return;

  formData.value.service_type = service.type;
  selectedTemplate.value = service;

  // Set defaults based on service type
  if (service.type === "github") {
    formData.value.requireSignature = true;
    formData.value.rateLimit = 100;
  } else if (service.type === "stripe") {
    formData.value.requireSignature = true;
    formData.value.rateLimit = 200;
  } else if (service.type === "discord") {
    formData.value.rateLimit = 50;
    formData.value.timeout = 10;
  }

  validateForm();
};

const validateForm = () => {
  // Only name and service_type are required
  // endpoint is optional (for sending webhooks)
  isFormValid.value = !!(formData.value.name.trim() && formData.value.service_type);
};

const generateRandomEndpoint = () => {
  const randomString = Math.random().toString(36).substring(7);
  formData.value.endpoint = `https://api.example.com/webhook/${randomString}`;
  validateForm();
};

const generateSecretKey = () => {
  const randomKey =
    Math.random().toString(36).substring(2, 15) +
    Math.random().toString(36).substring(2, 15);
  formData.value.secret = randomKey;
};

// Load available flows
const loadFlows = async () => {
  try {
    availableFlows.value = await invoke<FlowSummary[]>("list_flows");
  } catch (error) {
    console.error("Failed to load flows:", error);
    availableFlows.value = [];
  }
};

// Handler for flow selection change
const onFlowSelected = () => {
  // Can add additional logic here if needed
  console.log("Flow selected:", formData.value.assigned_flow_id);
};

// Get service icon component
const getServiceIcon = (serviceType: string) => {
  const icons: Record<string, Component> = {
    github: MdiGithub,
    discord: MdiDiscord,
    slack: MdiSlack,
    stripe: MdiCreditCard,
  };
  return icons[serviceType] || MdiWebhook;
};

// Get service color class
const getServiceColor = (serviceType: string) => {
  const colors: Record<string, string> = {
    github: "text-gray-800 dark:text-gray-200",
    discord: "text-indigo-600 dark:text-indigo-400",
    slack: "text-purple-600 dark:text-purple-400",
    stripe: "text-blue-600 dark:text-blue-400",
  };
  return colors[serviceType] || "text-gray-600";
};

const submitForm = () => {
  if (!isFormValid.value) return;

  const editMetadata = props.isEdit && props.webhook
    ? {
        id: props.webhook.id,
        trigger_count: props.webhook.trigger_count || 0,
        last_triggered: props.webhook.last_triggered || null,
      }
    : {};

  // Map old field names to new backend field names
  const webhookData = {
    // Map deprecated field names to correct ones
    name: formData.value.name,
    description: formData.value.description,
    service: formData.value.service_type, // Map service_type -> service
    target_url: formData.value.endpoint, // Map endpoint -> target_url
    enabled: formData.value.enabled,
    inbound_secret: formData.value.secret, // Map secret -> inbound_secret

    // Phase 1-3 fields
    test_url: formData.value.test_url,
    production_url: formData.value.production_url,
    response_mode: formData.value.response_mode,
    allowed_methods: JSON.stringify(formData.value.allowed_methods), // Convert array to JSON string
    auth_config: formData.value.auth_config || null,

    // Action field (flow assignment)
    action: formData.value.assigned_flow_id
      ? JSON.stringify({ FlowId: formData.value.assigned_flow_id })
      : null,

    // Keep edit-specific fields if editing
    ...editMetadata,

    updated_at: new Date().toISOString(),
  };

  emit("submit", webhookData);
  emit("close");
};
</script>

<style lang="scss" scoped>
@use "./modalTheme.scss" as *;

// Vue Multiselect custom styling
.custom-multiselect {
  ::-webkit-scrollbar {
    display: block;
  }

  :deep(.multiselect) {
    background: hsl(var(--background2));
    border: 1px solid hsl(var(--border2));
    border-radius: 0.5rem;
    color: hsl(var(--text));
  }

  :deep(.multiselect__tags) {
    background: hsl(var(--background3));
    border: 1px solid hsl(var(--border2) / 0.9);
    border-radius: 0.5rem;
    padding: 8px 40px 0 8px;
    min-height: 42px;
  }

  :deep(.multiselect__single) {
    background: transparent;
    color: hsl(var(--text));
    padding: 0;
    margin: 0;
    line-height: 1.5;
    font-size: 0.875rem;
  }

  :deep(.multiselect__input) {
    background: transparent;
    color: hsl(var(--text));
    border: none;
    padding: 0;
    margin: 0;
  }

  :deep(.multiselect__input::placeholder) {
    color: hsl(var(--text3));
  }

  :deep(.multiselect__placeholder) {
    color: hsl(var(--text3));
    padding: 0;
    margin: 0;
  }

  :deep(.multiselect__select) {
    background: transparent;
    border-left: 1px solid hsl(var(--border2) / 0.85);
    height: 42px;
    right: 1px;
    top: 1px;
    padding: 4px 8px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  :deep(.multiselect__select::before) {
    border-color: hsl(var(--text2)) transparent transparent;
    border-style: solid;
    border-width: 5px 5px 0 5px;
    content: "";
    display: block;
    height: 0;
    width: 0;
    position: relative;
    top: 0;
    right: 0;
    margin: 0;
  }

  :deep(.multiselect--active .multiselect__select::before) {
    transform: rotate(180deg);
  }

  :deep(.multiselect__content-wrapper) {
    background: hsl(var(--background2));
    border: 1px solid hsl(var(--border2) / 0.95);
    border-top: 1px solid hsl(var(--border2) / 0.85);
    border-radius: 0 0 0.5rem 0.5rem;
    margin-top: -1px;
    max-height: 240px;
    overflow-y: auto;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }

  :deep(.multiselect__content) {
    background: hsl(var(--background2));
  }

  :deep(.multiselect__option) {
    background: hsl(var(--background2));
    color: hsl(var(--text));
    padding: 10px 12px;
    min-height: auto;
    line-height: 1.4;

    &:hover {
      background: hsl(var(--background3));
      color: hsl(var(--text));
    }

    &.multiselect__option--selected {
      background: hsl(var(--primary));
      color: white;
      font-weight: 600;

      &:hover {
        background: hsl(var(--primary));
        color: white;
      }
    }

    &.multiselect__option--highlight {
      background: hsl(var(--background3));
      color: hsl(var(--text));

      &.multiselect__option--selected {
        background: hsl(var(--primary));
        color: white;
      }
    }
  }

  :deep(.multiselect--active) {
    .multiselect__tags {
      border-color: rgba(59, 130, 246, 0.5);
      box-shadow: 0 0 0 1px rgba(59, 130, 246, 0.2);
    }
  }
}
</style>
