/**
 * useWidgetEventForwarder — Phase 4: delivers live stream events to SANDBOXED
 * package widgets, filtered by each widget's granted `data-subscribe`.
 *
 * The widget iframe's host-injected bridge (see `useOverlayV3Sync` srcdoc) listens
 * for `postMessage({ category: 'stream', payload })` and dispatches it inside the
 * iframe as a `vortideck-stream-event` CustomEvent. This composable is the SENDER:
 * on each `streaming:event` CloudEvent it queries the DOM for widget iframes
 * (`iframe[data-subscribe]`), checks whether any of that widget's CONSENTED
 * subscriptions maps to the event's type (via `widgetSubscribeToCloudEvents`),
 * and postMessages ONLY to matching iframes. That filter is what makes the
 * consented `subscribe` capability actually mean something at delivery time.
 *
 * Every executable package widget goes through this path. Untrusted widgets are
 * additionally sandboxed without `allow-same-origin` and cannot reach Tauri IPC.
 *
 * Origin: the backend's tokio EventBus → `streaming:event` Tauri bridge
 * (see `useStreamingRust`). For the OBS browser-source page, the same logic
 * should hook its WebSocket handler instead — mirror this there (TODO).
 */
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { widgetSubscribeToCloudEvents } from './packageManifest'
import { normalizeWidgetStreamEvent, type WidgetStreamIngressEvent } from './widgetStreamProtocol'

type CloudEvent = WidgetStreamIngressEvent

const WIDGET_IFRAME_SELECTOR = 'iframe[data-subscribe]'

/**
 * Forward one stream event to every sandboxed widget iframe whose CONSENTED
 * `data-subscribe` list maps to that event's type. Least-privilege delivery.
 *
 * Exported so any context that receives stream events can drive it: the desktop
 * editor (via the `streaming:event` Tauri listener below) and the OBS browser-
 * source page (via its WebSocket handler — call this from there).
 */
export function forwardToWidgets(ev: CloudEvent) {
  if (!ev?.type) return
  const frames = document.querySelectorAll<HTMLIFrameElement>(WIDGET_IFRAME_SELECTOR)
  frames.forEach((frame) => {
    const raw = frame.getAttribute('data-subscribe') || ''
    const subscribed = raw.split(',').map((s) => s.trim()).filter(Boolean)
    // Least privilege: forward only if one of this widget's consented
    // subscriptions maps to this CloudEvent type.
    const matches = subscribed.some((cap) =>
      (widgetSubscribeToCloudEvents[cap] || []).includes(ev.type),
    )
    if (!matches) return
    try {
      // targetOrigin '*': sandboxed iframes have an opaque origin we can't pin.
      // Security is enforced by the filter (only matching events) + the sandbox
      // (iframe can't reach the parent). Outbound only.
      frame.contentWindow?.postMessage({ category: 'stream', payload: normalizeWidgetStreamEvent(ev) }, '*')
    } catch {
      /* iframe may have been removed; ignore */
    }
  })
}

export function useWidgetEventForwarder() {
  let unlisten: UnlistenFn | null = null

  /** Start forwarding streaming events to subscribed package-widget iframes. */
  async function start() {
    if (unlisten) return
    unlisten = await listen<CloudEvent>('streaming:event', (e) => {
      forwardToWidgets(e.payload)
    })
  }

  /** Stop forwarding. */
  function stop() {
    unlisten?.()
    unlisten = null
  }

  return { start, stop }
}
