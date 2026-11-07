<template lang="pug">
div(
  v-if="visible"
  class="flow-node-inspector h-full w-full flex flex-col"
  @mousedown.stop
  @click.stop
)
  div(class="flow-node-inspector__header flex items-center justify-between gap-3 px-4 py-3")
    div(class="min-w-0")
      p(class="flow-node-inspector__eyebrow") NODE INSPECTOR
      p(
        class="flow-node-inspector__title truncate"
        :title="node ? (node.text || node.type) : 'No node selected'"
      ) {{ node ? (node.text || node.type) : "No node selected" }}
    button(
      class="flow-node-inspector__close"
      aria-label="Close inspector"
      @click="$emit('close')"
    ) ✕

  div(v-if="node" class="flow-node-inspector__body flex-1 overflow-y-auto px-4 py-3 space-y-4")
    div(class="flow-node-inspector__card")
      div(class="flex items-center justify-between gap-2")
        span(class="flow-node-inspector__label") Type
        span(class="flow-node-inspector__chip") {{ node.type }}

      button(
        v-if="node.isAiNode"
        class="flow-node-inspector__config-ai mt-3"
        @click="$emit('aiConfigRequested', node)"
      ) Configure AI Model

      div(v-if="actionReference" class="mt-3 space-y-1.5")
        div(class="flow-node-inspector__label") Action Reference
        p(class="flow-node-inspector__text")
          | {{ actionReference.kind || "action" }} / {{ actionReference.id || "n/a" }}

    div(v-if="actionReference?.kind === 'saved_action'" class="flow-node-inspector__card space-y-3")
      div(class="flex items-center justify-between gap-2")
        div(class="flow-node-inspector__section-title") Action health
        span(
          class="flow-node-inspector__health"
          :class="healthClass"
        ) {{ healthLabel }}
      div(v-if="node.actionPreflight?.issues?.length" class="space-y-1.5")
        p(
          v-for="issue in node.actionPreflight.issues"
          :key="issue.code"
          class="flow-node-inspector__health-issue"
        ) {{ issue.message }}
      div(class="flow-node-inspector__actions")
        button(@click="refreshActionHealth" :disabled="operationPending") Refresh
        button(@click="testAction" :disabled="operationPending") Test action
        button(
          v-if="needsRebind"
          @click="openActionRepair"
          :disabled="operationPending"
        ) {{ isRecipeBinding ? 'Choose local action' : 'Rebind' }}
        button(
          v-if="needsConnectionRepair"
          @click="openIntegrationSettings"
          :disabled="operationPending"
        ) Connection settings
        button(
          v-if="node.actionPreflight && !isActionReady"
          @click="disableAffectedFlows"
          :disabled="operationPending"
        ) Disable affected flows
      div(v-if="replacementActions.length > 0" class="flow-node-inspector__replacement")
        select(ref="replacementSelect" v-model="replacementStableId" :disabled="operationPending")
          option(value="") {{ isRecipeBinding ? 'Choose a local saved action…' : 'Replace everywhere with…' }}
          option(
            v-for="action in replacementActions"
            :key="action.stable_id"
            :value="action.stable_id"
          ) {{ action.name }} · {{ action.integration_id }}
        button(
          @click="replaceEverywhere"
          :disabled="operationPending || !replacementStableId"
        ) {{ isRecipeBinding ? 'Bind this node' : 'Replace all references' }}
      p(v-if="operationMessage" class="flow-node-inspector__hint") {{ operationMessage }}

      div(class="flow-node-inspector__invocations")
        div(class="flex items-center justify-between")
          span(class="flow-node-inspector__label") Recent executions
          span(v-if="invocationsLoading" class="flow-node-inspector__hint") Loading…
        p(v-if="!invocationsLoading && recentInvocations.length === 0" class="flow-node-inspector__hint") No recorded executions for this node yet.
        div(
          v-for="invocation in recentInvocations"
          :key="invocation.invocationId"
          class="flow-node-inspector__invocation"
          :title="invocation.error || invocation.invocationId"
        )
          span {{ invocation.normalizedStatus || invocation.dispatchStatus }}
          span {{ invocation.executionTimeMs }} ms

    div(v-if="supportsExecutionPolicy" class="flow-node-inspector__card space-y-3")
      div(class="flow-node-inspector__section-title") Execution policy
      p(class="flow-node-inspector__hint") Retry and timeout are enforced by the workflow scheduler. They do not execute a nested copy of the node.
      div(class="grid grid-cols-2 gap-3")
        label(class="flow-node-inspector__field")
          span(class="flow-node-inspector__label") Timeout (ms)
          input(
            :value="node.executionConfig?.timeoutMs || 0"
            type="number"
            min="0"
            step="100"
            class="flow-node-inspector__input"
            @change="updateNumericExecutionConfig('timeoutMs', $event, 0, 86400000)"
          )
        label(class="flow-node-inspector__field")
          span(class="flow-node-inspector__label") Retries
          input(
            :value="node.executionConfig?.retryCount || 0"
            type="number"
            min="0"
            max="10"
            step="1"
            class="flow-node-inspector__input"
            @change="updateNumericExecutionConfig('retryCount', $event, 0, 10)"
          )
        label(class="flow-node-inspector__field col-span-2")
          span(class="flow-node-inspector__label") Delay between retries (ms)
          input(
            :value="node.executionConfig?.retryDelayMs ?? 1000"
            type="number"
            min="0"
            max="3600000"
            step="100"
            class="flow-node-inspector__input"
            @change="updateNumericExecutionConfig('retryDelayMs', $event, 0, 3600000)"
          )
      label(class="flow-node-inspector__field")
        span(class="flow-node-inspector__label") After final failure
        select(
          :value="node.executionConfig?.skipOnError ? 'route' : 'stop'"
          class="flow-node-inspector__input"
          @change="updateFailureBehavior"
        )
          option(value="stop") Stop and fail the flow
          option(value="route") Continue through On Error
      p(v-if="node.executionConfig?.skipOnError" class="flow-node-inspector__hint") Connect On Error to a catch or fallback branch. Error Message and Error Details can be connected as data.

    div(v-if="filteredInputs.length > 0" class="flow-node-inspector__card space-y-3")
      div(class="flow-node-inspector__section-title") Inputs
      div(
        v-for="input in filteredInputs"
        :key="input.id"
        class="flow-node-inspector__field"
        :class="{ 'flow-node-inspector__field--required-empty': isRequiredInputEmpty(input) }"
        :style="{ '--field-accent-rgb': getInputAccentRgb(input), '--field-accent-color': getTypeColor(input.type) }"
      )
        div(class="flow-node-inspector__field-head")
          div(class="flow-node-inspector__field-label-wrap")
            span(class="flow-node-inspector__field-kind") {{ getInputKindToken(input) }}
            label(class="flow-node-inspector__label truncate" :title="input.name") {{ input.name }}
            span(v-if="isInputRequired(input)" class="flow-node-inspector__required-star" aria-hidden="true") *
            span(v-if="isInputRequired(input)" class="flow-node-inspector__required-chip") required
          div(class="flex items-center gap-2")
            span(class="flow-node-inspector__type flow-node-inspector__type--input") {{ input.type }}
            span(v-if="input.attached" class="flow-node-inspector__connected-pill") linked

        div(class="flow-node-inspector__field-body")
          span(v-if="input.forceInput" class="flow-node-inspector__link-only") Connection only
          div(v-else-if="input.attached" class="flow-node-inspector__connected-row")
            span(class="flow-node-inspector__connected-dot")
            span(class="flow-node-inspector__link-only") Connected input
            span(
              v-if="input.value !== undefined && input.value !== null"
              class="flow-node-inspector__connected-preview truncate"
              :title="formatInputPreview(input.value)"
            ) {{ formatInputPreview(input.value) }}

          template(v-else-if="getWidgetComponent(input.widget)")
            component(
              :is="getWidgetComponent(input.widget)"
              v-model="input.value"
              v-bind="getWidgetProps(input)"
              @update:modelValue="onInputChanged(input)"
            )

          template(v-else-if="input.widget === 'textarea'")
            textarea(
              v-model="input.value"
              :rows="input.widgetOptions?.rows ?? 4"
              class="flow-node-inspector__textarea"
              @input="onInputChanged(input)"
            )

          template(v-else-if="input.type === 'number'")
            input(
              v-model.number="input.value"
              type="number"
              class="flow-node-inspector__input"
              @input="onInputChanged(input)"
            )

          template(v-else-if="input.type === 'boolean'")
            component(
              :is="getWidgetComponent('toggle')"
              v-model="input.value"
              @update:modelValue="onInputChanged(input)"
            )

          template(v-else-if="input.type === 'object' || input.type === 'array'")
            textarea(
              :value="getStructuredInputValue(input)"
              :rows="input.type === 'array' ? 3 : 4"
              class="flow-node-inspector__textarea flow-node-inspector__textarea--json"
              :class="{ 'flow-node-inspector__textarea--invalid': hasStructuredDraftError(input) }"
              @input="handleStructuredInputInput(input, $event)"
              @blur="handleStructuredInputBlur(input)"
            )
            p(class="flow-node-inspector__json-hint")
              | JSON {{ input.type }} (applies on blur)

          template(v-else)
            input(
              v-model="input.value"
              type="text"
              class="flow-node-inspector__input"
              @input="onInputChanged(input)"
            )

        div(class="flow-node-inspector__field-foot")
          span(class="flow-node-inspector__requirement")
            | {{ input.required === false ? "Optional" : "Required" }}
          span(v-if="isRequiredInputEmpty(input)" class="flow-node-inspector__required-warning") Required value missing
          span(
            v-if="!isRequiredInputEmpty(input) && !input.attached && !input.forceInput"
            class="flow-node-inspector__value-preview truncate"
            :title="formatInputPreview(input.value)"
          ) {{ formatInputPreview(input.value) }}

    div(v-if="node.outputs && node.outputs.length > 0" class="flow-node-inspector__card space-y-2.5")
      div(class="flow-node-inspector__section-title") Outputs
      div(
        v-for="output in node.outputs"
        :key="output.id"
        class="flow-node-inspector__output-row"
      )
        div(class="flow-node-inspector__output-dot" :style="{ borderColor: getTypeColor(output.type), backgroundColor: `${getTypeColor(output.type)}33` }")
        span(class="flow-node-inspector__text truncate" :title="output.name") {{ output.name }}
        span(class="flow-node-inspector__type") {{ output.type }}

  div(v-else class="flow-node-inspector__empty flex-1 px-4 py-6")
    div(class="flow-node-inspector__card h-full flex items-center justify-center")
      div(class="text-center space-y-2")
        p(class="flow-node-inspector__section-title") Inspector Ready
        p(class="flow-node-inspector__text") Select a node to edit inputs, outputs, and assignment details.
        p(class="flow-node-inspector__hint") Press `I` to show or hide this panel.
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { ActionPreflight, SavedActionPreflight } from "@base/types/actionReference";
import type { NodeExecutionConfig } from "../../../types/flow";
import { getWidgetComponent } from "../../../composables/useWidgetRegistry";
import { getTypeColor } from "../../../utils/typeColors";

