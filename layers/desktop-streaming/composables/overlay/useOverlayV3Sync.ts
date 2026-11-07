import { widgetSettingsBridgeScript } from '@base/utils/widgetSettingsBridge'
import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'
import { useOverlayTheme } from './useOverlayTheme'
import { getComponentSchema, buildComponentVars } from './componentSchema'
import { mediaData } from './mediaTemplate'

// ── Verify-on-load ─────────────────────────────────────────────────────────
// Granted widget ids (module-level singleton). A placed community/verified
// widget whose id is no longer in this set renders a "revoked" placeholder
// instead of running its (possibly stale) content. Refreshed from `list_packages`.
// A widget flagged needs_reconsent (changed since consent) is excluded too — it
// renders the placeholder until the user re-confirms via the capability dialog.
const grantedWidgetIds = new Set<string>()
let widgetGrantsLoaded = false
async function refreshWidgetGrants() {
  try {
    const pkgs: any[] = await invoke('list_packages')
    const currentGrants = new Set<string>()
    for (const p of pkgs) {
      // A bundle may legitimately provide both runtime code and widget
      // contributions. Grant the widget contribution based on consent, not on
      // whether the package also has a runtime contribution.
      if (p.is_granted && !p.needs_reconsent) currentGrants.add(p.id)
    }
    grantedWidgetIds.clear()
    currentGrants.forEach(id => grantedWidgetIds.add(id))
    widgetGrantsLoaded = true
    return pkgs
  } catch (e) {
    widgetGrantsLoaded = false
    grantedWidgetIds.clear()
    console.warn('refreshWidgetGrants failed:', e)
    throw e
  }
}

// ── CSS custom-property emission ────────────────────────────────────────────
// Theme and component variables are user/package data emitted as raw text into
// <style> blocks (overlay page and iframe srcdoc). A value that can end its
// declaration, rule or the <style> element is dropped, never emitted.
const CSS_VAR_NAME = /^--[A-Za-z0-9_-]+$/
const UNSAFE_CSS_VALUE = /[<>{};\\]|\/\*/

export const isSafeCssVar = (name: string, value: unknown): boolean =>
  CSS_VAR_NAME.test(name) && !UNSAFE_CSS_VALUE.test(String(value))

/** Serialize safe `--name: value` declarations; unsafe pairs are skipped. */
export const cssVarDeclarations = (vars: Record<string, unknown>, separator = '; ', colon = ': '): string =>
  Object.entries(vars)
    .filter(([name, value]) => isSafeCssVar(name, value))
    .map(([name, value]) => `${name}${colon}${String(value).replace(/"/g, "'")}`)
    .join(separator)

export const isSafeElementId = (id: unknown): id is string =>
  typeof id === 'string' && /^[A-Za-z0-9_-]{1,128}$/.test(id)

// ── Host-webview rendering decision ─────────────────────────────────────────
// The editor webview has Tauri IPC. Only first-party schema templates on
// builtin elements may reach host v-html; any element-supplied markup
// (customHTML, innerHTML, HTML content, legacy modular templates) and every
// non-builtin element with markup renders in the sandboxed iframe instead.
/** The element fields that decide where its markup may render. */
export type MarkupElement = {
  type?: string
  trust?: string
  template?: unknown
  bindings?: unknown
  customHTML?: unknown
  innerHTML?: unknown
  content?: unknown
  [key: string]: unknown
}

export const elementHasSuppliedMarkup = (element: MarkupElement): boolean =>
  Boolean(
    (element.type === 'modular' && element.template && element.bindings)
    || element.customHTML
    || element.innerHTML
    || (typeof element.content === 'string' && /[<&]/.test(element.content)),
  )

export const elementNeedsSandbox = (element: MarkupElement, schema?: { template?: string }): boolean => {
  // Community/verified widgets are always isolated, even without markup.
  if (element.trust === 'community' || element.trust === 'verified') return true
  const builtin = (element.trust || 'builtin') === 'builtin'
  // Modular elements render their own template even when a schema exists.
  if (element.type === 'modular' && element.template && element.bindings) return true
  if (schema?.template) return !builtin || schema.template.includes('<script')
  return elementHasSuppliedMarkup(element)
}

