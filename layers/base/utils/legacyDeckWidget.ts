import type { Button } from "@base/types/deck";
import type {
  DeckWidgetInstance,
  WidgetBinding,
  WidgetDefinition,
  WidgetExecutionPolicy,
  WidgetPlacement,
} from "@base/types/deckWidget";

export type LegacyDeckButton = Omit<Button, "id"> & { id: number | string };

const hasMeaningfulText = (value: string | null | undefined) => (
  typeof value === "string" && value.trim() !== "" && value.trim().toLowerCase() !== "null"
);

/** An untouched V1 action-button record represents an available grid slot,
 * not a rendered widget in the first-class document. */
export const isVacantLegacyButton = (button: LegacyDeckButton): boolean => (
  button.mode !== "rotary"
  && !hasMeaningfulText(button.label)
  && !hasMeaningfulText(button.icon)
  && !hasMeaningfulText(button.background)
  && !hasMeaningfulText(button.action)
  && !hasMeaningfulText(button.color)
  && (!button.execution_mode || button.execution_mode === "run_once")
  && !hasMeaningfulText(button.execution_config)
);

const triggerPolicies = [
  "run_once",
  "repeat_while_pressed",
  "toggle",
  "queue",
] as const;

export const legacyActionButtonDefinition: WidgetDefinition = {
  id: "vortideck.action-button",
  name: "Action Button",
  category: "actions",
  rendererId: "legacy-action-button",
  interactionType: "atomic_button",
  defaultSpan: { cols: 1, rows: 1 },
  minSpan: { cols: 1, rows: 1 },
  maxSpan: { cols: 4, rows: 4 },
  configSchema: { type: "object" },
  inputs: [
    {
      id: "primary",
      kind: "trigger",
      supportedPolicies: [...triggerPolicies],
    },
  ],
  mobileSupport: "host_schema",
};

export const legacyRotaryDefinition: WidgetDefinition = {
  id: "vortideck.rotary",
  name: "Rotary Control",
  category: "controls",
  rendererId: "legacy-rotary",
  interactionType: "continuous_value",
  defaultSpan: { cols: 1, rows: 1 },
  minSpan: { cols: 1, rows: 1 },
  maxSpan: { cols: 4, rows: 4 },
  configSchema: { type: "object" },
  stateSchema: { type: "object" },
  inputs: [
    {
      id: "value",
      kind: "value",
      valueType: "number",
      min: 0,
      max: 100,
      supportedDelivery: ["live", "commit", "live_and_commit"],
    },
  ],
  stateSubscriptions: ["system.volume"],
  mobileSupport: "host_schema",
};

const parseExecutionConfig = (value: string | null): Record<string, unknown> => {
  if (!value) return {};
  try {
    const parsed: unknown = JSON.parse(value);
    return parsed && typeof parsed === "object" ? parsed as Record<string, unknown> : {};
  } catch {
    return {};
  }
};

export const legacyExecutionPolicy = (button: LegacyDeckButton): WidgetExecutionPolicy => {
  const config = parseExecutionConfig(button.execution_config);
  switch (button.execution_mode ?? "run_once") {
    case "repeat_while_pressing":
      return {
        type: "repeat",
        holdMs: Number(config.hold_threshold_ms ?? 300),
        intervalMs: Number(config.repeat_interval_ms ?? 100),
      };
    case "switch":
      return { type: "toggle" };
    case "queue":
      return { type: "queue" };
    default:
      return { type: "run_once" };
  }
};

const legacyBinding = (button: LegacyDeckButton): WidgetBinding => ({
  id: `button:${button.id}:primary`,
  portId: button.mode === "rotary" ? "value" : "primary",
  action: button.action,
  policy: button.mode === "rotary"
    ? { type: "continuous", throttleMs: 50, commitOnRelease: true }
    : legacyExecutionPolicy(button),
});

export const legacyPositionToPlacement = (
  position: number,
  columns: number,
): WidgetPlacement => {
  const safeColumns = Math.max(1, columns);
  const index = Math.max(0, position - 1);
  return {
    x: index % safeColumns,
    y: Math.floor(index / safeColumns),
    cols: 1,
    rows: 1,
  };
};

export const legacyWidgetPlacementForLayout = (
  instance: DeckWidgetInstance,
  layoutKey: "portrait" | "landscape",
  columns: { portrait: number; landscape: number },
): WidgetPlacement | undefined => {
  const explicit = instance.layouts[layoutKey];
  if (explicit) return explicit;
  const position = Number(instance.config.position);
  if (instance.legacyButtonId != null && Number.isInteger(position) && position > 0) {
    return legacyPositionToPlacement(position, columns[layoutKey]);
  }
  return instance.layouts.default;
};

export const legacyButtonToWidget = (
  button: LegacyDeckButton,
  columns: number | { portrait: number; landscape: number },
): DeckWidgetInstance => {
  const portraitColumns = typeof columns === "number" ? columns : columns.portrait;
  const layouts: Record<string, WidgetPlacement> = typeof columns === "number"
    ? { default: legacyPositionToPlacement(button.position, portraitColumns) }
    : {
        default: legacyPositionToPlacement(button.position, portraitColumns),
        portrait: legacyPositionToPlacement(button.position, columns.portrait),
        landscape: legacyPositionToPlacement(button.position, columns.landscape),
      };

  return {
    id: `button:${button.id}`,
    definitionId: button.mode === "rotary"
      ? legacyRotaryDefinition.id
      : legacyActionButtonDefinition.id,
    configVersion: 1,
    config: {
      label: button.label,
      iconRef: String(button.id),
      iconUrl: button.icon ?? undefined,
      imageFit: button.icon_fit === "contain" ? "contain" : "cover",
      position: button.position,
      rotaryBehavior: "blocked",
    },
    layouts,
    bindings: [legacyBinding(button)],
    legacyButtonId: typeof button.id === "number" ? button.id : undefined,
    enabled: true,
  };
};

/** Reconcile mutable V1 button data into its first-class widget projection
 * without disturbing layouts or other instance-owned metadata. */
export const reconcileLegacyWidgetInstance = (
  instance: DeckWidgetInstance,
  button: LegacyDeckButton,
): DeckWidgetInstance => {
  if (instance.legacyButtonId !== button.id) return instance;
  const projection = legacyButtonToWidget(button, 1);
  return {
    ...instance,
    definitionId: projection.definitionId,
    config: {
      ...instance.config,
      ...projection.config,
    },
    bindings: projection.bindings,
  };
};