interface NodeIO {
  id: string;
  name: string;
  type: string;
  value?: any;
  attached?: boolean;
  enum?: string[];
  required?: boolean;
  forceInput?: boolean;
  widget?: string;
  widgetOptions?: {
    min?: number;
    max?: number;
    step?: number;
    accept?: string[];
    rows?: number;
    options?: string[];
  };
  displayOptions?: {
    show?: Record<string, any[]>;
    hide?: Record<string, any[]>;
  };
}

interface FlowNode {
  id: number;
  type: string;
  text: string;
  isAiNode?: boolean;
  aiModel?: string;
  inputs?: NodeIO[];
  outputs?: NodeIO[];
  parameters?: Record<string, any>;
  actionPreflight?: ActionPreflight;
  executionConfig?: NodeExecutionConfig;
}

interface InvocationRecord {
  invocationId: string;
  nodeId?: string | null;
  dispatchStatus: string;
  normalizedStatus?: string | null;
  executionTimeMs: number;
  error?: string | null;
}

interface SavedActionBinding {
  id: number;
  integration_id: string;
  stable_id: string;
  name: string;
}

const props = withDefaults(
  defineProps<{
    visible?: boolean;
    node?: FlowNode | null;
  }>(),
  {
    visible: false,
    node: null,
  }
);

