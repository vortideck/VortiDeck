<template>
  <!-- Javascript/WebGL Sandbox -->
  <div v-if="element.pendingImportReview" class="p-3 text-sm text-themeText bg-themeBackground2">Imported custom content — review before activation</div>
  <iframe v-else-if="needsSandbox"
    ref="sandboxIframe"
    @load="onIframeLoad"
    sandbox="allow-scripts"
    scrolling="no"
    frameborder="0"
    :srcdoc="sandboxSrcDoc"
    :data-subscribe="element?.capabilities?.subscribe?.length ? element.capabilities.subscribe.join(',') : undefined"
    :data-widget-id="element?.widgetId"
    style="width: 100%; height: 100%; border: none; background: transparent; overflow: hidden; pointer-events: none;"
    :style="elementStyles"
  ></iframe>

  <!-- Only first-party schema templates on builtin elements use host v-html -->
  <div v-else-if="hostSchemaHtml !== undefined"
    :style="elementStyles"
    v-html="hostSchemaHtml"
  />

  <!-- Everything else renders through bound attributes and text nodes -->
  <div v-else :style="elementStyles">
    <img v-if="imageSrc"
      :src="imageSrc"
      :alt="element.name || ''"
      :style="{ width: '100%', height: '100%', objectFit: safeObjectFit(element.objectFit) }"
    />
    <template v-else>{{ elementText }}</template>
  </div>
</template>

<script setup lang="ts">
import { widgetSettingsBridgeScript, widgetConfigMessage } from '@base/utils/widgetSettingsBridge'
import { computed, ref, onMounted, onUnmounted, watch } from 'vue'
import { useElementBindings } from '@desktopStreaming/composables/overlay/useElementBindings'
import { getComponentSchema, buildComponentVars } from '@desktopStreaming/composables/overlay/componentSchema'
import { mediaData } from '@desktopStreaming/composables/overlay/mediaTemplate'
import { useOverlayTheme } from '@desktopStreaming/composables/overlay/useOverlayTheme'
import { useStreamingRust } from '@desktopStreaming/composables/streaming/useStreamingRust'
import { useSessionManager } from '@desktopStreaming/composables/streaming/useSessionManager'
import { cssVarDeclarations, elementNeedsSandbox, safeObjectFit } from '@desktopStreaming/composables/overlay/useOverlayV3Sync'

interface Props {
  element: {
    id: string
    type?: string
    content?: string
    customHTML?: string
    innerHTML?: string
    name?: string
    src?: string
    imageUrl?: string
    objectFit?: string
    style?: Record<string, any>
    // Modular element properties
    template?: string
    bindings?: Record<string, string>
    dynamicStyles?: Record<string, string>
    modularType?: string
    // Any other properties that should be applied as CSS
    [key: string]: any
  }
}

const props = defineProps<Props>()

// Legacy saved health/mana elements remain renderable. Their values are now
// stable element data instead of an editor-owned fake-data timer.
const healthValue = computed(() => Number(props.element.data?.health ?? 90))
const maxHealth = computed(() => Math.max(1, Number(props.element.data?.maxHealth ?? 120)))
const manaValue = computed(() => Number(props.element.data?.mana ?? 48))
const maxMana = computed(() => Math.max(1, Number(props.element.data?.maxMana ?? 80)))
const healthPercent = computed(() => Math.max(0, Math.min(100, (healthValue.value / maxHealth.value) * 100)))
const manaPercent = computed(() => Math.max(0, Math.min(100, (manaValue.value / maxMana.value) * 100)))
const level = computed(() => Number(props.element.data?.level ?? 23))

const { renderElement } = useElementBindings()

let unwatchChat: (() => void) | null = null

// Support local reactive element data to avoid prop mutation overwrites
const elementDataLocal = ref<Record<string, any>>({ ...(props.element.data || {}) })

watch(() => props.element.data, (newData) => {
  elementDataLocal.value = { ...elementDataLocal.value, ...(newData || {}) }
}, { deep: true, immediate: true })

