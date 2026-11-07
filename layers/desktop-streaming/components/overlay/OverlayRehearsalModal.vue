<template>
  <OverlayModal :show="show" title="Rehearse draft" subtitle="Production widget renderer · sample events · native actions and API requests disabled" modal-class="max-w-6xl max-h-[90vh]" @close="$emit('close')">
    <div class="flex flex-col gap-3 p-4 min-h-0">
      <div class="flex items-center gap-3 text-sm text-themeText">
        <label>Sample event <select v-model="sampleType" class="bg-themeBackground2 border border-themeBorder rounded p-1"><option value="follow">Follow</option><option value="chatMessage">Chat</option><option value="raid">Raid</option><option value="subscription">Subscription</option></select></label>
        <button class="bg-themeButton text-white rounded px-3 py-1" @click="replay">Play sample</button>
        <button class="border border-themeBorder rounded px-3 py-1" @click="restart">Reset rehearsal</button>
        <span class="text-xs text-themeText2">{{ width }}×{{ height }} · {{ networkLimited ? 'Widgets requiring API access cannot fetch in rehearsal.' : 'Live output is unchanged.' }}</span>
      </div>
      <div class="bg-themeBackground2 border border-themeBorder rounded overflow-hidden" :style="{ aspectRatio: `${width} / ${height}` }">
        <iframe ref="preview" title="Isolated overlay rehearsal" sandbox="allow-scripts" :srcdoc="html" class="w-full h-full border-0" @load="hydrate" />
      </div>
      <ul v-if="diagnostics.length" class="text-xs text-themeText2 list-disc pl-5"><li v-for="(issue,index) in diagnostics" :key="index">{{ issue.elementId || 'Composition' }}: {{ issue.message }}</li></ul>
      <p v-if="error" role="alert" class="text-sm text-themeText">{{ error }}</p>
    </div>
  </OverlayModal>
</template>

<script setup lang="ts">
import { computed, ref, watch, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import OverlayModal from './OverlayModal.vue'
import { useOverlayV3Sync } from '../../composables/overlay/useOverlayV3Sync'
import { widgetSubscribeToCloudEvents } from '../../composables/overlay/packageManifest'
import { cloneComposition, type CompositionContent } from '../../composables/overlay/useOverlayCompositionLibrary'

const props = defineProps<{ show: boolean; content: CompositionContent }>()
defineEmits<{ close: [] }>()
const sync = useOverlayV3Sync({ live: () => false })
const preview = ref<HTMLIFrameElement | null>(null)
const html = ref('')
const sampleType = ref('follow')
const error = ref('')
const diagnostics = ref<{elementId?:string;message:string}[]>([])
let snapshot: CompositionContent | null = null
let generation = 0
const width = computed(() => Number(props.content.settings.canvasWidth) || 1920)
const height = computed(() => Number(props.content.settings.canvasHeight) || 1080)
const networkLimited = computed(() => props.content.elements.some(element => Boolean((element.capabilities as {network?:unknown})?.network)))
const inline = (value: unknown) => JSON.stringify(value).replaceAll('<', '\\u003c').replaceAll('\u2028', '\\u2028').replaceAll('\u2029', '\\u2029')

async function restart() {
  const request = ++generation
  error.value = ''
  snapshot = cloneComposition(props.content)
  try {
    const issues = await invoke<{elementId?:string;message:string}[]>('validate_overlay_composition', { content: snapshot })
    await sync.refreshWidgetGrants()
    if (request !== generation) return
    diagnostics.value = issues
    const elements = snapshot.elements.map(element => `<div id="${element.id}" data-instance-id="${element.id}" style="position:absolute;left:${Number(element.x)}px;top:${Number(element.y)}px;width:${Number(element.width)}px;height:${Number(element.height)}px;z-index:${Number(element.zIndex) || 0};display:${element.visible === false ? 'none' : 'block'}">${sync.generateElementHTML(element, true)}</div>`).join('')
    const descriptors = snapshot.elements.map(element => ({ id: element.id, config: element.config || {}, revision: element.draftConfigRevision ?? 0, subscriptions: (element.capabilities as {subscribe?:string[]})?.subscribe || [], package: Boolean(element.widgetId) }))
    html.value = `<!doctype html><html><head><meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'unsafe-inline' 'unsafe-eval'; style-src 'unsafe-inline'; frame-src 'self' about:; img-src data: blob: https:; media-src https:; font-src data: https:; connect-src 'none'; form-action 'none'; base-uri 'none'"><style>html,body{margin:0;width:100%;height:100%;overflow:hidden;background:transparent}#canvas{position:absolute;width:${width.value}px;height:${height.value}px;transform-origin:top left}</style></head><body><div id="canvas">${elements}</div><script>
      const descriptors=${inline(descriptors)}, mapping=${inline(widgetSubscribeToCloudEvents)};
      const frames=()=>Array.from(document.querySelectorAll('iframe'));
      const descriptorFor=frame=>descriptors.find(item=>item.id===frame.closest('[data-instance-id]')?.dataset.instanceId);
      function hydrate(){frames().forEach(frame=>{const descriptor=descriptorFor(frame); if(!descriptor)return; frame.contentWindow.postMessage({type:'UPDATE_CONFIG',version:1,payload:descriptor.config,revision:descriptor.revision},'*')})}
      frames().forEach(frame=>frame.addEventListener('load',hydrate));
      function fit(){document.getElementById('canvas').style.transform='scale('+Math.min(innerWidth/${width.value},innerHeight/${height.value})+')'};
      addEventListener('resize',fit);fit();
      addEventListener('message',event=>{if(event.source!==parent)return; if(event.data?.type==='REHEARSAL_HYDRATE')hydrate(); if(event.data?.type!=='REHEARSAL_SAMPLE')return; frames().forEach(frame=>{const descriptor=descriptorFor(frame);if(!descriptor)return;const sample=event.data.payload;if(descriptor.package&&!descriptor.subscriptions.some(cap=>(mapping[cap]||[]).includes(sample.cloudType)))return;frame.contentWindow.postMessage({category:'stream',payload:sample},'*')})});
    ${'<' + '/script>'}</body></html>`
    await nextTick()
  } catch (cause) { if (request === generation) error.value = String(cause) }
}

function hydrate() { preview.value?.contentWindow?.postMessage({ type: 'REHEARSAL_HYDRATE' }, '*') }
function replay() {
  const type = sampleType.value
  preview.value?.contentWindow?.postMessage({ type: 'REHEARSAL_SAMPLE', payload: {
    type, cloudType: type === 'chatMessage' ? 'chat_message' : type,
    platform: 'twitch', userName: 'sample_viewer', userDisplayName: 'Sample Viewer',
    message: 'Hello from rehearsal!', viewerCount: 42, amount: 1,
    eventId: `rehearsal-${Date.now()}`, timestamp: new Date().toISOString(), rehearsal: true,
  } }, '*')
}
watch(() => props.show, show => { if (show) void restart(); else { ++generation; html.value = ''; snapshot = null } }, { immediate: true })
</script>