const emit = defineEmits<{
  close: [];
  inputChanged: [payload: { node: FlowNode; input: NodeIO }];
  executionConfigChanged: [payload: { node: FlowNode; executionConfig: NodeExecutionConfig }];
  aiConfigRequested: [node: FlowNode];
  actionReferenceChanged: [payload: { node: FlowNode; stableId: string }];
  actionPreflightChanged: [payload: { node: FlowNode; preflight: ActionPreflight }];
}>();

const actionReference = computed(() => props.node?.parameters?.actionReference || null);
const operationPending = ref(false);
const operationMessage = ref("");
const invocationsLoading = ref(false);
const recentInvocations = ref<InvocationRecord[]>([]);
const replacementActions = ref<SavedActionBinding[]>([]);
const replacementStableId = ref("");
const replacementSelect = ref<HTMLSelectElement | null>(null);
const isRecipeBinding = computed(() => actionReference.value?.id?.startsWith("recipe-binding:") === true);
const supportsExecutionPolicy = computed(() => {
  const type = props.node?.type?.toLowerCase() || "";
  return Boolean(type && !type.includes("trigger") && type !== "approval");
});

const currentExecutionConfig = (): NodeExecutionConfig => ({
  timeoutMs: props.node?.executionConfig?.timeoutMs,
  retryCount: props.node?.executionConfig?.retryCount ?? 0,
  retryDelayMs: props.node?.executionConfig?.retryDelayMs ?? 1000,
  skipOnError: props.node?.executionConfig?.skipOnError ?? false,
  parallelExecution: props.node?.executionConfig?.parallelExecution ?? false,
});

