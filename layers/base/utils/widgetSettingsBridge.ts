/** Same versioned, parent-only configuration bridge in editor and OBS iframes.
 * Configuration is replaced atomically; runtime data belongs to a different channel.
 * Keep settings out of srcdoc so editing them never restarts a widget.
 */
export const widgetSettingsBridgeScript = `
window.vortideckConfig = {};
window.vortideckConfigRevision = -1;
window.addEventListener('message', function(event) {
  var message = event.data;
  if (event.source !== window.parent || !message || message.type !== 'UPDATE_CONFIG' || message.version !== 1) return;
  if (!message.payload || typeof message.payload !== 'object' || Array.isArray(message.payload)) return;
  if (!Number.isSafeInteger(message.revision) || message.revision < window.vortideckConfigRevision) return;
  if (message.revision === window.vortideckConfigRevision && JSON.stringify(message.payload) === JSON.stringify(window.vortideckConfig)) return;
  window.vortideckConfig = message.payload;
  window.vortideckConfigRevision = message.revision;
  window.dispatchEvent(new CustomEvent('vortideck-config-update', { detail: window.vortideckConfig }));
});
`

export function widgetConfigMessage(config: Record<string, unknown> = {}, revision = 0) {
  return { type: 'UPDATE_CONFIG', version: 1, payload: JSON.parse(JSON.stringify(config)), revision }
}