onMounted(async () => {
  if (props.element.type === 'chat') {
    try {
      const streamingRust = useStreamingRust()
      const sessionManager = useSessionManager()

      const setupChatSession = async (active: any) => {
        // Unsubscribe from any previous message watcher
        if (unwatchChat) {
          unwatchChat()
          unwatchChat = null
        }

        if (!active) {
          console.log('🎬 [UniversalElement] No active session found to load chat')
          elementDataLocal.value = { ...elementDataLocal.value, chatHistory: [] }
          return
        }

        console.log('🎬 ACTIVE SESSION FOR HISTORICAL CHAT:', JSON.stringify(active, null, 2))
        if (active.platforms) {
          console.log('🎬 ACTIVE PLATFORMS:', JSON.stringify(active.platforms, null, 2))
          const accountIds = active.platforms
            .map((p: any) => Number(p.accountId || p.account_id))
            .filter((id: number) => !isNaN(id))

          console.log('🎬 EXTRACTED ACCOUNT IDS:', accountIds)
          if (accountIds.length > 0) {
            const historical = await streamingRust.getMessagesForAccounts(accountIds, 20)
            console.log('🎬 RETRIEVED HISTORICAL MESSAGES COUNT:', historical.length)
            elementDataLocal.value = { ...elementDataLocal.value, chatHistory: historical }
          } else {
            elementDataLocal.value = { ...elementDataLocal.value, chatHistory: [] }
          }
        } else {
          elementDataLocal.value = { ...elementDataLocal.value, chatHistory: [] }
        }

        // Subscribe to new messages
        unwatchChat = streamingRust.onMessage((msg) => {
          // Only append if it belongs to active session's platform accounts
          if (active && active.platforms) {
            const accountIds = active.platforms.map((p: any) => String(p.accountId || p.account_id))
            if (!accountIds.includes(String(msg.account_id))) {
              return
            }
          }
          elementDataLocal.value = { ...elementDataLocal.value, chatMessage: msg }
        })
      }

      // Load initial sessions
      await sessionManager.loadSessions()

      // Watch for active session changes and dynamically hot-reload history
      watch(
        () => {
          const active = sessionManager.activeSessions.value[0] || sessionManager.recentSessions.value[0] || sessionManager.sessions.value[0]
          return active?.id || null
        },
        async (newSessionId) => {
          console.log('🎬 [UniversalElement] Active session changed, loading:', newSessionId)
          await sessionManager.loadSessions()
          const active = sessionManager.sessions.value.find(s => s.id === newSessionId) ||
                         sessionManager.activeSessions.value[0] ||
                         sessionManager.recentSessions.value[0] ||
                         sessionManager.sessions.value[0]
          await setupChatSession(active)
        },
        { immediate: true }
      )
    } catch (err) {
      console.error('Failed to setup dynamic chat preview inside UniversalElement:', err)
    }
  }

})
onUnmounted(() => {
  if (unwatchChat) {
    unwatchChat()
  }
})

const legacyFunctionData = computed<Record<string, Record<string, unknown>>>(() => ({
  'game-stats': {
    health: healthValue.value,
    maxHealth: maxHealth.value,
    healthPercent: healthPercent.value,
    mana: manaValue.value,
    maxMana: maxMana.value,
    manaPercent: manaPercent.value,
    level: level.value,
  },
}))

/**
 * Element-supplied markup (including legacy modular templates). It is only
 * ever rendered inside the sandboxed iframe, never by host v-html.
 */
const sandboxSourceHtml = (schema?: { template?: string }): string => {
  const element = props.element

  // Handle modular elements with templates
  if (element.type === 'modular' && element.template && element.bindings) {
    const dynamicElement = {
      id: element.id,
      type: element.modularType || 'custom',
      template: element.template,
      bindings: element.bindings,
      dynamicStyles: element.dynamicStyles || {},
      style: element.style || {},
      x: 0,
      y: 0,
      width: 200,
      height: 100
    }

    // Transform saved function bindings using stable element data.
    const transformedData: Record<string, any> = {}

    Object.entries(dynamicElement.bindings).forEach(([templateBinding, functionId]) => {
      if (legacyFunctionData.value[functionId]) {
        transformedData[templateBinding] = legacyFunctionData.value[functionId]
      }
    })

    const rendered = renderElement(dynamicElement, transformedData)
    return rendered.html
  }

  // A schema ID is the versioned first-party renderer. Prefer the current
  // reviewed template over the creation-time HTML snapshot so saved overlays
  // receive security and visual fixes without losing their data/style values.
  return schema?.template || element.customHTML || element.innerHTML
    || (typeof element.content === 'string' && /[<&]/.test(element.content) ? element.content : '')
}

/** First-party schema template of a builtin element: the only host v-html. */
const hostSchemaHtml = computed<string | undefined>(() => {
  if (needsSandbox.value) return undefined
  const schema = props.element.schemaId ? getComponentSchema(props.element.schemaId) : undefined
  if (!schema?.template) return undefined
  const mergedData = { ...(schema.defaultData || {}), ...(props.element.data || {}) }
  return applyDataSubstitutions(schema.template, mergedData)
})

const imageSrc = computed(() => props.element.content ? undefined : (props.element.src || props.element.imageUrl || undefined))
const elementText = computed(() => props.element.content || props.element.name || props.element.type || 'Element')