const emitExecutionConfig = (patch: Partial<NodeExecutionConfig>) => {
  if (!props.node) return;
  emit("executionConfigChanged", {
    node: props.node,
    executionConfig: { ...currentExecutionConfig(), ...patch },
  });
};

const updateNumericExecutionConfig = (
  field: "timeoutMs" | "retryCount" | "retryDelayMs",
  event: Event,
  min: number,
  max: number,
) => {
  const raw = Number((event.target as HTMLInputElement).value);
  const value = Number.isFinite(raw) ? Math.min(max, Math.max(min, Math.round(raw))) : min;
  emitExecutionConfig({ [field]: field === "timeoutMs" && value === 0 ? undefined : value });
};

const updateFailureBehavior = (event: Event) => {
  emitExecutionConfig({
    skipOnError: (event.target as HTMLSelectElement).value === "route",
  });
};

const isActionReady = computed(() => ["ready", "receipt_degraded"].includes(
  props.node?.actionPreflight?.status || "",
));
const healthLabel = computed(() => (props.node?.actionPreflight?.status || "checking").replace(/_/g, " "));
const healthClass = computed(() => isActionReady.value
  ? "flow-node-inspector__health--ready"
  : props.node?.actionPreflight?.status === "syncing"
    || props.node?.actionPreflight?.status === "disconnected"
    ? "flow-node-inspector__health--warning"
    : "flow-node-inspector__health--error");
const repairKinds = computed(() => props.node?.actionPreflight?.issues
  ?.flatMap((issue) => issue.repairs || []) || []);
const needsRebind = computed(() => repairKinds.value.includes("rebind_action"));
const needsConnectionRepair = computed(() => repairKinds.value.includes("open_integration_settings"));

const loadRecentInvocations = async () => {
  if (!props.node) return;
  invocationsLoading.value = true;
  try {
    const records = await invoke<InvocationRecord[]>("list_action_invocations", {
      limit: 100,
      source: "flow",
      status: null,
      workflowRunId: null,
      nodeId: String(props.node.id),
    });
    recentInvocations.value = records.slice(0, 8);
  } catch {
    recentInvocations.value = [];
  } finally {
    invocationsLoading.value = false;
  }
};

const refreshActionHealth = async () => {
  if (!props.node || !actionReference.value?.id) return;
  if (isRecipeBinding.value) {
    operationMessage.value = "Choose a local saved action to repair this recipe binding.";
    return;
  }
  operationPending.value = true;
  try {
    const result = await invoke<SavedActionPreflight>("preflight_saved_integration_action", {
      stableId: actionReference.value.id,
    });
    emit("actionPreflightChanged", { node: props.node, preflight: result.preflight });
    operationMessage.value = "Health refreshed.";
  } catch (error) {
    operationMessage.value = String(error);
  } finally {
    operationPending.value = false;
  }
};

const testAction = async () => {
  operationPending.value = true;
  try {
    const result = await invoke<{ status: string }>("invoke_action_reference", {
      reference: actionReference.value,
      value: null,
      variables: {},
    });
    operationMessage.value = `Test completed: ${result.status.replace(/_/g, " ")}.`;
    await loadRecentInvocations();
  } catch (error) {
    operationMessage.value = `Test failed: ${String(error)}`;
    await loadRecentInvocations();
  } finally {
    operationPending.value = false;
  }
};

const openActionRepair = async () => {
  if (isRecipeBinding.value) {
    operationMessage.value = "Choose a compatible local saved action below.";
    await nextTick();
    replacementSelect.value?.focus();
    return;
  }
  operationPending.value = true;
  try {
    const binding = await invoke<SavedActionBinding>("get_saved_integration_action_by_stable_id", {
      stableId: actionReference.value.id,
    });
    await navigateTo({
      path: "/deck/new/action",
      query: { integrationType: binding.integration_id, actionId: String(binding.id) },
    });
  } catch (error) {
    operationMessage.value = String(error);
  } finally {
    operationPending.value = false;
  }
};

const openIntegrationSettings = () => {
  useSettingsModal().openSettings("connection");
};

