<template>
  <details class="mt-3 p-3 bg-themeBackground2 rounded-lg border border-themeBorder2">
    <summary class="cursor-pointer text-xs font-medium text-themeText2">Command inputs</summary>
    <p class="text-xs text-themeText3 my-2">Map chat values to the selected action's declared inputs. Omitted inputs use its defaults.</p>
    <div v-for="(binding, name) in modelValue" :key="name" class="flex gap-2 mt-2">
      <input :value="name" aria-label="Action input name" class="min-w-0 flex-1 px-2 py-1.5 bg-themeBackground3 border border-themeBorder2 rounded text-xs text-themeText" @change="renameInput(String(name), ($event.target as HTMLInputElement).value)">
      <select :value="binding.source" aria-label="Chat value source" class="min-w-0 flex-1 px-2 py-1.5 bg-themeBackground3 border border-themeBorder2 rounded text-xs text-themeText" @change="setSource(String(name), ($event.target as HTMLSelectElement).value)">
        <option value="args">Command text</option>
        <option value="argsNumber">Command number</option>
        <option value="argsBoolean">Command true / false</option>
        <option value="message">Full chat message</option>
        <option value="username">Sender login</option>
        <option value="displayName">Sender display name</option>
        <option value="platform">Platform</option>
        <option value="literal">Preset value</option>
      </select>
      <input v-if="binding.source === 'literal'" :value="JSON.stringify(binding.value)" aria-label="Preset JSON scalar" class="min-w-0 flex-1 px-2 py-1.5 bg-themeBackground3 border border-themeBorder2 rounded text-xs text-themeText" @change="setLiteral(String(name), ($event.target as HTMLInputElement).value)">
      <button type="button" class="text-xs text-themeText2 px-2" aria-label="Remove input mapping" @click="remove(String(name))">Remove</button>
    </div>
    <p v-if="error" role="alert" class="text-xs text-themeError mt-2">{{ error }}</p>
    <button type="button" class="text-xs text-themeButton mt-2" @click="add">Add input mapping</button>
  </details>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import type { CommandInputSource, InvocationScalar } from '@base/types/actionInputs';

const props = defineProps<{ modelValue: Record<string, CommandInputSource> }>();
const emit = defineEmits<{ 'update:modelValue': [value: Record<string, CommandInputSource>] }>();
const error = ref('');

function publish(value: Record<string, CommandInputSource>) {
  error.value = '';
  emit('update:modelValue', value);
}
function add() {
  let name = 'text';
  let index = 1;
  while (Object.hasOwn(props.modelValue, name)) name = `input${index++}`;
  publish({ ...props.modelValue, [name]: { source: 'args' } });
}
function remove(name: string) {
  const values = Object.fromEntries(Object.entries(props.modelValue).filter(([key]) => key !== name));
  publish(values);
}
function renameInput(previous: string, name: string) {
  const binding = props.modelValue[previous];
  if (!binding || name === previous) return;
  if (!name.trim() || Object.hasOwn(props.modelValue, name)) {
    error.value = 'Input names must be nonempty and distinct.';
    return;
  }
  const values = Object.fromEntries(Object.entries(props.modelValue).filter(([key]) => key !== previous));
  publish({ ...values, [name]: binding });
}
function setSource(name: string, source: string) {
  const supported = ['args', 'argsNumber', 'argsBoolean', 'message', 'username', 'displayName', 'platform'];
  if (source === 'literal') publish({ ...props.modelValue, [name]: { source, value: '' } });
  else if (supported.includes(source)) publish({ ...props.modelValue, [name]: { source } as CommandInputSource });
}
function setLiteral(name: string, text: string) {
  try {
    const value: unknown = JSON.parse(text);
    if (value !== null && !['string', 'number', 'boolean'].includes(typeof value)) throw new Error();
    publish({ ...props.modelValue, [name]: { source: 'literal', value: value as InvocationScalar } });
  } catch {
    error.value = 'Presets accept a JSON string, number, true, false or null.';
  }
}
</script>
