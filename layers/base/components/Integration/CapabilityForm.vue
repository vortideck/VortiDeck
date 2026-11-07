<template lang="pug">
div(class="space-y-4")
  div(v-for="input in visibleInputs" :key="input.id" class="space-y-1.5")
    label(class="block text-sm font-semibold text-themeText" :for="fieldId(input.id)")
      | {{ formatLabel(input.label) }}
      span(v-if="input.required" class="text-red-400 ml-1") *
    p(v-if="formatDescription(input.description, input.label)" class="text-xs text-themeText3") {{ formatDescription(input.description, input.label) }}

    select(
      v-if="input.widget === 'select'"
      :id="fieldId(input.id)"
      class="capability-control"
      :disabled="disabled"
      :value="optionKey(modelValue[input.id])"
      @change="setSelect(input, $event)"
    )
      option(value="") Select…
      option(v-for="option in optionsFor(input)" :key="optionKey(option.value)" :value="optionKey(option.value)") {{ option.label }}

    select(
      v-else-if="input.widget === 'multi_select'"
      :id="fieldId(input.id)"
      class="capability-control"
      multiple
      :disabled="disabled"
      :value="multiSelectKeys(input)"
      @change="setMultiSelect(input, $event)"
    )
      option(v-for="option in optionsFor(input)" :key="optionKey(option.value)" :value="optionKey(option.value)") {{ option.label }}

    label(v-else-if="input.widget === 'toggle'" class="inline-flex items-center gap-2")
      input(type="checkbox" :checked="Boolean(modelValue[input.id])" :disabled="disabled" @change="setBoolean(input.id, $event)")
      span(class="text-sm text-themeText2") {{ modelValue[input.id] ? 'Enabled' : 'Disabled' }}

    textarea(
      v-else-if="input.widget === 'textarea' || input.widget === 'json'"
      :id="fieldId(input.id)"
      class="capability-control min-h-24 resize-y"
      :disabled="disabled"
      :value="displayValue(input)"
      @input="setText(input, $event)"
    )

    input(
      v-else
      :id="fieldId(input.id)"
      :type="input.widget === 'color' ? 'color' : input.widget === 'number' ? 'number' : 'text'"
      class="capability-control"
      :disabled="disabled"
      :value="displayValue(input)"
      @input="setText(input, $event)"
    )

    p(v-if="fieldErrors[input.id]" class="text-xs text-red-400") {{ fieldErrors[input.id] }}
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { CapabilityOption, CapabilityParameter } from "@base/types/integrationCapability";
import { capabilityOptionKey, visibleCapabilityInputs } from "@base/types/integrationCapability";

const { t, te } = useI18n();