const disableAffectedFlows = async () => {
  operationPending.value = true;
  try {
    const count = await invoke<number>("disable_flows_using_saved_action", {
      stableId: actionReference.value.id,
    });
    operationMessage.value = `Disabled ${count} affected flow${count === 1 ? "" : "s"}.`;
  } catch (error) {
    operationMessage.value = String(error);
  } finally {
    operationPending.value = false;
  }
};

const loadReplacementActions = async () => {
  if (!actionReference.value?.id) return;
  try {
    const grouped = await invoke<Record<string, SavedActionBinding[]>>(
      "get_all_saved_integration_actions",
    );
    replacementActions.value = Object.values(grouped)
      .flat()
      .filter((action) => action.stable_id !== actionReference.value.id);
  } catch {
    replacementActions.value = [];
  }
};

const replaceEverywhere = async () => {
  if (!replacementStableId.value || !props.node) return;
  operationPending.value = true;
  try {
    if (isRecipeBinding.value) {
      emit("actionReferenceChanged", { node: props.node, stableId: replacementStableId.value });
      operationMessage.value = "Local action bound to this imported node.";
      replacementStableId.value = "";
      await Promise.all([refreshActionHealth(), loadReplacementActions()]);
      return;
    }
    const impact = await invoke<Record<string, string | number>>("replace_saved_action_references", {
      oldStableId: actionReference.value.id,
      newStableId: replacementStableId.value,
    });
    const changed = Object.entries(impact)
      .filter(([key]) => key !== "stableId")
      .reduce((sum, [, count]) => sum + (typeof count === "number" ? count : 0), 0);
    emit("actionReferenceChanged", { node: props.node, stableId: replacementStableId.value });
    operationMessage.value = `Replaced ${changed} reference${changed === 1 ? "" : "s"}.`;
    replacementStableId.value = "";
    await Promise.all([refreshActionHealth(), loadReplacementActions()]);
  } catch (error) {
    operationMessage.value = String(error);
  } finally {
    operationPending.value = false;
  }
};

watch(() => props.node?.id, async () => {
  operationMessage.value = "";
  if (actionReference.value?.kind === "saved_action") {
    const checks = [loadRecentInvocations(), loadReplacementActions()];
    if (!isRecipeBinding.value) checks.push(refreshActionHealth());
    else operationMessage.value = "Choose a local saved action to repair this recipe binding.";
    await Promise.all(checks);
  } else {
    recentInvocations.value = [];
    replacementActions.value = [];
  }
}, { immediate: true });

const isInputVisible = (input: NodeIO, node: FlowNode): boolean => {
  if (!input.displayOptions) return true;

  const { show, hide } = input.displayOptions;

  if (show) {
    for (const [fieldName, allowedValues] of Object.entries(show)) {
      const fieldInput = node.inputs?.find((item) => item.id === fieldName);
      if (!fieldInput || !allowedValues.includes(fieldInput.value)) {
        return false;
      }
    }
  }

  if (hide) {
    for (const [fieldName, disallowedValues] of Object.entries(hide)) {
      const fieldInput = node.inputs?.find((item) => item.id === fieldName);
      if (fieldInput && disallowedValues.includes(fieldInput.value)) {
        return false;
      }
    }
  }

  return true;
};

const filteredInputs = computed(() => {
  if (!props.node?.inputs) return [];
  const visible = props.node.inputs.filter((input) =>
    isInputVisible(input, props.node as FlowNode)
  );
  return visible.sort((a, b) => {
    const aRequired = isInputRequired(a);
    const bRequired = isInputRequired(b);
    if (aRequired === bRequired) return a.name.localeCompare(b.name);
    return aRequired ? -1 : 1;
  });
});

const getWidgetProps = (input: NodeIO): Record<string, any> => {
  const widgetProps: Record<string, any> = {};

  if (input.widgetOptions) {
    Object.assign(widgetProps, input.widgetOptions);
  }

  if (input.widget === "file") {
    widgetProps.placeholder = input.widgetOptions?.accept?.join(", ") || `${input.name} file`;
  }

  if (input.widget === "select" && input.enum) {
    widgetProps.options = input.enum;
  }

  return widgetProps;
};

const onInputChanged = (input: NodeIO) => {
  if (!props.node) return;
  emit("inputChanged", { node: props.node, input });
};

const structuredInputDrafts = ref<Record<string, string>>({});

const stringifyStructuredValue = (value: any): string => {
  if (value === undefined || value === null) return "";
  if (typeof value === "string") return value;
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
};

const getStructuredInputValue = (input: NodeIO): string => {
  const draft = structuredInputDrafts.value[input.id];
  if (draft !== undefined) return draft;
  return stringifyStructuredValue(input.value);
};