// --- JS Sandbox Implementation ---
const { cssVars } = useOverlayTheme()
const sandboxIframe = ref<HTMLIFrameElement | null>(null)
const iframeReady = ref(false)
// `srcdoc` is the iframe's executable identity and must stay stable while a
// widget is running. Theme changes are delivered below through UPDATE_VARS so
// stateful widgets recolor without being reconstructed.
const initialThemeVars = { ...cssVars.value }

// The host webview has Tauri IPC: element-supplied or non-builtin markup must
// never fall through to host-page v-html (see elementNeedsSandbox).
const needsSandbox = computed(() => elementNeedsSandbox(
  props.element,
  props.element.schemaId ? getComponentSchema(props.element.schemaId) : undefined,
))

// Non-reactive initial copy of element data and styles to keep the iframe srcdoc static and avoid reloading
const initialElementData = { ...(props.element.data || {}) }
const initialElementStyle = { ...(props.element.style || {}) }

const sandboxSrcDoc = computed(() => {
  if (!needsSandbox.value) return ''
  
  const schema = props.element.schemaId ? getComponentSchema(props.element.schemaId) : undefined
  const mergedData = mediaData({ ...props.element, data: initialElementData }, schema?.defaultData)
  const dataString = serializeForInlineScript(mergedData)
  
  // Resolve component CSS variables
  const componentVars = schema ? buildComponentVars(schema, initialElementStyle) : {}
  const allVars = { ...initialThemeVars, ...componentVars }
  
  const styleString = cssVarDeclarations(allVars)

  // We do NOT apply data substitutions to the HTML string here, because we want the JS inside
  // to dynamically read window.vortideckData instead of hardcoding {data.val} into strings.
  // Actually, we DO substitute it for the initial load just in case they used standard HTML bindings too.
  const rawHtml = sandboxSourceHtml(schema)
  const htmlContent = applyDataSubstitutions(rawHtml, mergedData)
  const isUntrusted = props.element.trust === 'community' || props.element.trust === 'verified'
  // A draft preview never receives desktop authority or makes API requests.
  // Declared production network grants remain effective in the OBS renderer.
  const cspMeta = `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'unsafe-inline' 'unsafe-eval'; style-src 'unsafe-inline'; img-src data: blob: https:; media-src https:; font-src data: https:; connect-src 'none'; form-action 'none'; base-uri 'none'">`

  return `
    <!DOCTYPE html>
    <html>
      <head>
        <meta charset="utf-8">
        ${cspMeta}
        <style>
          :root { ${styleString} }
          body { margin: 0; padding: 0; overflow: hidden; width: 100vw; height: 100vh; background: transparent; }
        </style>
        <script>
          ${widgetSettingsBridgeScript}
          window.vortideckData = ${dataString};
          window.addEventListener('message', (event) => {
            if (event.data?.type === 'UPDATE_DATA') {
              window.vortideckData = { ...window.vortideckData, ...event.data.payload };
              window.dispatchEvent(new CustomEvent('vortideck-data-update', { detail: window.vortideckData }));
            } else if (event.data?.type === 'UPDATE_VARS') {
              const vars = event.data.payload;
              for (const [key, value] of Object.entries(vars)) {
                document.documentElement.style.setProperty(key, value);
              }
            } else if (event.data?.category === 'stream' && event.data?.payload) {
              window.dispatchEvent(new CustomEvent('vortideck-stream-event', { detail: event.data.payload }));
            }
          });
          window.addEventListener('DOMContentLoaded', () => {
            document.querySelectorAll('canvas').forEach(canvas => {
              const resizeObserver = new ResizeObserver(() => {
                canvas.width = window.innerWidth;
                canvas.height = window.innerHeight;
              });
              resizeObserver.observe(canvas);
            });
          });
        ${'<' + '/script>'}
      </head>
      <body>
        ${htmlContent}
      </body>
    </html>
  `
})

const sendWidgetConfig = () => {
  sandboxIframe.value?.contentWindow?.postMessage(widgetConfigMessage(props.element.config || {}, props.element.draftConfigRevision ?? props.element.configRevision ?? 0), '*')
}
watch(() => [props.element.config, props.element.configRevision, props.element.draftConfigRevision], () => {
  if (iframeReady.value) sendWidgetConfig()
}, { deep: true })

const onIframeLoad = () => {
  sendWidgetConfig()
  iframeReady.value = true
  if (sandboxIframe.value?.contentWindow) {
    const schema = props.element.schemaId ? getComponentSchema(props.element.schemaId) : undefined
    const mergedData = { ...(schema?.defaultData || {}), ...elementDataLocal.value }
    const plainData = JSON.parse(JSON.stringify(mergedData))
    sandboxIframe.value.contentWindow.postMessage({ type: 'UPDATE_DATA', payload: plainData }, '*')
  }
}

