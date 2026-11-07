<template lang="pug">
div(class="capability-page")
  section(class="capability-shell" aria-labelledby="action-editor-title")
    header(class="capability-header")
      div
        p(class="capability-eyebrow") Integration action
        h1(id="action-editor-title" class="capability-title") {{ editorTitle }}
        p(class="capability-description") Configure one reusable action for decks, flows, menus, and stream commands.
      div(class="flex items-center gap-2")
        span(class="status-pill" :class="isConnected ? 'status-pill--ready' : 'status-pill--offline'")
          | {{ isConnected ? 'Connected' : 'Offline' }}
        button(v-if="integrationId === 'discord' && isConnected" type="button" class="secondary-button" :disabled="connecting" @click="disconnectDiscord") Disconnect

    form(class="space-y-6" @submit.prevent="save")
      div(class="field")
        label(class="field-label" for="integration-provider") Provider
        select(id="integration-provider" v-model="integrationId" class="control" :disabled="Boolean(actionId)")
          option(v-for="provider in providers" :key="provider.id" :value="provider.id") {{ provider.name }}

      div(v-if="integrationId === 'discord' && !isConnected" class="connection-card")
        div
          p(class="font-semibold text-themeText") Connect Discord webhook
          p(class="text-xs text-themeText3 mt-1") The URL is stored in your operating system credential manager, never in a saved action.
        div(class="flex gap-2 mt-3")
          input(v-model="discordWebhook" type="password" class="control flex-1" autocomplete="off" placeholder="https://discord.com/api/webhooks/…")
          button(type="button" class="secondary-button" :disabled="connecting || !discordWebhook.trim()" @click="connectDiscord")
            | {{ connecting ? 'Checking…' : 'Connect' }}

      div(v-if="!isConnected && integrationId === 'discord'" class="notice")
        | Discord webhook is not configured. Connect a webhook above to test and send Discord messages.

      div(class="field")
        label(class="field-label" for="saved-action-name") Name
        input(id="saved-action-name" v-model="actionName" class="control" autocomplete="off" placeholder="Community alert")

      div(class="field")
        label(class="field-label" for="capability-select") Capability
        select(id="capability-select" v-model="selectedCapabilityId" class="control" :disabled="loadingCapabilities || capabilities.length === 0")
          option(value="") {{ loadingCapabilities ? 'Loading capabilities…' : 'Select an action…' }}
          option(v-for="capability in capabilities" :key="capability.capabilityId" :value="capability.capabilityId")
            | {{ formatCapabilityName(capability) }}
        p(v-if="selectedCapability && formatCapabilityDescription(selectedCapability)" class="field-hint") {{ formatCapabilityDescription(selectedCapability) }}
        p(v-else-if="!loadingCapabilities && capabilities.length === 0" class="field-hint") {{ integrationId === 'obs' ? 'Launch OBS Studio with the companion plugin to load scenes and stream actions.' : 'No actions currently available.' }}

      IntegrationCapabilityForm(
        v-if="selectedCapability"
        v-model="parameterValues"
        :inputs="selectedCapability.inputs || []"
        :disabled="false"
        :id-prefix="`${integrationId}-${selectedCapability.capabilityId}`"
        :resolve-options="resolveOptions"
        @validity-change="formValid = $event"
      )

      div(v-if="errorMessage" class="error-card") {{ errorMessage }}
      ActionInputContractEditor(:reference="savedStableId ? { kind: 'saved_action', id: savedStableId } : null")

      footer(class="flex justify-end gap-3 pt-2")
        button(type="button" class="secondary-button" @click="router.back()") Cancel
        button(type="submit" class="primary-button" :disabled="saving || !canSave")
          | {{ saving ? 'Saving…' : actionId ? 'Update action' : 'Save action' }}
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import IntegrationCapabilityForm from "@base/components/Integration/CapabilityForm.vue";
import ActionInputContractEditor from "../../../components/ActionInputContractEditor.vue";
import type { CapabilityDefinition, CapabilityOption, CapabilityParameter } from "@base/types/integrationCapability";
import { validateCapabilityValues } from "@base/types/integrationCapability";
import { invalidateActionCache } from "../../../components/ActionPicker.vue";
import { useMainStore } from "@/stores/Main";