const handleStructuredInputInput = (input: NodeIO, event: Event) => {
  const target = event.target as HTMLTextAreaElement;
  structuredInputDrafts.value[input.id] = target.value;
};

const hasStructuredDraftError = (input: NodeIO): boolean => {
  const draft = structuredInputDrafts.value[input.id];
  if (draft === undefined) return false;
  const raw = draft.trim();
  if (!raw) return false;
  try {
    const parsed = JSON.parse(raw);
    if (input.type === "array") return !Array.isArray(parsed);
    return typeof parsed !== "object" || parsed === null || Array.isArray(parsed);
  } catch {
    return true;
  }
};

const handleStructuredInputBlur = (input: NodeIO) => {
  const draft = structuredInputDrafts.value[input.id];
  if (draft === undefined) return;

  const raw = draft.trim();

  if (!raw) {
    input.value = input.type === "array" ? [] : {};
    delete structuredInputDrafts.value[input.id];
    onInputChanged(input);
    return;
  }

  try {
    const parsed = JSON.parse(raw);
    const isValid =
      input.type === "array"
        ? Array.isArray(parsed)
        : typeof parsed === "object" && parsed !== null && !Array.isArray(parsed);

    if (!isValid) return;

    input.value = parsed;
    delete structuredInputDrafts.value[input.id];
    onInputChanged(input);
  } catch {
    // Keep draft as-is so user can fix JSON without losing text.
  }
};

const isInputRequired = (input: NodeIO): boolean => input.required !== false;

const isRequiredInputEmpty = (input: NodeIO): boolean => {
  if (!isInputRequired(input)) return false;
  if (input.attached || input.forceInput) return false;

  const value = input.value;
  if (value === undefined || value === null) return true;
  if (typeof value === "string") return value.trim().length === 0;
  if (Array.isArray(value)) return value.length === 0;
  return false;
};

const normalizeType = (type?: string): string => String(type || "").toLowerCase();

const typeAccentRgbMap: Record<string, string> = {
  string: "99, 102, 241",
  number: "56, 189, 248",
  boolean: "74, 222, 128",
  array: "168, 85, 247",
  object: "251, 191, 36",
  flow: "248, 113, 113",
};

const getInputAccentRgb = (input: NodeIO): string => {
  const normalized = normalizeType(input.type);
  return typeAccentRgbMap[normalized] || "125, 151, 199";
};

const getInputKindToken = (input: NodeIO): string => {
  if (input.forceInput) return "IN";

  const widget = String(input.widget || "").toLowerCase();
  const type = normalizeType(input.type);

  if (widget === "select") return "SEL";
  if (widget === "multiselect") return "ARR";
  if (widget === "slider") return "NUM";
  if (widget === "toggle" || type === "boolean") return "BOOL";
  if (widget === "color") return "CLR";
  if (widget === "file") return "FILE";
  if (widget === "textarea") return "TXT";
  if (type === "number") return "NUM";
  if (type === "array") return "ARR";
  if (type === "object") return "OBJ";
  return "STR";
};

const formatInputPreview = (value: any): string => {
  if (value === undefined || value === null || value === "") return "Empty";
  if (typeof value === "boolean") return value ? "True" : "False";
  if (typeof value === "number") return String(value);
  if (typeof value === "string") return value.length > 48 ? `${value.slice(0, 48)}...` : value;
  if (Array.isArray(value)) return `${value.length} item(s)`;
  if (typeof value === "object") return "JSON object";
  return String(value);
};
</script>

<style lang="scss" scoped>
.flow-node-inspector__health {
  border-radius: 999px;
  padding: 0.15rem 0.48rem;
  font-size: 0.62rem;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

.flow-node-inspector__health--ready {
  background: rgba(16, 185, 129, 0.14);
  color: #6ee7b7;
}

.flow-node-inspector__health--warning {
  background: rgba(245, 158, 11, 0.14);
  color: #fcd34d;
}

.flow-node-inspector__health--error {
  background: rgba(244, 63, 94, 0.14);
  color: #fda4af;
}

.flow-node-inspector__health-issue {
  color: hsl(var(--text2));
  font-size: 0.72rem;
  line-height: 1.35;
}

.flow-node-inspector__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}

.flow-node-inspector__actions button {
  border: 1px solid hsl(var(--border));
  border-radius: 0.5rem;
  background: hsl(var(--background));
  color: hsl(var(--text2));
  font-size: 0.68rem;
  padding: 0.32rem 0.5rem;
}

