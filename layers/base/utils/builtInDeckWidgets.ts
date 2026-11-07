import type { WidgetDefinition } from "@base/types/deckWidget";

export const faderWidgetDefinition: WidgetDefinition = {
  id: "vortideck.fader",
  name: "Fader",
  category: "controls",
  rendererId: "builtin-fader",
  interactionType: "continuous_value",
  defaultSpan: { cols: 1, rows: 1 },
  minSpan: { cols: 1, rows: 1 },
  maxSpan: { cols: 4, rows: 4 },
  configSchema: {
    type: "object",
    properties: {
      label: { type: "string" },
      min: { type: "number" },
      max: { type: "number" },
    },
  },
  inputs: [{
    id: "value",
    kind: "value",
    valueType: "number",
    min: 0,
    max: 100,
    supportedDelivery: ["live", "commit", "live_and_commit"],
  }],
  stateSubscriptions: ["system.volume"],
  mobileSupport: "host_schema",
};

export const toggleWidgetDefinition: WidgetDefinition = {
  id: "vortideck.toggle",
  name: "Toggle",
  category: "controls",
  rendererId: "builtin-toggle",
  interactionType: "atomic_button",
  defaultSpan: { cols: 1, rows: 1 },
  minSpan: { cols: 1, rows: 1 },
  maxSpan: { cols: 4, rows: 4 },
  configSchema: { type: "object", properties: { label: { type: "string" } } },
  inputs: [{ id: "toggle", kind: "trigger", supportedPolicies: ["toggle", "run_once"] }],
  mobileSupport: "host_schema",
};

export const mediaControlsWidgetDefinition: WidgetDefinition = {
  id: "vortideck.media-controls",
  name: "Media Controls",
  category: "media",
  rendererId: "builtin-media-controls",
  interactionType: "compound_zones",
  defaultSpan: { cols: 1, rows: 1 },
  minSpan: { cols: 1, rows: 1 },
  maxSpan: { cols: 4, rows: 4 },
  configSchema: { type: "object" },
  inputs: ["previous", "playPause", "next"].map((id) => ({
    id,
    kind: "trigger" as const,
    supportedPolicies: ["run_once" as const, "queue" as const],
  })),
  stateSubscriptions: ["media.playback"],
  mobileSupport: "host_schema",
};

export const telemetryWidgetDefinition: WidgetDefinition = {
  id: "vortideck.telemetry",
  name: "Telemetry",
  category: "status",
  rendererId: "builtin-telemetry",
  interactionType: "telemetry_display",
  defaultSpan: { cols: 1, rows: 1 },
  minSpan: { cols: 1, rows: 1 },
  maxSpan: { cols: 4, rows: 4 },
  configSchema: {
    type: "object",
    properties: { label: { type: "string" }, valueKey: { type: "string" }, unit: { type: "string" } },
  },
  inputs: [],
  stateSubscriptions: ["telemetry"],
  mobileSupport: "host_schema",
};

export const builtInDeckWidgetDefinitions = [
  faderWidgetDefinition,
  toggleWidgetDefinition,
  mediaControlsWidgetDefinition,
  telemetryWidgetDefinition,
];
