<template>
  <details class="rounded-lg border border-themeBorder2 p-3 text-themeText">
    <summary class="cursor-pointer text-sm font-medium">Reusable action inputs</summary>
    <p class="text-xs text-themeText3 my-2">Declare values callers may supply. Bind them to selected fields; the action's target and credentials remain fixed.</p>
    <p v-if="!reference" class="text-xs text-themeText3">Save the action first, then edit its input definitions.</p>
    <fieldset v-else :disabled="busy" class="grid gap-2 mt-3">
      <label class="flex gap-2 text-xs"><input v-model="enabled" type="checkbox">Enable declared inputs</label>
      <template v-if="enabled">
        <label class="text-xs">Input schema (JSON Schema draft 7)
          <textarea v-model="schemaText" aria-label="Action input schema" rows="8" spellcheck="false" class="w-full mt-1 p-2 rounded border border-themeBorder2 bg-themeBackground text-themeText font-mono" />
        </label>
        <label v-if="reference.kind !== 'flow'" class="text-xs">Field bindings
          <textarea v-model="bindingsText" aria-label="Action input bindings" rows="5" spellcheck="false" class="w-full mt-1 p-2 rounded border border-themeBorder2 bg-themeBackground text-themeText font-mono" />
        </label>
        <p class="text-xs text-themeText3">Inputs retain their types. Use schema defaults for omitted values. A flow receives the declared fields as its read-only inputs variable.</p>
        <details v-if="reference.kind === 'http_request'" class="text-xs text-themeText3">
          <summary class="cursor-pointer">HTTP example</summary>
          <p class="mt-1">For a saved body containing a text field:</p>
          <pre class="overflow-auto mt-1">{{ example }}</pre>
          <p>Supported destinations: JSON body fields, named query values and individual path segments.</p>
        </details>
      </template>
      <div class="flex gap-2">
        <button type="button" class="px-3 py-1.5 rounded bg-themeButton text-white text-xs" @click="save">{{ busy ? 'Saving…' : 'Save input definitions' }}</button>
        <button type="button" class="px-3 py-1.5 rounded border border-themeBorder2 text-xs" @click="load">Reload saved definitions</button>
      </div>
      <p v-if="message" role="status" class="text-xs text-themeText2">{{ message }}</p>
      <p v-if="error" role="alert" class="text-xs text-themeError">{{ error }}</p>
    </fieldset>
  </details>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { ActionReference } from '@base/types/actionReference';
import type { ActionInputContract, InputContractSnapshot } from '@base/types/actionInputs';

const props = defineProps<{ reference: ActionReference | null }>();
const emit = defineEmits<{ saved: [snapshot: InputContractSnapshot] }>();
const enabled = ref(false);
const schemaText = ref('{}');
const bindingsText = ref('[]');
const revision = ref(0);
const busy = ref(false);
const error = ref('');
const message = ref('');
let loadGeneration = 0;
const example = JSON.stringify({
  inputSchema: { type: 'object', properties: { text: { type: 'string', maxLength: 500 } }, required: ['text'] },
  bindings: [{ input: 'text', target: { type: 'httpBody', pointer: '/text' } }],
}, null, 2);

async function load() {
  const generation = ++loadGeneration;
  const reference = props.reference;
  error.value = ''; message.value = '';
  if (!reference) return;
  busy.value = true;
  try {
    const snapshot = await invoke<InputContractSnapshot>('get_action_input_contract', { reference });
    if (generation !== loadGeneration) return;
    revision.value = snapshot.revision;
    enabled.value = snapshot.contract !== null;
    schemaText.value = JSON.stringify(snapshot.contract?.inputSchema ?? { type: 'object', properties: {}, additionalProperties: false }, null, 2);
    bindingsText.value = JSON.stringify(snapshot.contract?.bindings ?? [], null, 2);
  } catch (cause) {
    if (generation === loadGeneration) error.value = String(cause);
  } finally {
    if (generation === loadGeneration) busy.value = false;
  }
}
async function save() {
  if (!props.reference || busy.value) return;
  busy.value = true; error.value = ''; message.value = '';
  const generation = loadGeneration;
  const reference = props.reference;
  try {
    const contract: ActionInputContract | null = enabled.value ? {
      version: 1, inputSchema: JSON.parse(schemaText.value),
      bindings: props.reference.kind === 'flow' ? [] : JSON.parse(bindingsText.value),
    } : null;
    const snapshot = await invoke<InputContractSnapshot>('save_action_input_contract', {
      reference, expectedRevision: revision.value, contract,
    });
    if (generation !== loadGeneration) return;
    revision.value = snapshot.revision;
    message.value = 'Input definitions saved.';
    emit('saved', snapshot);
  } catch (cause) { if (generation === loadGeneration) error.value = String(cause); }
  finally { if (generation === loadGeneration) busy.value = false; }
}
watch(() => props.reference ? `${props.reference.kind}:${props.reference.id}` : '', load, { immediate: true });
</script>