.flow-node-inspector__actions button:hover:not(:disabled) {
  border-color: hsl(var(--primary) / 0.65);
  color: hsl(var(--primary));
}

.flow-node-inspector__actions button:disabled {
  opacity: 0.5;
}

.flow-node-inspector__replacement {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 0.4rem;
}

.flow-node-inspector__replacement select,
.flow-node-inspector__replacement button {
  min-width: 0;
  border: 1px solid hsl(var(--border));
  border-radius: 0.5rem;
  background: hsl(var(--background));
  color: hsl(var(--text2));
  font-size: 0.66rem;
  padding: 0.35rem 0.45rem;
}

.flow-node-inspector__invocations {
  border-top: 1px solid hsl(var(--border));
  padding-top: 0.65rem;
}

.flow-node-inspector__invocation {
  display: flex;
  justify-content: space-between;
  gap: 0.5rem;
  margin-top: 0.35rem;
  border-radius: 0.45rem;
  background: hsl(var(--background));
  color: hsl(var(--text2));
  font-size: 0.66rem;
  padding: 0.35rem 0.48rem;
}
.flow-node-inspector {
  background: hsl(var(--background2) / 0.96);
  color: hsl(var(--text));
  border-left: 1px solid hsl(var(--border));
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
}

.flow-node-inspector__header {
  border-bottom: 1px solid hsl(var(--border) / 0.6);
  background: hsl(var(--background2));
}

.flow-node-inspector__eyebrow {
  font-size: 0.62rem;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: hsl(var(--text3));
  font-weight: 700;
}

.flow-node-inspector__title {
  font-size: 0.95rem;
  font-weight: 700;
  color: hsl(var(--text));
}

.flow-node-inspector__close {
  width: 1.65rem;
  height: 1.65rem;
  border-radius: 0.45rem;
  border: 1px solid hsl(var(--border));
  background: hsl(var(--background));
  color: hsl(var(--text2));
  font-size: 0.75rem;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
  cursor: pointer;
}

.flow-node-inspector__close:hover {
  background: hsl(var(--background3));
  border-color: hsl(var(--border2));
  color: hsl(var(--text));
}

.flow-node-inspector__card {
  border: 1px solid hsl(var(--border));
  border-radius: 0.85rem;
  padding: 0.85rem;
  background: hsl(var(--background) / 0.6);
}

.flow-node-inspector__section-title {
  font-size: 0.68rem;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: hsl(var(--text3));
  font-weight: 700;
}

.flow-node-inspector__field {
  border: 1px solid hsl(var(--border));
  border-left: 3px solid var(--field-accent-color, hsl(var(--primary)));
  border-radius: 0.7rem;
  background: hsl(var(--background2));
  padding: 0.6rem 0.75rem;
}

.flow-node-inspector__field-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.45rem;
  margin-bottom: 0.45rem;
}

.flow-node-inspector__field-label-wrap {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  min-width: 0;
}

.flow-node-inspector__field-kind {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 2.15rem;
  height: 1.1rem;
  border-radius: 0.35rem;
  border: 1px solid hsl(var(--border));
  background: hsl(var(--background3));
  color: var(--field-accent-color, hsl(var(--primary)));
  font-size: 0.58rem;
  letter-spacing: 0.06em;
  font-weight: 700;
}

.flow-node-inspector__required-star {
  color: #f43f5e;
  font-size: 0.75rem;
  line-height: 1;
}

.flow-node-inspector__required-chip {
  border: 1px solid rgba(244, 63, 94, 0.3);
  background: rgba(244, 63, 94, 0.1);
  color: #f43f5e;
  font-size: 0.58rem;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  border-radius: 999px;
  padding: 0.05rem 0.35rem;
}

.flow-node-inspector__label {
  font-size: 0.76rem;
  color: hsl(var(--text));
  font-weight: 600;
}

.flow-node-inspector__text {
  font-size: 0.82rem;
  color: hsl(var(--text2));
  line-height: 1.25;
}

.flow-node-inspector__chip {
  font-size: 0.66rem;
  color: hsl(var(--text3));
  border: 1px solid hsl(var(--border));
  background: hsl(var(--background3));
  border-radius: 999px;
  padding: 0.15rem 0.5rem;
}

.flow-node-inspector__config-ai {
  width: 100%;
  border: 1px solid hsl(var(--primary) / 0.4);
  border-radius: 0.6rem;
  padding: 0.5rem 0.6rem;
  font-size: 0.76rem;
  font-weight: 600;
  color: hsl(var(--primary));
  background: hsl(var(--primary) / 0.1);
  transition: all 0.15s ease;
  cursor: pointer;
}