const IMAGE_OBJECT_FITS = ['fill', 'contain', 'cover', 'none', 'scale-down'] as const
type ImageObjectFit = (typeof IMAGE_OBJECT_FITS)[number]
export const safeObjectFit = (value: unknown): ImageObjectFit =>
  IMAGE_OBJECT_FITS.find((fit) => fit === value) ?? 'cover'

/**
 * Schema-Driven Overlay Sync — Phase 1
 *
 * Reads each element's component schema to know which --c-* vars exist,
 * resolves them through the theme, and generates concrete HTML for OBS.
 *
 * Resolution chain (OBS path):
 *   var(--c-bg)  →  schema default (e.g. "linear-gradient(135deg, var(--theme-accent), var(--theme-primary))")
 *                →  resolveVar  →  concrete value (e.g. "linear-gradient(135deg, #ff1493, #ff00ff)")
 */
export const useOverlayV3Sync = (options?: { live: () => boolean }) => {
  const { resolveVar, cssVars } = useOverlayTheme()
  const lastSyncError = ref('')

  /**
   * Universal Element Renderer — works for ANY element using its schema + data.
   * All var(--c-*) and var(--theme-*) references are resolved to concrete values for OBS.
   */
  const generateElementHTML = (element: any, rehearsal = false): string => {
    if (element.pendingImportReview) return '<div style="padding:12px;font-family:sans-serif">Imported custom content — review before activation</div>'
    // Handle modular elements with templates and data binding
    if (element.type === 'modular' && element.template && element.bindings) {
      const maxHealth = Math.max(1, Number(element.data?.maxHealth ?? 120))
      const maxMana = Math.max(1, Number(element.data?.maxMana ?? 80))
      const health = Number(element.data?.health ?? 90)
      const mana = Number(element.data?.mana ?? 48)
      const legacyBindingData: Record<string, Record<string, unknown>> = {
        'game-stats': {
          health,
          maxHealth,
          healthPercent: Math.max(0, Math.min(100, (health / maxHealth) * 100)),
          mana,
          maxMana,
          manaPercent: Math.max(0, Math.min(100, (mana / maxMana) * 100)),
          level: Number(element.data?.level ?? 23),
        },
      }

      let processedTemplate = element.template
      const bindingRefs = processedTemplate.match(/\{([^}]+)\}/g) || []

      for (const ref of bindingRefs) {
        const path = ref.slice(1, -1)
        const [binding, ...propertyPath] = path.split('.')

        if (element.bindings[binding] && legacyBindingData[element.bindings[binding]]) {
          let value: unknown = legacyBindingData[element.bindings[binding]]
          for (const prop of propertyPath) {
            value = typeof value === 'object' && value !== null
              ? (value as Record<string, unknown>)[prop]
              : undefined
          }
          if (value !== undefined) {
            const safeValue = String(value).replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;').replaceAll("'", '&#39;')
            processedTemplate = processedTemplate.replace(ref, safeValue)
          }
        }
      }

      // Legacy templates use the same isolation/settings bridge as every other
      // widget; raw markup here previously escaped the iframe boundary.
      return generateElementHTML({ ...element, type: 'custom', template: undefined, customHTML: resolveVar(processedTemplate), schemaId: undefined }, rehearsal)
    }

    // ── Schema-based resolution ─────────────────────────────────

    /**
     * Substitute {data.key} placeholders from element.data (design-time content).
     * {event.key} is reserved for the future Event Bus and is left as-is.
     */
    const applyDataSubstitutions = (html: string, data?: Record<string, any>): string => {
      if (!html || !data) return html
      return html.replace(/\{data\.([^}]+)\}/g, (_match, key) => {
        const val = data[key]
        return val !== undefined ? escapeHtml(String(val)) : _match
      })
    }

    const escapeHtml = (value: string): string => value
      .replaceAll('&', '&amp;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;')
      .replaceAll('"', '&quot;')
      .replaceAll("'", '&#39;')

    const serializeForInlineScript = (value: unknown): string => JSON.stringify(value)
      .replaceAll('<', '\\u003c')
      .replaceAll('\u2028', '\\u2028')
      .replaceAll('\u2029', '\\u2029')

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
    // Unsafe custom properties are dropped before any emit site sees them.
    for (const [key, val] of Object.entries(componentVars)) {
      if (!isSafeCssVar(key, val)) Reflect.deleteProperty(componentVars, key)
    }

    // NOTE: vars are intentionally left as live references (var(--c-*) →
    // var(--theme-*)) in the generated HTML so theme changes propagate through
    // the CSS cascade via UPDATE_VARS, with no iframe reload. --theme-* values
    // are baked into the iframe srcdoc (from cssVars) and the overlay :root
    // (backend), and updated live by UPDATE_VARS.

    // Build wrapper styles — explicit px dimensions so OBS renders correctly
    const isScaled = !!(element.scaleContent && element.designWidth && element.designHeight)

    // Visual styles (bg, border, radius) — when scale mode is ON these go on the scale
    // wrapper (design-size card), not the outer wrapper. Otherwise the dark background
    // fills the entire element.width×height box instead of just the card area.
    const visualStyles: Record<string, any> = {
      ...element.style,
      ...(!( schema?.template || element.customHTML) && !element.style?.background && !element.style?.backgroundColor &&
        componentVars['--c-bg'] ? { background: componentVars['--c-bg'] } : {}),
      ...(element.backgroundColor && { backgroundColor: element.backgroundColor }),
      ...(element.background && { background: element.background }),
      ...(element.border && { border: element.border }),
      ...(element.borderRadius && { borderRadius: element.borderRadius }),
      ...(element.boxShadow && { boxShadow: element.boxShadow }),
      ...(element.style?.padding === undefined && componentVars['--c-padding'] && { padding: componentVars['--c-padding'] }),
    }

    // Structural styles always on outer wrapper
    const outerStyles: Record<string, any> = {
      position: 'relative',
      width: (element.width || 200) + 'px',
      height: (element.height || 100) + 'px',
      fontFamily: 'Inter, -apple-system, BlinkMacSystemFont, sans-serif',
      color: element.color || '#ffffff',
      ...(element.opacity && { opacity: element.opacity }),
      // NOTE: z-index is intentionally NOT here. It is rendered on the real
      // outer `.overlay-element` div by the Rust backend (from element.z_index)
      // so it controls sibling stacking and survives the overlay's fetch+diff.
      // In non-scale mode: visual styles go on outer wrapper too
      ...(isScaled ? {} : visualStyles),
    }

    const styleString = Object.entries(outerStyles)
      .filter(([key, value]) => value !== undefined && value !== null && value !== '' && !key.startsWith('--c-'))
      .map(([key, value]) => `${key.replace(/([A-Z])/g, '-$1').toLowerCase()}: ${String(value).replace(/"/g, "'")}`)
      .join('; ')

    // Define --c-* on the element (in the overlay DOM) so the outer wrapper's
    // frame styling — background/border/etc. that uses var(--c-*) — resolves.
    // These are live references to --theme-* (on the overlay :root), so theme
    // changes recolor the frame via the cascade too. The iframe content resolves
    // its own --c-* from the srcdoc :root; this block is for the frame around it.
    const cssVarBlock = Object.keys(componentVars).length > 0 && isSafeElementId(element.id)
      ? `<style>#${element.id}{${cssVarDeclarations(componentVars, ';', ':')}}</style>`
      : ''

    // Content: use schema template, customHTML, or fallbacks. Vars are kept as
    // live references — they resolve inside the widget's iframe :root.
    const template = schema?.template || element.customHTML
    // Merge schema defaultData (fallback) with element.data (user overrides) so
    // existing elements created before defaultData seeding still render placeholders.
    const resolvedData = mediaData(element, schema?.defaultData)
    const sourceHtml = template || element.innerHTML || ''

    let innerHtml = ''
    if (sourceHtml) {
      innerHtml = applyDataSubstitutions(sourceHtml, resolvedData)
    } else if (element.content) {
      innerHtml = element.content
    } else if (element.src || element.imageUrl) {
      innerHtml = `<img src="${escapeHtml(String(element.src || element.imageUrl))}" alt="${escapeHtml(String(element.name || ''))}" style="width: 100%; height: 100%; object-fit: ${safeObjectFit(element.objectFit)};" />`
    } else if (element.name) {
      innerHtml = element.name
    } else {
      innerHtml = element.type || 'Element'
    }

    // ALWAYS isolate the widget in a sandboxed iframe. Uniform rendering path;
    // every widget gets its own document + :root (theme cascade identical for
    // all widgets), and there's no style/JS leakage between widgets. --theme-*
    // (from cssVars) + --c-* (references to --theme-*) are baked into the iframe
    // :root for first paint; UPDATE_VARS updates them live with no reload.
    const componentVarString = cssVarDeclarations(componentVars)
    const themeVarString = cssVarDeclarations(cssVars.value)
    const iframeStyleString = `${themeVarString}; ${componentVarString}`

    // ── Phase 2: per-widget sandbox + CSP (trust-aware) ───────────────────
    // Builtin widgets are trusted → full sandbox (allow-same-origin) + no CSP.
    // Community/verified widgets are untrusted → strict sandbox (opaque origin,
    // no parent/storage access) + a CSP that locks network to the granted
    // capability. The network capability drives connect-src (non-lossy):
    //   none/undefined → 'none' | whitelist → those hosts | full → * (high-risk)
    // Native primitives have no package identity. Executable first-party and
    // third-party widgets share this renderer; trust only changes isolation.
    const isUntrusted = element.trust === 'community' || element.trust === 'verified' || element.trust === 'local'
    const sandbox = isUntrusted || rehearsal ? 'allow-scripts' : 'allow-scripts allow-same-origin'
    // Verify-on-load: a community/verified widget whose grant was revoked renders
    // a placeholder instead of running its content (grant no longer valid).
    if (isUntrusted && element.trust !== 'local' && (!element.widgetId || !widgetGrantsLoaded || !grantedWidgetIds.has(element.widgetId))) {
      innerHtml = '<div style="width:100%;height:100%;display:flex;align-items:center;justify-content:center;color:#9ca3af;font-family:sans-serif;font-size:12px;text-align:center;padding:8px;background:rgba(0,0,0,.35);border:1px dashed #6b7280;border-radius:8px;box-sizing:border-box;">Widget access revoked — re-install it from My Plugins</div>'
    }
    let cspMeta = ''
    if (isUntrusted || rehearsal) {
      const net = element.capabilities?.network
      let connectSrc = "'none'"
      if (!rehearsal && net === 'full') {
        connectSrc = '*'
      } else if (!rehearsal && net && typeof net === 'object' && 'whitelist' in net && Array.isArray(net.whitelist) && net.whitelist.length) {
        connectSrc = net.whitelist.join(' ')
      }
      const csp = `default-src 'none'; script-src 'unsafe-inline' 'unsafe-eval'; style-src 'unsafe-inline'; img-src data: blob: https:; media-src https:; font-src data: https:; connect-src ${connectSrc}; form-action 'none'; base-uri 'none'`
      cspMeta = `<meta http-equiv="Content-Security-Policy" content="${csp}">`
    }

    const srcdoc = `
      <!DOCTYPE html>
      <html>
        <head>
          ${cspMeta}
          <style>
            :root { ${iframeStyleString} }
            body { margin: 0; padding: 0; overflow: hidden; width: 100vw; height: 100vh; background: transparent; color: var(--c-text, #ffffff); font-family: var(--c-font, inherit); }
          </style>
          <script>
             ${widgetSettingsBridgeScript}
          window.vortideckData = ${serializeForInlineScript(resolvedData)};
          window.vortideckRehearsal = ${rehearsal};
            const vortideckActionRequests = new Map();
            window.vortideck = Object.freeze({
              requestAction(action, payload = null) {
                if (window.vortideckRehearsal) return Promise.reject(new Error('Native actions are disabled in rehearsal'));
                const requestId = self.crypto && self.crypto.randomUUID
                  ? self.crypto.randomUUID()
                  : 'widget-' + Date.now() + '-' + Math.random().toString(16).slice(2);
                return new Promise((resolve, reject) => {
                  const timeout = setTimeout(() => {
                    vortideckActionRequests.delete(requestId);
                    reject(new Error('Widget action request timed out'));
                  }, 15000);
                  vortideckActionRequests.set(requestId, { resolve, reject, timeout });
                  window.parent.postMessage({
                    type: 'VORTIDECK_WIDGET_ACTION_REQUEST',
                    requestId,
                    action,
                    payload
                  }, '*');
                });
              },
              acknowledgeRendered(command) {
                const commandId = command?.commandId;
                const appliedRevision = command?.deliveryRevision || 1;
                if (typeof commandId !== 'string' || !commandId) {
                  throw new Error('A valid overlay command is required for visual ACK');
                }
                window.parent.postMessage({
                  type: 'VORTIDECK_WIDGET_RENDERED',
                  commandId,
                  appliedRevision
                }, '*');
              }
            });
            window.addEventListener('message', (event) => {
              if (event.data?.type === 'UPDATE_DATA') {
                window.vortideckData = { ...window.vortideckData, ...event.data.payload };
                window.dispatchEvent(new CustomEvent('vortideck-data-update', { detail: window.vortideckData }));
              } else if (event.data?.type === 'UPDATE_VARS') {
                const vars = event.data.payload;
                for (const [key, value] of Object.entries(vars)) {
                  document.documentElement.style.setProperty(key, value);
                }
              } else if (event.data?.category === 'streamValues' && Array.isArray(event.data?.payload)) {
                window.dispatchEvent(new CustomEvent('vortideck-stream-values', { detail: event.data.payload }));
              } else if (event.data?.category === 'stream' && event.data?.payload) {
                // Forward raw stream events to the widget's own handlers.
                window.dispatchEvent(new CustomEvent('vortideck-stream-event', { detail: event.data.payload }));
              } else if (event.data?.category === 'overlay' && event.data?.payload) {
                window.dispatchEvent(new CustomEvent('vortideck-overlay-command', { detail: event.data.payload }));
              } else if (event.data?.type === 'VORTIDECK_WIDGET_ACTION_RESULT') {
                const pending = vortideckActionRequests.get(event.data.requestId);
                if (!pending) return;
                clearTimeout(pending.timeout);
                vortideckActionRequests.delete(event.data.requestId);
                if (event.data.ok) pending.resolve(event.data.result);
                else pending.reject(new Error(event.data.error || 'Widget action was rejected'));
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
          ${innerHtml}
        </body>
      </html>
    `
      .replace(/&/g, '&amp;')
      .replace(/"/g, '&quot;'); // Escape for the iframe srcdoc attribute

    // cssVarBlock defines --c-* on #id in the overlay DOM (frame styling);
    // the iframe (transparent) sits inside showing the widget content.
    // data-subscribe: the widget's granted event subscriptions (Phase 3 — the
    // overlay page filters stream events by this attribute). Only community
    // widgets have capabilities; builtin widgets omit it (receive all events).
    const escapeAttribute = (value: unknown) => String(value)
      .replace(/&/g, '&amp;')
      .replace(/"/g, '&quot;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
    const grantedSubscriptions = Array.isArray(element.capabilities?.subscribe)
      ? element.capabilities.subscribe
      : []
    const grantedActions = Array.isArray(element.capabilities?.actions)
      ? element.capabilities.actions
      : []
    // Every package widget—including embedded first-party contributions—carries
    // an explicit subscription list. Native primitives omit the attribute.
    const isPackageWidget = Boolean(element.widgetId)
    const subscribeAttr = isPackageWidget
      ? ` data-subscribe="${escapeAttribute(grantedSubscriptions.join(","))}"`
      : ''
    const widgetIdentityAttrs = isPackageWidget
      ? ` data-widget="${escapeAttribute(element.trust || 'builtin')}" data-widget-package-id="${escapeAttribute(element.widgetId || '')}" data-widget-contribution-id="${escapeAttribute(element.contributionId || element.schemaId || '')}" data-widget-instance-id="${escapeAttribute(element.id || '')}" data-actions="${escapeAttribute(grantedActions.join(','))}"`
      : ''
    const rawContent = `${cssVarBlock}<iframe${subscribeAttr}${widgetIdentityAttrs} style="width:100%;height:100%;border:none;pointer-events:none;background:transparent;overflow:hidden;" sandbox="${sandbox}" srcdoc="${srcdoc}"></iframe>`;

    // Apply scale content transform — mirrors OverlayCanvas.vue logic exactly.
    let content = rawContent
    if (isScaled) {
      const dw = element.designWidth
      const dh = element.designHeight
      const uniformScale = Math.min(element.width / dw, element.height / dh)

      // Build visual style string for the scale wrapper (card-level styles)
      const scaleWrapperStyle = Object.entries(visualStyles)
        .filter(([key, value]) => value !== undefined && value !== null && value !== '' && !key.startsWith('--c-'))
        .map(([key, value]) => `${key.replace(/([A-Z])/g, '-$1').toLowerCase()}: ${String(value).replace(/"/g, "'")}`)
        .join('; ')

      content = `<div style="position:absolute;top:0;left:0;width:${dw}px;height:${dh}px;transform:scale(${uniformScale.toFixed(4)});transform-origin:top left;${scaleWrapperStyle}">${rawContent}</div>`
    }

    return `<div style="${styleString}">${content}</div>`

  }

  /**
   * Sync all elements to backend in a single batch.
   * Backend clears + re-adds all, regenerates HTML once, broadcasts once.
   */
  const syncElementsToBackend = async (
    elements: any[],
    canvas?: { width: number; height: number },
    publication?: { compositionId: string; expectedRevision: number },
  ) => {
    if (options && !options.live() && !publication) return true
    // Hidden elements still own saved settings and must survive full syncs.
    const syncedElements = elements

    try {
      // Consent may change after this editor opened. Never persist HTML built
      // from its initial grant cache after a grant, re-grant, or revocation.
      await refreshWidgetGrants()
      // An agent may have saved while this editor was closed. Recover newer
      // host settings before submitting the editor's cached project snapshot.
      await Promise.all(syncedElements.map(async element => {
        try {
          const saved = await invoke<any>('get_overlay_widget_settings', { elementId: element.id })
          if (saved.packageId === (element.widgetId || null)
            && saved.contributionId === (element.contributionId || null)
            && !publication && saved.configRevision > (element.configRevision || 0)) {
            element.config = saved.config
            element.configRevision = saved.configRevision
          }
          if (publication) element.configRevision = saved.configRevision
          element.configSchema = saved.configSchema || element.configSchema
        } catch (error) {
          if (!String(error).includes('Overlay element not found')) throw error
        }
      }))
      const batch = syncedElements.map(element => ({
        id: element.id,
        visible: element.visible !== false,
        x: element.x || 0,
        y: element.y || 0,
        width: element.width || 200,
        height: element.height || 100,
        zIndex: element.zIndex ?? null,
        schemaVersion: 1,
        revision: Number(element.revision || 0),
        kind: element.type || 'custom',
        packageId: element.widgetId || null,
        contributionId: element.contributionId || null,
        config: element.config || {},
        configRevision: element.configRevision || 0,
        schemaId: element.schemaId || null,
        trust: element.trust || 'builtin',
        capabilities: element.capabilities || null,
        data: element.data || null,
        html: generateElementHTML(element),
      }))

      await invoke('batch_sync_overlay', { elements: batch, canvas, ...(publication || {}) })
      // Defaults and revisions are assigned by the host, never guessed locally.
      await Promise.all(syncedElements.map(async element => {
        const saved = await invoke<any>('get_overlay_widget_settings', { elementId: element.id })
        element.config = saved.config
        element.configRevision = saved.configRevision
        element.configSchema = saved.configSchema || undefined
      }))
      lastSyncError.value = ''
      return true
    } catch (error) {
      lastSyncError.value = String(error)
      console.error('Batch sync failed:', error)
      return false
    }
  }

  /** Export overlay to OBS */
  const exportToOBS = async (elements: any[], canvas?: { width: number; height: number }, publication?: { compositionId: string; expectedRevision: number }) => {
    try {
      const synced = await syncElementsToBackend(elements, canvas, publication)
      if (!synced) throw new Error('Failed to sync elements')
      await invoke('send_overlay_to_obs')
      return true
    } catch (error) {
      console.error('Export to OBS failed:', error)
      return false
    }
  }

  /** Initial full sync — push all elements to backend on load */
  const initialSync = async (elements: any[], canvas?: { width: number; height: number }) => {
    return await syncElementsToBackend(elements, canvas)
  }

  /** Live drag — lightweight inline style patch, no HTML regen */
  const syncElementPosition = async (elementId: string, x: number, y: number) => {
    if (options && !options.live()) return
    try {
      await invoke('broadcast_element_patch', { elementId, styles: { left: `${x}px`, top: `${y}px` } })
    } catch (_e) {
      // best-effort during drag
    }
  }

  /** Update element in backend — regenerates HTML + broadcasts via WebSocket */
  const syncElementUpdate = async (element: any) => {
    if (options && !options.live()) return
    try {
      await invoke('update_overlay_element', {
        elementId: element.id,
        updates: {
          x: element.x,
          y: element.y,
          width: element.width,
          height: element.height,
          visible: element.visible !== false,
          zIndex: element.zIndex ?? null,
          revision: Number(element.revision || 0),
          element_type: element.type || 'custom',
          package_id: element.widgetId || null,
          schema_id: element.schemaId || null,
          trust: element.trust || 'builtin',
          capabilities: element.capabilities || null,
          data: element.data || null,
          render_html: generateElementHTML(element),
        },
      })
    } catch (e) {
      console.warn('syncElementUpdate failed:', e)
      lastSyncError.value = String(e)
    }
  }

  /** Add element to backend */
  const syncElementAdd = async (element: any) => {
    if (options && !options.live()) return true
    try {
      await refreshWidgetGrants()
      await invoke('add_overlay_element_v3', {
        id: element.id,
        x: element.x || 0,
        y: element.y || 0,
        width: element.width || 200,
        height: element.height || 100,
        html: generateElementHTML(element),
        metadata: {
          packageId: element.widgetId || null,
          contributionId: element.contributionId || null,
          schemaId: element.schemaId || null,
          trust: element.trust || 'builtin',
          capabilities: element.capabilities || null,
          data: element.data || null,
          config: element.config || {},
        },
      })
      const saved = await invoke<any>('get_overlay_widget_settings', { elementId: element.id })
      element.config = saved.config
      element.configRevision = saved.configRevision
      element.configSchema = saved.configSchema || undefined
      lastSyncError.value = ''
      return true
    } catch (e) {
      console.warn('syncElementAdd failed:', e)
      lastSyncError.value = String(e)
      throw e
    }
  }

  /** Remove element from backend */
  const syncElementRemove = async (elementId: string) => {
    if (options && !options.live()) return
    try {
      await invoke('remove_overlay_element', { elementId })
    } catch (e) {
      console.warn('syncElementRemove failed:', e)
    }
  }

  return {
    lastSyncError,
    generateElementHTML,
    refreshWidgetGrants,
    syncElementsToBackend,
    exportToOBS,
    initialSync,
    syncElementPosition,
    syncElementUpdate,
    syncElementAdd,
    syncElementRemove,
  }
}
