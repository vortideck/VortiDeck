export type WidgetInteractionType =
  | "atomic_button"
  | "compound_zones"
  | "continuous_value"
  | "telemetry_display";

export type WidgetInputKind = "trigger" | "value" | "command";
export type WidgetIntentPhase =
  | "start"
  | "preview"
  | "commit"
  | "end"
  | "cancel";
export type WidgetInputSource =
  | "mouse"
  | "touch"
  | "pen"
  | "keyboard"
  | "remote";
export type WidgetHostMode = "edit" | "interactive" | "readonly";

export interface WidgetGridSpan {
  cols: number;
  rows: number;
}

export type PortableWidgetPrimitive =
  | "stack"
  | "row"
  | "column"
  | "grid"
  | "panel"
  | "text"
  | "icon"
  | "image"
  | "badge"
  | "progress"
  | "meter"
  | "press-zone"
  | "toggle"
  | "slider"
  | "rotary"
  | "spacer"
  | "divider";

export type PortableWidgetValue =
  | { literal: string | number | boolean | null }
  | { config: string; fallback?: string | number | boolean | null }
  | { state: string; fallback?: string | number | boolean | null };

export interface PortableWidgetGestureBinding {
  gesture: "press" | "hold" | "release" | "rotate" | "drag";
  portId: string;
  delivery?: "start_end" | "preview_commit" | "commit";
}

export interface PortableWidgetGestureEvent {
  binding: PortableWidgetGestureBinding;
  phase: WidgetIntentPhase;
  value?: unknown;
  inputSource: WidgetInputSource;
}

export interface PortableWidgetNode {
  type: PortableWidgetPrimitive;
  id?: string;
  children?: PortableWidgetNode[];
  value?: PortableWidgetValue;
  label?: PortableWidgetValue;
  source?: PortableWidgetValue | { asset: string };
  icon?: string;
  direction?: "horizontal" | "vertical";
  align?: "start" | "center" | "end" | "stretch";
  justify?: "start" | "center" | "end" | "between";
  gap?: number;
  columns?: number;
  min?: number;
  max?: number;
  step?: number;
  colorToken?: string;
  backgroundToken?: string;
  radius?: number;
  padding?: number;
  fontSize?: number;
  fontWeight?: 400 | 500 | 600 | 700 | 800;
  maxLines?: number;
  gestures?: PortableWidgetGestureBinding[];
}

export interface PortableWidgetAssetRef {
  id: string;
  sha256: string;
  mediaType: "image/png" | "image/jpeg" | "image/webp";
  byteLength: number;
  purpose: "icon" | "artwork" | "background";
}

export interface PortableWidgetView {
  schemaVersion: 1;
  root: PortableWidgetNode;
  assets?: PortableWidgetAssetRef[];
}

export interface WidgetPlacement extends WidgetGridSpan {
  x: number;
  y: number;
}

export type TriggerExecutionPolicyType =
  | "run_once"
  | "repeat_while_pressed"
  | "toggle"
  | "queue";

export type WidgetInputPort =
  | {
      id: string;
      kind: "trigger";
      supportedPolicies: TriggerExecutionPolicyType[];
    }
  | {
      id: string;
      kind: "value";
      valueType: "number";
      min: number;
      max: number;
      supportedDelivery: Array<"live" | "commit" | "live_and_commit">;
    }
  | {
      id: string;
      kind: "command";
      payloadSchema?: Record<string, unknown>;
    };

export interface WidgetDefinition {
  id: string;
  name: string;
  category: string;
  rendererId: string;
  interactionType: WidgetInteractionType;
  defaultSpan: WidgetGridSpan;
  minSpan: WidgetGridSpan;
  maxSpan: WidgetGridSpan;
  configSchema: Record<string, unknown>;
  stateSchema?: Record<string, unknown>;
  inputs: WidgetInputPort[];
  stateSubscriptions?: string[];
  mobileSupport: "host_schema" | "synced_web_bundle" | "fallback_only";
  portableView?: PortableWidgetView;
  contentHash?: string;
}

export type WidgetExecutionPolicy =
  | { type: "run_once" }
  | { type: "repeat"; holdMs: number; intervalMs: number; maxHoldMs?: number }
  | {
      type: "toggle";
      onAction?: unknown;
      offAction?: unknown;
      persistState?: boolean;
    }
  | { type: "queue"; maxDepth?: number }
  | {
      type: "continuous";
      throttleMs: number;
      commitOnRelease: boolean;
    };

export interface WidgetValueTransform {
  inputMin: number;
  inputMax: number;
  outputMin: number;
  outputMax: number;
  curve?: "linear" | "logarithmic";
}

export interface WidgetBinding {
  id: string;
  portId: string;
  action: unknown;
  policy: WidgetExecutionPolicy;
  transform?: WidgetValueTransform;
}

export interface DeckWidgetInstance {
  id: string;
  definitionId: string;
  packageId?: string;
  contributionId?: string;
  configVersion: number;
  config: Record<string, unknown>;
  appearance?: Record<string, unknown>;
  layouts: Record<string, WidgetPlacement>;
  bindings: WidgetBinding[];
  legacyButtonId?: number;
  enabled: boolean;
}

/** Public projection used by companion clients. It never contains executable actions. */
export interface RemoteDeckWidgetInstance {
  id: string;
  definitionId: string;
  packageId?: string;
  contributionId?: string;
  configVersion: number;
  config: Record<string, unknown>;
  appearance?: Record<string, unknown>;
  layouts: Record<string, WidgetPlacement>;
  availablePorts: string[];
  legacyButtonId?: number;
  enabled: boolean;
}

export interface WidgetIntent {
  /** Per-renderer-mount dedup namespace; authority comes from the host transport. */
  interactionSessionId?: string;
  instanceId: string;
  portId: string;
  legacyButtonId?: number;
  kind: WidgetInputKind;
  phase: WidgetIntentPhase;
  value?: unknown;
  sequence: number;
  timestamp: number;
  inputSource: WidgetInputSource;
}

export interface WidgetRuntimeError {
  code: string;
  message: string;
}

export interface WidgetProviderObservation {
  revision: number;
  availability: "available" | "unavailable";
  observedAt: number;
  values: Record<string, unknown>;
}

export interface WidgetRequestedState {
  requestId: string;
  requestedAt: number;
  values: Record<string, unknown>;
}

export interface WidgetRuntimeState {
  instanceId: string;
  revision: number;
  status: "loading" | "ready" | "executing" | "disconnected" | "error";
  /** Flattened compatibility projection of providerStates. Never optimistic. */
  values: Record<string, unknown>;
  providerStates?: Record<string, WidgetProviderObservation>;
  request?: WidgetRequestedState;
  error?: WidgetRuntimeError;
}

export interface WidgetHostContext {
  mode: WidgetHostMode;
  interactive: boolean;
  selected: boolean;
  locked: boolean;
  runtimeState: WidgetRuntimeState;
  theme: Record<string, unknown>;
}