// Send live updates to iframe without reloading
watch(elementDataLocal, (newData) => {
  if (iframeReady.value && sandboxIframe.value?.contentWindow) {
    const schema = props.element.schemaId ? getComponentSchema(props.element.schemaId) : undefined
    const mergedData = { ...(schema?.defaultData || {}), ...(newData || {}) }
    const plainData = JSON.parse(JSON.stringify(mergedData))
    sandboxIframe.value.contentWindow.postMessage({ type: 'UPDATE_DATA', payload: plainData }, '*')
  }
}, { deep: true })

watch(() => props.element.style, (newStyle) => {
  if (sandboxIframe.value?.contentWindow) {
    const schema = props.element.schemaId ? getComponentSchema(props.element.schemaId) : undefined
    const componentVars = schema ? buildComponentVars(schema, newStyle) : {}
    const plainVars = JSON.parse(JSON.stringify(componentVars))
    sandboxIframe.value.contentWindow.postMessage({ type: 'UPDATE_VARS', payload: plainVars }, '*')
  }
}, { deep: true })

watch(cssVars, (newVars) => {
  if (iframeReady.value && sandboxIframe.value?.contentWindow) {
    const plainVars = JSON.parse(JSON.stringify(newVars))
    sandboxIframe.value.contentWindow.postMessage({ type: 'UPDATE_VARS', payload: plainVars }, '*')
  }
}, { deep: true })


/**
 * Substitute {data.key} placeholders in a template string from element.data.
 * {event.key} is reserved for the future Event Bus and left as-is.
 */
function applyDataSubstitutions(template: string, data?: Record<string, any>): string {
  if (!template || !data) return template
  return template.replace(/\{data\.([^}]+)\}/g, (_match, key) => {
    const val = data[key]
    return val !== undefined ? escapeHtml(String(val)) : _match
  })
}

function escapeHtml(value: string): string {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;')
}

function serializeForInlineScript(value: unknown): string {
  return JSON.stringify(value)
    .replaceAll('<', '\\u003c')
    .replaceAll('\u2028', '\\u2028')
    .replaceAll('\u2029', '\\u2029')
}

/**
 * Pure data-driven styles - ALL from element properties
 */
const elementStyles = computed(() => {
  const element = props.element

  // Look up the component schema for this element
  const schema = element.schemaId ? getComponentSchema(element.schemaId) : undefined

  // Build --c-* variable map: schema defaults + per-element overrides
  const componentVars: Record<string, string> = schema
    ? buildComponentVars(schema, element.style)
    : {}

  // Also pick up any --c-* vars set directly on the element style (for elements without schema)
  if (!schema && element.style) {
    for (const [key, val] of Object.entries(element.style)) {
      if (typeof key === 'string' && key.startsWith('--c-') && val) {
        componentVars[key] = String(val)
      }
    }
  }

  // Visual styles (bg, border, radius, shadow, padding)
  const visualStyles = {
    ...element.style,
    ...componentVars, // Inject all --c-* custom properties so child elements can inherit/reference them
    ...(!(schema?.template || element.customHTML) && !element.style?.background && !element.style?.backgroundColor &&
      componentVars['--c-bg'] ? { background: componentVars['--c-bg'] } : {}),
    ...(element.backgroundColor && { backgroundColor: element.backgroundColor }),
    ...(element.background && { background: element.background }),
    ...(element.border && { border: element.border }),
    ...(element.borderRadius && { borderRadius: element.borderRadius }),
    ...(element.boxShadow && { boxShadow: element.boxShadow }),
    ...(element.style?.padding === undefined && componentVars['--c-padding'] && { padding: componentVars['--c-padding'] }),
  }

  return {
    // Base positioning and layout
    position: 'absolute',
    inset: '0',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    fontFamily: 'Inter, -apple-system, BlinkMacSystemFont, sans-serif',
    color: '#ffffff',
    ...visualStyles,
    ...(element.color && { color: element.color }),
    ...(element.fontSize && { fontSize: element.fontSize }),
    ...(element.fontWeight && { fontWeight: element.fontWeight }),
    ...(element.fontFamily && { fontFamily: element.fontFamily }),
    ...(element.textAlign && { textAlign: element.textAlign }),
    ...(element.margin && { margin: element.margin }),
    ...(element.opacity && { opacity: element.opacity }),
    ...(element.overflow && { overflow: element.overflow }),
    ...(element.flexDirection && { flexDirection: element.flexDirection }),
    ...(element.alignItems && { alignItems: element.alignItems }),
    ...(element.justifyContent && { justifyContent: element.justifyContent })
  }
})
</script>

<style scoped>
/* No hardcoded styles - everything comes from element data */
</style>
