<template lang="pug">
section.widget-inspector
  header.widget-inspector__header
    span.widget-inspector__pill W
    div.widget-inspector__title-col
      strong.widget-inspector__title {{ definition.name }}
      span.widget-inspector__subtitle {{ definition.category }} widget
    button(type="button" class="widget-inspector__close" aria-label="Close widget settings" @click="$emit('close')") ×

  div.widget-inspector__body
    ContextSubmenu(v-if="schemaFields.length" :width="216" :preferred-side="submenuLeft ? 'left' : 'right'" menu-class="widget-inspector-floating-menu")
      template(#trigger="{ open, show }")
        button(type="button" class="widget-inspector__row" :class="{ 'widget-inspector__row--open': open }" @click="show")
          SolarSettingsLinear.widget-inspector__row-icon
          span Appearance & Settings
          SolarAltArrowRightLinear.widget-inspector__arrow
      div.widget-inspector__submenu-content
          p.widget-inspector__section-title Appearance & Settings
          label(v-for="field in schemaFields" :key="field.key" class="widget-inspector__field")
            span {{ field.title }}
            select(v-if="field.enumValues" :value="instance.config[field.key] ?? field.defaultValue ?? ''" @change="updateConfig(field, eventValue($event))")
              option(v-for="option in field.enumValues" :key="String(option)" :value="option") {{ option }}
            input(v-else-if="field.type === 'boolean'" type="checkbox" :checked="Boolean(instance.config[field.key] ?? field.defaultValue)" @change="updateConfig(field, eventChecked($event))")
            input(v-else :type="field.type === 'number' || field.type === 'integer' ? 'number' : 'text'" :min="field.minimum" :max="field.maximum" :step="field.type === 'integer' ? 1 : field.multipleOf" :value="instance.config[field.key] ?? field.defaultValue ?? ''" @change="updateConfig(field, eventValue($event))")
            small(v-if="field.description") {{ field.description }}

    div(v-if="schemaFields.length && definition.inputs.length" class="widget-inspector__divider")

    ContextSubmenu(
      v-for="port in definition.inputs"
      :key="port.id"
      :width="248"
      :preferred-side="submenuLeft ? 'left' : 'right'"
      menu-class="widget-inspector-floating-menu"
    )
      template(#trigger="{ open, show }")
        button(type="button" class="widget-inspector__row" :class="{ 'widget-inspector__row--open': open }" @click="show")
          SolarBoltLinear.widget-inspector__row-icon.widget-inspector__row-icon--binding
          span {{ port.id }} Binding
          small {{ bindingFor(port.id) ? policyLabel(policyTypeFor(port.id)) : port.kind }}
          SolarAltArrowRightLinear.widget-inspector__arrow
      div.widget-inspector__submenu-content
          div.widget-inspector__binding-head
            strong {{ port.id }}
            span {{ port.kind }}
          ActionPicker(:model-value="selectedActions[port.id] ?? null" :placeholder="bindingFor(port.id) ? 'Replace bound action…' : 'Search actions…'" @update:model-value="selectAction(port, $event)")
          select(v-if="policyOptions(port).length > 1" class="widget-inspector__policy" :value="policyTypeFor(port.id)" @change="changePolicy(port, eventValue($event))")
            option(v-for="policy in policyOptions(port)" :key="policy" :value="policy") {{ policyLabel(policy) }}
          div(v-if="policyTypeFor(port.id) === 'repeat'" class="widget-inspector__inline")
            label Hold ms
              input(type="number" min="0" max="5000" :value="repeatValue(port.id, 'holdMs', 300)" @change="changeRepeat(port, 'holdMs', $event)")
            label Interval ms
              input(type="number" min="25" max="60000" :value="repeatValue(port.id, 'intervalMs', 100)" @change="changeRepeat(port, 'intervalMs', $event)")
            label Max hold ms
              input(type="number" min="100" max="3600000" :value="repeatValue(port.id, 'maxHoldMs', 300000)" @change="changeRepeat(port, 'maxHoldMs', $event)")
          div(v-if="policyTypeFor(port.id) === 'queue'" class="widget-inspector__inline")
            label Maximum queue depth
              input(type="number" min="1" max="10000" :value="queueDepth(port.id)" @change="changeQueue(port, $event)")
          div(v-if="policyTypeFor(port.id) === 'toggle'" class="widget-inspector__toggle-actions")
            label.widget-inspector__checkbox
              input(type="checkbox" :checked="togglePersists(port.id)" @change="changeTogglePersistence(port, $event)")
              span Remember state after restart
            small Optional explicit state actions
            ActionPicker(placeholder="Action when enabled…" @update:model-value="selectToggleAction(port, 'onAction', $event)")
            ActionPicker(placeholder="Action when disabled…" @update:model-value="selectToggleAction(port, 'offAction', $event)")
          div(v-if="policyTypeFor(port.id) === 'continuous'" class="widget-inspector__inline")
            label Throttle ms
              input(type="number" min="0" max="5000" :value="continuousThrottle(port.id)" @change="changeContinuous(port, $event)")
          div(v-if="port.kind === 'value' && bindingFor(port.id)" class="widget-inspector__transform")
            small Value transform
            div.widget-inspector__inline
              label Output min
                input(type="number" :value="transformValue(port, 'outputMin')" @change="changeTransform(port, 'outputMin', $event)")
              label Output max
                input(type="number" :value="transformValue(port, 'outputMax')" @change="changeTransform(port, 'outputMax', $event)")
              label Curve
                select(:value="bindingFor(port.id)?.transform?.curve ?? 'linear'" @change="changeTransformCurve(port, $event)")
                  option(value="linear") Linear
                  option(value="logarithmic") Logarithmic
          small(v-if="bindingFor(port.id)") Bound · {{ bindingFor(port.id)?.policy.type }}
          button(v-if="bindingFor(port.id)" type="button" class="widget-inspector__clear" @click="$emit('unbind', port.id)") Clear binding

    div(v-if="!schemaFields.length && !definition.inputs.length" class="widget-inspector__empty") No editable settings or input ports.

    template(v-if="removable")
      div.widget-inspector__divider
      button(type="button" class="widget-inspector__row widget-inspector__row--danger" @click="$emit('remove')")
        SolarTrashBinTrashLinear.widget-inspector__row-icon
        span Remove widget
</template>

<script setup lang="ts">
import { computed, reactive } from "vue";
import ActionPicker from "@desktop/components/ActionPicker.vue";
import ContextSubmenu from "@desktop/components/ContextSubmenu.vue";
import SolarAltArrowRightLinear from "~icons/solar/alt-arrow-right-linear?width=16px&height=16px";
import SolarBoltLinear from "~icons/solar/bolt-linear?width=20px&height=20px";
import SolarSettingsLinear from "~icons/solar/settings-linear?width=20px&height=20px";
import SolarTrashBinTrashLinear from "~icons/solar/trash-bin-trash-linear?width=20px&height=20px";
import type {
  DeckWidgetInstance,
  WidgetBinding,
  WidgetExecutionPolicy,
  WidgetInputPort,
} from "@base/types/deckWidget";
import type { WidgetDefinition } from "@base/types/deckWidget";

type ActionSelection = { type: string; id: string; label: string; value: string };
type SchemaField = {
  key: string;
  title: string;
  description?: string;
  type: string;
  enumValues?: unknown[];
  defaultValue?: unknown;
  minimum?: number;
  maximum?: number;
  multipleOf?: number;
};

const props = withDefaults(defineProps<{
  instance: DeckWidgetInstance;
  definition: WidgetDefinition;
  removable?: boolean;
  submenuLeft?: boolean;
}>(), {
  removable: true,
  submenuLeft: false,
});

const emit = defineEmits<{
  (event: "configure", patch: Record<string, unknown>): void;
  (event: "bind", binding: WidgetBinding): void;
  (event: "unbind", portId: string): void;
  (event: "remove"): void;
  (event: "close"): void;
}>();

const selectedActions = reactive<Record<string, ActionSelection | null>>({});
const eventValue = (event: Event) => (event.target as HTMLInputElement | HTMLSelectElement).value;
const eventChecked = (event: Event) => (event.target as HTMLInputElement).checked;

const schemaFields = computed<SchemaField[]>(() => {
  const properties = props.definition.configSchema?.properties;
  if (!properties || typeof properties !== "object" || Array.isArray(properties)) return [];
  return Object.entries(properties as Record<string, Record<string, unknown>>).map(([key, schema]) => ({
    key,
    title: typeof schema.title === "string" ? schema.title : key,
    description: typeof schema.description === "string" ? schema.description : undefined,
    type: typeof schema.type === "string" ? schema.type : "string",
    enumValues: Array.isArray(schema.enum) ? schema.enum : undefined,
    defaultValue: schema.default,
    minimum: typeof schema.minimum === "number" ? schema.minimum : undefined,
    maximum: typeof schema.maximum === "number" ? schema.maximum : undefined,
    multipleOf: typeof schema.multipleOf === "number" ? schema.multipleOf : undefined,
  }));
});

const bindingFor = (portId: string) => props.instance.bindings.find(
  (binding) => binding.portId === portId,
);

const convertConfigValue = (field: SchemaField, value: unknown) => {
  if (field.type === "number" || field.type === "integer") {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : undefined;
  }
  return value;
};

const updateConfig = (field: SchemaField, value: unknown) => {
  emit("configure", { [field.key]: convertConfigValue(field, value) });
};

const policyOptions = (port: WidgetInputPort): WidgetExecutionPolicy["type"][] => {
  if (port.kind === "value") return ["continuous"];
  if (port.kind === "command") return ["run_once"];
  return port.supportedPolicies.map((policy) => (
    policy === "repeat_while_pressed" ? "repeat" : policy
  ));
};

const defaultPolicy = (port: WidgetInputPort): WidgetExecutionPolicy => {
  const type = policyOptions(port)[0] ?? "run_once";
  if (type === "repeat") return { type, holdMs: 300, intervalMs: 100 };
  if (type === "continuous") return { type, throttleMs: 50, commitOnRelease: true };
  return { type } as WidgetExecutionPolicy;
};

const policyTypeFor = (portId: string) => bindingFor(portId)?.policy.type ?? "run_once";
const policyLabel = (policy: string) => ({
  run_once: "Run once",
  repeat: "Hold to repeat",
  toggle: "Toggle",
  queue: "Queue",
  continuous: "Continuous value",
}[policy] ?? policy);

const emitBinding = (port: WidgetInputPort, action: unknown, policy: WidgetExecutionPolicy) => {
  emit("bind", {
    id: bindingFor(port.id)?.id ?? `${props.instance.id}:${port.id}`,
    portId: port.id,
    action,
    policy,
    transform: bindingFor(port.id)?.transform,
  });
};

const selectAction = (port: WidgetInputPort, selection: ActionSelection | null) => {
  selectedActions[port.id] = selection;
  if (!selection) return;
  emitBinding(port, selection.value, bindingFor(port.id)?.policy ?? defaultPolicy(port));
};

const policyFromType = (type: string): WidgetExecutionPolicy => {
  if (type === "repeat") return { type, holdMs: 300, intervalMs: 100 };
  if (type === "continuous") return { type, throttleMs: 50, commitOnRelease: true };
  if (type === "toggle") return { type };
  if (type === "queue") return { type };
  return { type: "run_once" };
};

const changePolicy = (port: WidgetInputPort, type: string) => {
  const binding = bindingFor(port.id);
  if (!binding) return;
  emitBinding(port, binding.action, policyFromType(type));
};

const repeatValue = (portId: string, key: "holdMs" | "intervalMs" | "maxHoldMs", fallback: number) => {
  const policy = bindingFor(portId)?.policy;
  return policy?.type === "repeat" ? policy[key] : fallback;
};

const changeRepeat = (port: WidgetInputPort, key: "holdMs" | "intervalMs" | "maxHoldMs", event: Event) => {
  const binding = bindingFor(port.id);
  if (!binding) return;
  const current = binding.policy.type === "repeat"
    ? binding.policy
    : { type: "repeat" as const, holdMs: 300, intervalMs: 100 };
  emitBinding(port, binding.action, {
    ...current,
    [key]: Number((event.target as HTMLInputElement).value),
  });
};

const queueDepth = (portId: string) => {
  const policy = bindingFor(portId)?.policy;
  return policy?.type === "queue" ? policy.maxDepth ?? 100 : 100;
};

const changeQueue = (port: WidgetInputPort, event: Event) => {
  const binding = bindingFor(port.id);
  if (!binding) return;
  emitBinding(port, binding.action, {
    type: "queue",
    maxDepth: Number((event.target as HTMLInputElement).value),
  });
};

const selectToggleAction = (
  port: WidgetInputPort,
  branch: "onAction" | "offAction",
  selection: ActionSelection | null,
) => {
  const binding = bindingFor(port.id);
  if (!binding || binding.policy.type !== "toggle") return;
  emitBinding(port, binding.action, {
    ...binding.policy,
    [branch]: selection?.value,
  });
};

const togglePersists = (portId: string) => {
  const policy = bindingFor(portId)?.policy;
  return policy?.type === "toggle" && policy.persistState === true;
};

const changeTogglePersistence = (port: WidgetInputPort, event: Event) => {
  const binding = bindingFor(port.id);
  if (!binding || binding.policy.type !== "toggle") return;
  emitBinding(port, binding.action, {
    ...binding.policy,
    persistState: (event.target as HTMLInputElement).checked,
  });
};

const continuousThrottle = (portId: string) => {
  const policy = bindingFor(portId)?.policy;
  return policy?.type === "continuous" ? policy.throttleMs : 50;
};

const changeContinuous = (port: WidgetInputPort, event: Event) => {
  const binding = bindingFor(port.id);
  if (!binding) return;
  emitBinding(port, binding.action, {
    type: "continuous",
    throttleMs: Number((event.target as HTMLInputElement).value),
    commitOnRelease: true,
  });
};

const transformValue = (port: WidgetInputPort, key: "outputMin" | "outputMax") => {
  const transform = bindingFor(port.id)?.transform;
  if (transform) return transform[key];
  if (port.kind !== "value") return 0;
  return key === "outputMin" ? port.min : port.max;
};

const nextTransform = (port: WidgetInputPort) => {
  const binding = bindingFor(port.id);
  if (!binding || port.kind !== "value") return undefined;
  return binding.transform ?? {
    inputMin: port.min,
    inputMax: port.max,
    outputMin: port.min,
    outputMax: port.max,
    curve: "linear" as const,
  };
};

const changeTransform = (
  port: WidgetInputPort,
  key: "outputMin" | "outputMax",
  event: Event,
) => {
  const binding = bindingFor(port.id);
  const transform = nextTransform(port);
  if (!binding || !transform) return;
  emit("bind", {
    ...binding,
    transform: { ...transform, [key]: Number((event.target as HTMLInputElement).value) },
  });
};

const changeTransformCurve = (port: WidgetInputPort, event: Event) => {
  const binding = bindingFor(port.id);
  const transform = nextTransform(port);
  if (!binding || !transform) return;
  emit("bind", {
    ...binding,
    transform: {
      ...transform,
      curve: (event.target as HTMLSelectElement).value as "linear" | "logarithmic",
    },
  });
};
</script>

<style scoped>
.widget-inspector {
  width: 100%;
  color: hsl(var(--text));
  user-select: none;
}

.widget-inspector__header {
  display: flex;
  align-items: center;
  gap: .55rem;
  padding: .55rem .75rem;
  border-bottom: 1px solid hsl(var(--border2) / .7);
  border-radius: .85rem .85rem 0 0;
  background: hsl(var(--editor-surface-soft, var(--background2)) / .65);
}

.widget-inspector__pill {
  border: 1px solid hsl(var(--button) / .35);
  border-radius: .35rem;
  background: hsl(var(--button) / .18);
  padding: .12rem .42rem;
  color: hsl(var(--button));
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: .65rem;
  font-weight: 800;
}

.widget-inspector__title-col {
  display: flex;
  min-width: 0;
  flex: 1 1 auto;
  flex-direction: column;
}

.widget-inspector__title {
  overflow: hidden;
  font-size: .78rem;
  line-height: 1.2;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.widget-inspector__subtitle {
  color: hsl(var(--text3));
  font-size: .65rem;
  text-transform: capitalize;
}

.widget-inspector__close {
  display: grid;
  width: 1.5rem;
  height: 1.5rem;
  place-items: center;
  border-radius: 999px;
  color: hsl(var(--text3));
  font-size: .9rem;
}

.widget-inspector__close:hover,
.widget-inspector__row:hover,
.widget-inspector__row--open {
  background: hsl(var(--hover));
}

.widget-inspector__body {
  display: flex;
  flex-direction: column;
  gap: .12rem;
  padding: .32rem;
}

.widget-inspector__row {
  display: flex;
  width: 100%;
  align-items: center;
  gap: .6rem;
  border-radius: .55rem;
  padding: .45rem .6rem;
  color: hsl(var(--text));
  font-size: .8rem;
  font-weight: 550;
  text-align: left;
  transition: background .15s ease, color .15s ease;
}

.widget-inspector__row > span:not(.widget-inspector__subtitle) {
  min-width: 0;
  flex: 1 1 auto;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.widget-inspector__row small {
  color: hsl(var(--text3));
  font-size: .6rem;
  text-transform: capitalize;
  white-space: nowrap;
}

.widget-inspector__row-icon,
.widget-inspector__arrow {
  width: 1.15rem;
  height: 1.15rem;
  flex: 0 0 auto;
  color: rgb(192 132 252);
}

.widget-inspector__row-icon--binding {
  color: rgb(251 191 36);
}

.widget-inspector__arrow {
  width: .875rem;
  height: .875rem;
  color: hsl(var(--text3));
}

.widget-inspector__row--danger:hover {
  background: hsl(var(--error) / .14);
  color: hsl(var(--error));
}

.widget-inspector__row--danger .widget-inspector__row-icon {
  color: rgb(248 113 113);
}

.widget-inspector__divider {
  height: 1px;
  margin: .18rem .2rem;
  background: hsl(var(--border2) / .6);
}

.widget-inspector__submenu-content {
  display: grid;
  gap: .45rem;
  padding: .33rem;
}

.widget-inspector__section-title {
  margin: 0;
  color: hsl(var(--text3));
  font-size: .62rem;
  font-weight: 750;
  letter-spacing: .08em;
  text-transform: uppercase;
}

.widget-inspector__binding-head,
.widget-inspector__inline {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: .5rem;
}

.widget-inspector__binding-head span,
.widget-inspector__submenu-content small {
  color: hsl(var(--text3));
  font-size: .68rem;
}

.widget-inspector__field {
  display: grid;
  gap: .3rem;
  font-size: .72rem;
}

input,
select {
  min-width: 0;
  border: 1px solid hsl(var(--border2) / .9);
  border-radius: .45rem;
  background: hsl(var(--background2));
  padding: .42rem .5rem;
  color: hsl(var(--text));
}

input[type="checkbox"] {
  justify-self: start;
}

.widget-inspector__policy {
  width: 100%;
  font-size: .72rem;
}

.widget-inspector__inline label {
  display: grid;
  min-width: 0;
  gap: .2rem;
  font-size: .68rem;
}

.widget-inspector__inline input {
  width: 100%;
}

.widget-inspector__empty {
  padding: .45rem .6rem;
  color: hsl(var(--text3));
  font-size: .72rem;
}

.widget-inspector__toggle-actions,
.widget-inspector__transform {
  display: grid;
  gap: .4rem;
}

.widget-inspector__clear {
  justify-self: start;
  padding: .2rem 0;
  color: rgb(252 165 165);
  font-size: .7rem;
}

.widget-inspector__checkbox {
  display: flex;
  align-items: center;
  gap: .4rem;
  font-size: .7rem;
}

</style>