const { t, te } = useI18n();

const formatCapabilityName = (capability: CapabilityDefinition): string => {
  const raw = capability.presentation?.name || capability.capabilityId;
  const id = capability.capabilityId;

  if (te(`obs_actions.${id}`)) {
    return t(`obs_actions.${id}`);
  }
  if (raw.startsWith("APPLET_OBS_")) {
    const converted = "obs_" + raw.replace(/^APPLET_OBS_/, "").toLowerCase();
    if (te(`obs_actions.${converted}`)) {
      return t(`obs_actions.${converted}`);
    }
  }
  if (te(`obs_actions.${raw}`)) {
    return t(`obs_actions.${raw}`);
  }
  if (te(raw)) {
    return t(raw);
  }
  if (raw.startsWith("APPLET_OBS_")) {
    return raw
      .replace(/^APPLET_OBS_/, "")
      .toLowerCase()
      .split("_")
      .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
      .join(" ");
  }
  return raw;
};

const formatCapabilityDescription = (capability: CapabilityDefinition): string => {
  const desc = capability.presentation?.description;
  const name = formatCapabilityName(capability);
  if (!desc || desc === capability.presentation?.name || desc === capability.capabilityId || desc === name) {
    return "";
  }
  return desc;
};

interface IntegrationStatus {
  id: string;
  name: string;
  status: "Connected" | "Disconnected" | "Connecting" | { Error: string };
}

interface SavedIntegrationAction {
  id: number;
  stable_id: string;
  integration_id: string;
  name: string;
  action: string;
  params: string;
  presentation: string;
}

const route = useRoute();
const router = useRouter();
const mainStore = useMainStore();
const { invalidateCache } = useBackgroundPreloader();

const queryValue = (value: unknown): string => Array.isArray(value) ? String(value[0] || "") : String(value || "");
const actionId = ref(queryValue(route.query.actionId));
const savedStableId = ref('');
const copyFromId = ref(queryValue(route.query.copyFrom));
const integrationId = ref(queryValue(route.query.integrationType) || "obs");
const actionName = ref("New action");
const capabilities = ref<CapabilityDefinition[]>([]);
const selectedCapabilityId = ref("");
const parameterValues = ref<Record<string, unknown>>({});
const statuses = ref<Record<string, IntegrationStatus>>({});
const loadingCapabilities = ref(false);
const saving = ref(false);
const connecting = ref(false);
const discordWebhook = ref("");
const errorMessage = ref("");
const formValid = ref(true);
const unlisteners: UnlistenFn[] = [];

const providers = computed(() => {
  const known = Object.values(statuses.value).map((status) => ({ id: status.id, name: status.name }));
  if (!known.some((provider) => provider.id === integrationId.value)) {
    known.push({ id: integrationId.value, name: integrationId.value.toUpperCase() });
  }
  return known.sort((a, b) => a.name.localeCompare(b.name));
});
const selectedCapability = computed(() => capabilities.value.find((item) => item.capabilityId === selectedCapabilityId.value) || null);
const isConnected = computed(() => statuses.value[integrationId.value]?.status === "Connected");
const editorTitle = computed(() => `${actionId.value ? "Update" : "Create"} ${statuses.value[integrationId.value]?.name || integrationId.value.toUpperCase()} action`);
const canSave = computed(() => formValid.value && Boolean(actionName.value.trim()) && validateCapabilityValues(selectedCapability.value, parameterValues.value));

const loadStatuses = async () => {
  statuses.value = await invoke<Record<string, IntegrationStatus>>("get_all_integration_statuses");
};