.flow-node-inspector__config-ai:hover {
  border-color: hsl(var(--primary) / 0.7);
  background: hsl(var(--primary) / 0.18);
}

.flow-node-inspector__connected-pill {
  border: 1px solid hsl(var(--primary) / 0.4);
  background: hsl(var(--primary) / 0.12);
  color: hsl(var(--primary));
  font-size: 0.62rem;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  border-radius: 999px;
  padding: 0.08rem 0.38rem;
}

.flow-node-inspector__type {
  font-size: 0.66rem;
  color: hsl(var(--text3));
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.flow-node-inspector__type--input {
  color: hsl(var(--text3));
}

.flow-node-inspector__field-body {
  margin-bottom: 0.4rem;
}

.flow-node-inspector__field-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.45rem;
}

.flow-node-inspector__requirement {
  font-size: 0.63rem;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: hsl(var(--text3));
}

.flow-node-inspector__value-preview {
  font-size: 0.66rem;
  color: hsl(var(--text2));
  max-width: 9.2rem;
}

.flow-node-inspector__required-warning {
  font-size: 0.66rem;
  color: #f43f5e;
}

.flow-node-inspector__input,
.flow-node-inspector__textarea {
  width: 100%;
  border: 1px solid hsl(var(--border));
  border-radius: 0.55rem;
  background: hsl(var(--background));
  color: hsl(var(--text));
  font-size: 0.8rem;
  padding: 0.45rem 0.6rem;
}

.flow-node-inspector__textarea {
  resize: vertical;
  min-height: 4.5rem;
}

.flow-node-inspector__textarea--json {
  font-family: "JetBrains Mono", "Consolas", "Menlo", monospace;
  font-size: 0.72rem;
  line-height: 1.35;
}

.flow-node-inspector__textarea--invalid {
  border-color: #f43f5e !important;
  box-shadow: 0 0 0 2px rgba(244, 63, 94, 0.15) !important;
}

.flow-node-inspector__json-hint {
  margin-top: 0.28rem;
  font-size: 0.64rem;
  color: hsl(var(--text3));
}

.flow-node-inspector__link-only {
  font-size: 0.7rem;
  color: hsl(var(--primary));
}

.flow-node-inspector__connected-row {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  border: 1px solid hsl(var(--primary) / 0.3);
  border-radius: 0.55rem;
  background: hsl(var(--primary) / 0.08);
  padding: 0.4rem 0.55rem;
}

.flow-node-inspector__field--required-empty {
  border-color: rgba(244, 63, 94, 0.4);
}

.flow-node-inspector__connected-dot {
  width: 0.48rem;
  height: 0.48rem;
  border-radius: 999px;
  background: hsl(var(--primary));
}

.flow-node-inspector__connected-preview {
  margin-left: auto;
  font-size: 0.66rem;
  color: hsl(var(--text2));
  max-width: 8rem;
}

.flow-node-inspector__output-row {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  border: 1px solid hsl(var(--border));
  border-radius: 0.55rem;
  background: hsl(var(--background));
  padding: 0.4rem 0.55rem;
}

.flow-node-inspector__output-dot {
  width: 0.62rem;
  height: 0.62rem;
  border-radius: 999px;
  border: 1px solid;
  flex-shrink: 0;
}

.flow-node-inspector__hint {
  font-size: 0.7rem;
  color: hsl(var(--text3));
}

.flow-node-inspector__field-body :deep(input[type="text"]),
.flow-node-inspector__field-body :deep(input[type="number"]),
.flow-node-inspector__field-body :deep(textarea),
.flow-node-inspector__field-body :deep(select),
.flow-node-inspector__field-body :deep(button) {
  border-radius: 0.55rem !important;
}

.flow-node-inspector__field-body :deep(input[type="text"]),
.flow-node-inspector__field-body :deep(input[type="number"]),
.flow-node-inspector__field-body :deep(textarea),
.flow-node-inspector__field-body :deep(select) {
  background: hsl(var(--background)) !important;
  border: 1px solid hsl(var(--border)) !important;
  color: hsl(var(--text)) !important;
  box-shadow: none !important;
}

.flow-node-inspector__field-body :deep(input:focus),
.flow-node-inspector__field-body :deep(textarea:focus),
.flow-node-inspector__field-body :deep(select:focus) {
  border-color: hsl(var(--primary) / 0.6) !important;
  box-shadow: 0 0 0 2px hsl(var(--primary) / 0.15) !important;
}

.flow-node-inspector__field-body :deep(.duration-picker),
.flow-node-inspector__field-body :deep(.duration-inputs) {
  width: 100%;
}
</style>