const formatLabel = (raw: string): string => {
  if (!raw) return "";
  if (te(`obs_actions.parameters.${raw}`)) {
    return t(`obs_actions.parameters.${raw}`);
  }
  if (te(`parameters.${raw}`)) {
    return t(`parameters.${raw}`);
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

const formatDescription = (desc: string | undefined, label: string): string => {
  if (!desc) return "";
  const formattedDesc = formatLabel(desc);
  const formattedLabel = formatLabel(label);
  if (formattedDesc.toLowerCase() === formattedLabel.toLowerCase() || desc === label) {
    return "";
  }
  return formattedDesc;
};

const props = withDefaults(defineProps<{
  inputs: CapabilityParameter[];
  modelValue: Record<string, unknown>;
  disabled?: boolean;
  idPrefix?: string;
  resolveOptions?: (input: CapabilityParameter, values: Record<string, unknown>) => Promise<CapabilityOption[]>;
}>(), { disabled: false, idPrefix: "capability" });

const emit = defineEmits<{
  "update:modelValue": [value: Record<string, unknown>];
  "validity-change": [valid: boolean];
}>();

const visibleInputs = computed(() => visibleCapabilityInputs(props.inputs || [], props.modelValue));
const dynamicOptions = ref<Record<string, CapabilityOption[]>>({});
const fieldErrors = ref<Record<string, string>>({});
const optionKey = capabilityOptionKey;
const fieldId = (id: string) => `${props.idPrefix}-${id.replace(/[^a-zA-Z0-9_-]/g, "-")}`;

const update = (id: string, value: unknown) => emit("update:modelValue", { ...props.modelValue, [id]: value });
const optionsFor = (input: CapabilityParameter) => dynamicOptions.value[input.id] || input.options || [];

const setSelect = (input: CapabilityParameter, event: Event) => {
  const key = (event.target as HTMLSelectElement).value;
  const next = { ...props.modelValue };
  const removeNested = (parameters: CapabilityParameter[]) => {
    for (const parameter of parameters) {
      delete next[parameter.id];
      for (const option of parameter.options || []) removeNested(option.parameters || []);
    }
  };
  for (const option of optionsFor(input)) removeNested(option.parameters || []);
  if (!key) {
    next[input.id] = undefined;
    return emit("update:modelValue", next);
  }
  const option = optionsFor(input).find((candidate) => optionKey(candidate.value) === key);
  next[input.id] = option?.value;
  for (const parameter of option?.parameters || []) {
    if (parameter.defaultValue !== undefined) next[parameter.id] = parameter.defaultValue;
  }
  emit("update:modelValue", next);
};

const multiSelectKeys = (input: CapabilityParameter) => {
  const values = Array.isArray(props.modelValue[input.id]) ? props.modelValue[input.id] as unknown[] : [];
  return values.map(optionKey);
};

const setMultiSelect = (input: CapabilityParameter, event: Event) => {
  const keys = Array.from((event.target as HTMLSelectElement).selectedOptions).map((option) => option.value);
  update(input.id, optionsFor(input).filter((option) => keys.includes(optionKey(option.value))).map((option) => option.value));
};

const setBoolean = (id: string, event: Event) => update(id, (event.target as HTMLInputElement).checked);

const displayValue = (input: CapabilityParameter): string => {
  const value = props.modelValue[input.id] ?? input.defaultValue ?? "";
  if (input.widget === "color" && typeof value === "number") return `#${value.toString(16).padStart(6, "0")}`;
  if (input.widget === "json" && typeof value !== "string") return value ? JSON.stringify(value, null, 2) : "";
  return String(value);
};

const setText = (input: CapabilityParameter, event: Event) => {
  const raw = (event.target as HTMLInputElement | HTMLTextAreaElement).value;
  if (input.widget === "color") return update(input.id, Number.parseInt(raw.slice(1), 16));
  if (input.widget === "number") return update(input.id, raw === "" ? undefined : Number(raw));
  if (input.widget === "json") {
    if (!raw.trim()) {
      fieldErrors.value = { ...fieldErrors.value, [input.id]: "" };
      return update(input.id, undefined);
    }
    try {
      const parsed = JSON.parse(raw);
      fieldErrors.value = { ...fieldErrors.value, [input.id]: "" };
      return update(input.id, parsed);
    } catch {
      fieldErrors.value = { ...fieldErrors.value, [input.id]: "Enter valid JSON." };
      return;
    }
  }
  update(input.id, raw);
};

watch(fieldErrors, (errors) => emit("validity-change", !Object.values(errors).some(Boolean)), { deep: true, immediate: true });

watch(
  [visibleInputs, () => props.modelValue],
  async ([inputs]) => {
    if (!props.resolveOptions) return;
    for (const input of inputs as CapabilityParameter[]) {
      if (!input.optionSource) continue;
      try {
        const options = await props.resolveOptions(input, props.modelValue);
        dynamicOptions.value = { ...dynamicOptions.value, [input.id]: options };
      } catch {
        dynamicOptions.value = { ...dynamicOptions.value, [input.id]: input.options || [] };
      }
    }
  },
  { deep: true, immediate: true },
);
</script>

<style scoped>
.capability-control {
  width: 100%;
  border: 1px solid hsl(var(--border2));
  border-radius: 0.65rem;
  background: hsl(var(--background));
  color: hsl(var(--text));
  padding: 0.65rem 0.75rem;
  outline: none;
}
.capability-control:focus { border-color: hsl(var(--primary)); }
.capability-control:disabled { opacity: 0.55; cursor: not-allowed; }
</style>