const loadCapabilities = async () => {
  loadingCapabilities.value = true;
  errorMessage.value = "";
  try {
    capabilities.value = await invoke<CapabilityDefinition[]>("get_integration_capabilities", {
      integrationId: integrationId.value,
    });
  } catch (error) {
    capabilities.value = [];
    errorMessage.value = String(error);
  } finally {
    loadingCapabilities.value = false;
  }
};

const isLoadingSavedAction = ref(false);

const applyCapabilityDefaults = (capability: CapabilityDefinition | null) => {
  if (!capability || isLoadingSavedAction.value) return;
  parameterValues.value = Object.fromEntries(
    (capability.inputs || [])
      .filter((input) => input.defaultValue !== undefined)
      .map((input) => [input.id, input.defaultValue]),
  );
  if (!actionId.value && !copyFromId.value) actionName.value = formatCapabilityName(capability);
};

watch(selectedCapabilityId, () => {
  if (isLoadingSavedAction.value) return;
  applyCapabilityDefaults(selectedCapability.value);
});
watch(integrationId, async () => {
  if (isLoadingSavedAction.value) return;
  selectedCapabilityId.value = "";
  parameterValues.value = {};
  await loadCapabilities();
});

const loadSavedAction = async () => {
  const id = copyFromId.value || actionId.value;
  if (!id) return;
  isLoadingSavedAction.value = true;
  try {
    const saved = await invoke<SavedIntegrationAction>("get_integration_action_by_id", {
      integrationId: integrationId.value,
      actionId: Number(id),
    });
    integrationId.value = saved.integration_id;
    savedStableId.value = copyFromId.value ? '' : saved.stable_id;
    await loadCapabilities();
    let cachedCapability: CapabilityDefinition | undefined;
    try { cachedCapability = JSON.parse(saved.presentation)?.capability; } catch {}
    if (!capabilities.value.some((item) => item.capabilityId === saved.action) && cachedCapability) {
      capabilities.value.push(cachedCapability);
    }
    selectedCapabilityId.value = saved.action;
    let parsedParams: Record<string, unknown> = {};
    try {
      parsedParams = saved.params ? JSON.parse(saved.params) : {};
    } catch (e) {
      console.error("Failed to parse saved action parameters", e);
    }
    const capability = selectedCapability.value || cachedCapability;
    const defaults = Object.fromEntries(
      (capability?.inputs || [])
        .filter((input) => input.defaultValue !== undefined)
        .map((input) => [input.id, input.defaultValue]),
    );
    parameterValues.value = { ...defaults, ...parsedParams };
    actionName.value = copyFromId.value ? `${saved.name} (Copy)` : saved.name;
  } finally {
    await nextTick();
    isLoadingSavedAction.value = false;
  }
};

const connectDiscord = async () => {
  connecting.value = true;
  errorMessage.value = "";
  try {
    await invoke("configure_discord_webhook", { webhookUrl: discordWebhook.value.trim() });
    discordWebhook.value = "";
    await loadStatuses();
    await loadCapabilities();
  } catch (error) {
    errorMessage.value = String(error);
  } finally {
    connecting.value = false;
  }
};

const disconnectDiscord = async () => {
  connecting.value = true;
  errorMessage.value = "";
  try {
    await invoke("disconnect_discord");
    await loadStatuses();
  } catch (error) {
    errorMessage.value = String(error);
  } finally {
    connecting.value = false;
  }
};

const resolveOptions = (input: CapabilityParameter, values: Record<string, unknown>) =>
  invoke<CapabilityOption[]>("resolve_integration_capability_options", {
    integrationId: integrationId.value,
    capabilityId: selectedCapabilityId.value,
    parameterId: input.id,
    currentValues: values,
  });

