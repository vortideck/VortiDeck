# Overlay widget authoring settings

Implemented contract, September 2026. This describes saved widget settings;
live stream data and package permissions are separate contracts.

## Package declaration

An overlay `widget` contribution may declare `configSchema`. Use a stable
contribution `id`, especially when a package contains several widgets.

```json
{
  "type": "widget",
  "id": "follower-goal",
  "configSchema": {
    "$schema": "http://json-schema.org/draft-07/schema#",
    "type": "object",
    "additionalProperties": false,
    "properties": {
      "goal": { "type": "integer", "minimum": 1, "default": 1000 },
      "title": { "type": "string", "default": "Followers" },
      "showLatest": { "type": "boolean", "default": true }
    },
    "required": ["goal", "title", "showLatest"]
  }
}
```

This is the settings portion of a contribution; normal content and capability
fields are still required. See the complete
[follower-goal example](../../examples/plugins/follower-goal-widget/manifest.json).

The host uses JSON Schema draft 7 with format assertions disabled. Local `$ref`
references work; remote and file retrieval are disabled. Schema and resulting
configuration each have a 64 KiB limit. The host inserts missing `default`
values from explicit object properties, including nested properties and array
items. It does not select `oneOf`/`anyOf` branches, resolve defaults through
`$ref`, coerce types, strip unknown fields, or overwrite explicit `null` values.
Authors should supply defaults for the normal first-use experience. When required
values have no defaults, the editor keeps the rejected add as a draft until the
user provides valid settings and applies them.

The inspector renders scalar fields and enums. Structured values have a JSON
editor; the complete object is validated on Apply. `title` and `description`
provide presentation text. `format: color` is not a validator or a special
permission: use a pattern to constrain a hex color when that is the contract.

## Instance and renderer contract

Each placed instance saves `contributionId`, `config` and `configRevision` in
the editor project. Native serialization uses `contribution_id`, `config` and
`config_revision`. Legacy instances deserialize to `{}` and revision zero.
The native active-output snapshot survives restarting the application. Hidden
instances remain in this snapshot and are excluded from rendering by visibility.

Widget code consumes settings as follows:

```js
let settings = window.vortideckConfig || {};
function applySettings(next) {
  settings = next;
  document.querySelector('.title').textContent = settings.title || 'Followers';
  // Re-render presentation using the existing runtime count; do not reset it.
}
window.addEventListener('vortideck-config-update', event => applySettings(event.detail));
applySettings(settings);
```

Initial settings arrive through the same hydration channel after the frame loads.
Listen for the event rather than reading the global only once during script
evaluation. Later updates replace the complete object, including removed keys.
They do not recreate `srcdoc`, reset runtime `data`, or recreate the iframe.
The shared bridge accepts version-1 `UPDATE_CONFIG` messages from its parent
only and ignores older revisions. OBS receives the current configuration from
host-owned wrapper metadata on load/reconnect and live notifications while open.

The package still implements what each property does. A schema does not
automatically bind arbitrary CSS, grant network access, or make a counter durable.
The example's count remains ephemeral; application/provider-owned counters are
follow-up work.

## Shared native and assistant operations

`get_overlay_widget_settings(elementId)` returns the installed contribution's
`configSchema`, saved `config`, and `configRevision`.
`configure_overlay_widget(elementId, config, expectedRevision)` replaces the
settings atomically against that revision. The installed manifest supplies the
schema; cached frontend metadata is not validation authority. Failed validation,
stale revisions or failed persistence do not publish a candidate to active state.

The internal assistant receives:

- `vortideck_inspect_overlay_widgets`: list active instances, or inspect a
  specific `elementId` to discover its schema and current settings.
- `vortideck_configure_overlay_widget`: replace one instance's settings with
  `elementId`, complete `config`, and the inspected `expectedRevision`.

These tools call the same host service as Tauri. This is an authoring operation,
not a flow node or a second execution engine. Applying settings changes the
active output immediately. The result proves a saved host configuration, not a
visual-render ACK from OBS. Editing a saved project library, staging a separate
preview and publishing a presentation are not yet exposed by these tools.
The tools accept the existing interactive assistant context; scoped flow,
plugin and remote execution contexts are rejected. Workflow use of AI does not
implicitly grant the flow editor-level authoring authority.

There is no new external MCP server or remote authentication path in this change.
A future inbound adapter can reuse `OverlayWidgetSettingsApi`; it must establish
caller authorization before exposing the service. Never use an OBS browser-source
access token as authoring authority.

## Compatibility

Widgets without `configSchema` keep their existing behavior. Built-in schema
widgets retain their existing `editableProps` data/style controls. Deck widgets
retain `deckSurface.configSchema` and their interactive deck renderer. Both
surfaces use schema-described settings, but their runtime/UI contracts are
distinct. The existing deck validator has not been silently replaced with a
different validation dialect in this change.

Package schema changes participate in the existing consent/content hash. A
package update therefore follows normal re-consent. Invalid saved settings must
be corrected explicitly; updates do not silently erase user values.
