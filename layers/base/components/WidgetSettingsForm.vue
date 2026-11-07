<template>
  <form class="widget-settings" @submit.prevent="submit">
    <h3>{{ heading || 'Widget settings' }}</h3>
    <p v-if="schema.description" class="hint">{{ schema.description }}</p>
    <fieldset :disabled="busy">
      <template v-if="!jsonMode">
        <label v-for="field in fields" :key="field.key">
          <span>{{ field.schema.title || field.key }}{{ required.includes(field.key) ? ' *' : '' }}</span>
          <select v-if="Array.isArray(field.schema.enum)" :value="JSON.stringify(draft[field.key])" @change="setEnum(field.key, $event)">
            <option disabled value="">Choose a value</option>
            <option v-for="(value, index) in field.schema.enum" :key="index" :value="JSON.stringify(value)">{{ typeof value === 'string' ? value : JSON.stringify(value) }}</option>
          </select>
          <input v-else-if="field.schema.type === 'boolean'" type="checkbox" :checked="draft[field.key] === true" @change="draft[field.key] = ($event.target as HTMLInputElement).checked">
          <input v-else-if="field.schema.type === 'number' || field.schema.type === 'integer'" type="number" :value="draft[field.key]" :min="field.schema.minimum" :max="field.schema.maximum" :step="field.schema.multipleOf || (field.schema.type === 'integer' ? 1 : 'any')" @input="setNumber(field.key, $event)">
          <input v-else type="text" :value="draft[field.key] ?? ''" @input="draft[field.key] = ($event.target as HTMLInputElement).value">
          <small v-if="field.schema.description" class="hint">{{ field.schema.description }}</small>
        </label>
      </template>
      <label v-else>
        <span>Settings JSON</span>
        <textarea v-model="jsonDraft" rows="12" spellcheck="false" aria-label="Widget settings JSON" />
        <small class="hint">Structured settings use JSON. The widget schema validates the entire object when you apply.</small>
      </label>
      <button type="button" @click="toggleMode">{{ jsonMode ? 'Show fields' : 'Edit JSON' }}</button>
      <button type="submit">{{ busy ? (busyLabel || 'Saving…') : (submitLabel || 'Apply settings') }}</button>
      <button type="button" @click="reset">Discard edits</button>
    </fieldset>
    <p v-if="parseError || error" role="alert">{{ parseError || error }}</p>
    <p v-if="changedElsewhere" role="status">Saved settings changed while you were editing. Discard edits to load them, or apply to check for a conflict.</p>
  </form>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
const props = defineProps<{ schema: Record<string, any>; config: Record<string, unknown>; revision?: number; busy?: boolean; error?: string; heading?: string; submitLabel?: string; busyLabel?: string }>()
const emit = defineEmits<{ apply: [config: Record<string, unknown>, expectedRevision: number] }>()
const draft = ref<Record<string, unknown>>({})
const jsonDraft = ref('{}')
const jsonMode = ref(false)
const parseError = ref('')
const baseline = ref('{}')
const baselineRevision = ref(0)
const changedElsewhere = ref(false)
const dirty = computed(() => {
  try { return JSON.stringify(jsonMode.value ? JSON.parse(jsonDraft.value) : draft.value) !== baseline.value }
  catch { return true }
})
const fields = computed(() => Object.entries(props.schema.properties || {}).map(([key, schema]) => ({ key, schema: schema as Record<string, any> })))
const required = computed<string[]>(() => props.schema.required || [])
const needsJson = computed(() => fields.value.some(({ schema }) => !schema.enum && !['string', 'number', 'integer', 'boolean'].includes(schema.type)))
function reset() {
  draft.value = JSON.parse(JSON.stringify(props.config || {}))
  jsonDraft.value = JSON.stringify(draft.value, null, 2)
  parseError.value = ''
  baseline.value = JSON.stringify(draft.value)
  baselineRevision.value = props.revision || 0
  changedElsewhere.value = false
}
watch(() => [props.config, props.revision], () => {
  if (!dirty.value || props.busy) reset()
  else changedElsewhere.value = true
}, { immediate: true, deep: true })
watch(needsJson, value => { if (value) jsonMode.value = true }, { immediate: true })
function setNumber(key: string, event: Event) {
  const text = (event.target as HTMLInputElement).value
  if (text === '') delete draft.value[key]
  else draft.value[key] = Number(text)
}
function setEnum(key: string, event: Event) { draft.value[key] = JSON.parse((event.target as HTMLSelectElement).value) }
function parseJson(): Record<string, unknown> {
  const value = JSON.parse(jsonDraft.value)
  if (!value || Array.isArray(value) || typeof value !== 'object') throw new Error('Settings must be a JSON object.')
  return value
}
function toggleMode() {
  try {
    if (jsonMode.value) {
      if (needsJson.value) { parseError.value = 'This widget has structured fields. Use JSON to edit them.'; return }
      draft.value = parseJson()
    } else jsonDraft.value = JSON.stringify(draft.value, null, 2)
    jsonMode.value = !jsonMode.value
    parseError.value = ''
  } catch (error) { parseError.value = String(error) }
}
function submit() {
  try { parseError.value = ''; emit('apply', jsonMode.value ? parseJson() : JSON.parse(JSON.stringify(draft.value)), baselineRevision.value) }
  catch (error) { parseError.value = String(error) }
}
</script>

<style scoped>
.widget-settings { display: grid; gap: .65rem; font-size: .8rem; }
h3 { font-weight: 600; }
fieldset { display: grid; gap: .7rem; min-width: 0; }
label { display: grid; gap: .3rem; }
input:not([type=checkbox]), select, textarea { width: 100%; padding: .5rem; border: 1px solid currentColor; border-radius: .35rem; background: transparent; color: inherit; }
input[type=checkbox] { justify-self: start; }
textarea { font-family: monospace; resize: vertical; }
button { padding: .45rem; border: 1px solid currentColor; border-radius: .35rem; }
.hint { opacity: .7; }
[role=alert] { color: hsl(var(--error)); overflow-wrap: anywhere; }
</style>