const save = async () => {
  if (!selectedCapability.value || !canSave.value) return;
  saving.value = true;
  errorMessage.value = "";
  const actionData = {
    integrationId: integrationId.value,
    actionName: selectedCapability.value.capabilityId,
    displayName: actionName.value.trim(),
    parameters: parameterValues.value,
    presentation: {
      integrationName: statuses.value[integrationId.value]?.name || integrationId.value,
      capabilityName: selectedCapability.value.presentation.name,
      capability: selectedCapability.value,
    },
  };
  try {
    if (actionId.value) {
      await invoke("update_integration_action_in_db", {
        integrationId: integrationId.value,
        actionData,
        actionId: Number(actionId.value),
      });
    } else {
      await invoke("save_integration_action_to_db", { integrationId: integrationId.value, actionData });
    }
    invalidateActionCache();
    invalidateCache("savedActions");
    mainStore.addNotification({
      title: "Action saved",
      message: `${actionName.value.trim()} is ready to assign`,
      type: "success",
      autoClose: true,
    });
    router.back();
  } catch (error) {
    errorMessage.value = String(error);
  } finally {
    saving.value = false;
  }
};

onMounted(async () => {
  await loadStatuses();
  await loadCapabilities();
  await loadSavedAction();
  unlisteners.push(await listen("integration_status_changed", async () => { await loadStatuses(); await loadCapabilities(); }));
  unlisteners.push(await listen("integration_registered", async () => { await loadStatuses(); await loadCapabilities(); }));
  unlisteners.push(await listen("obs_register_action", loadCapabilities));
});
onUnmounted(() => unlisteners.forEach((unlisten) => unlisten()));
</script>

<style scoped>
.capability-page { min-height: 100%; padding: 2rem; background: hsl(var(--background)); }
.capability-shell { max-width: 48rem; margin: 0 auto; padding: 1.5rem; border: 1px solid hsl(var(--border2)); border-radius: 1rem; background: hsl(var(--background2)); }
.capability-header { display: flex; justify-content: space-between; gap: 1rem; margin-bottom: 1.5rem; }
.capability-eyebrow { color: hsl(var(--primary)); font-size: .72rem; font-weight: 800; text-transform: uppercase; letter-spacing: .11em; }
.capability-title { color: hsl(var(--text)); font-size: 1.55rem; font-weight: 800; }
.capability-description, .field-hint { color: hsl(var(--text3)); font-size: .8rem; margin-top: .25rem; }
.field { display: flex; flex-direction: column; gap: .4rem; }
.field-label { color: hsl(var(--text)); font-size: .85rem; font-weight: 700; }
.control { width: 100%; border: 1px solid hsl(var(--border2)); border-radius: .65rem; background: hsl(var(--background)); color: hsl(var(--text)); padding: .65rem .75rem; outline: none; }
.control:focus { border-color: hsl(var(--primary)); }
.notice, .connection-card, .error-card { border-radius: .7rem; padding: .8rem; font-size: .8rem; }
.notice { background: rgb(245 158 11 / .1); border: 1px solid rgb(245 158 11 / .25); color: #fbbf24; }
.connection-card { border: 1px solid hsl(var(--border2)); background: hsl(var(--background3)); }
.error-card { background: hsl(var(--error) / .1); color: hsl(var(--error)); }
.primary-button, .secondary-button { border-radius: .6rem; padding: .6rem .9rem; font-size: .82rem; font-weight: 750; }
.primary-button { background: hsl(var(--primary)); color: white; }
.secondary-button { border: 1px solid hsl(var(--border2)); color: hsl(var(--text)); background: hsl(var(--background3)); }
button:disabled { opacity: .5; cursor: not-allowed; }
.status-pill { height: fit-content; border-radius: 999px; padding: .3rem .55rem; font-size: .65rem; font-weight: 800; }
.status-pill--ready { color: #6ee7b7; background: rgb(16 185 129 / .13); }
.status-pill--offline { color: #fbbf24; background: rgb(245 158 11 / .13); }
</style>
