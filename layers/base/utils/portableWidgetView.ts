import type {
  PortableWidgetAssetRef,
  PortableWidgetGestureBinding,
  PortableWidgetNode,
  PortableWidgetValue,
  PortableWidgetView,
  WidgetDefinition,
} from "@base/types/deckWidget";

const MAX_NODES = 64;
const MAX_DEPTH = 8;
const MAX_STRING = 256;
const MAX_ASSETS = 16;
const MAX_ASSET_BYTES = 2 * 1024 * 1024;
const HASH_PATTERN = /^[a-f0-9]{64}$/;
const PATH_PATTERN = /^[A-Za-z0-9_-]+(?:\.[A-Za-z0-9_-]+)*$/;
const TOKEN_PATTERN = /^[A-Za-z][A-Za-z0-9_.-]{0,63}$/;

const primitives = new Set([
  "stack", "row", "column", "grid", "panel", "text", "icon", "image", "badge",
  "progress", "meter", "press-zone", "toggle", "slider", "rotary", "spacer", "divider",
]);
const containers = new Set(["stack", "row", "column", "grid", "panel", "press-zone"]);
const controls = new Set(["press-zone", "toggle", "slider", "rotary"]);
const allowedNodeKeys = new Set([
  "type", "id", "children", "value", "label", "source", "icon", "direction", "align",
  "justify", "gap", "columns", "min", "max", "step", "colorToken", "backgroundToken",
  "radius", "padding", "fontSize", "fontWeight", "maxLines", "gestures",
]);

export interface PortableWidgetValidationResult {
  valid: boolean;
  errors: string[];
}

const objectValue = (value: unknown): value is Record<string, unknown> => (
  typeof value === "object" && value !== null && !Array.isArray(value)
);

const finiteInRange = (value: unknown, min: number, max: number) => (
  value === undefined || (typeof value === "number" && Number.isFinite(value)
    && value >= min && value <= max)
);

const validShortString = (value: unknown) => (
  typeof value === "string" && value.length > 0 && value.length <= MAX_STRING
);

const validateValue = (value: unknown, path: string, errors: string[]) => {
  if (!objectValue(value)) {
    errors.push(`${path} must be a value binding object`);
    return;
  }
  const keys = Object.keys(value);
  const selectors = ["literal", "config", "state"].filter((key) => key in value);
  if (selectors.length !== 1 || keys.some((key) => ![...selectors, "fallback"].includes(key))) {
    errors.push(`${path} must contain exactly one literal, config, or state selector`);
    return;
  }
  const selector = selectors[0];
  if (selector === "literal") {
    const literal = value.literal;
    if (!(literal === null || ["string", "number", "boolean"].includes(typeof literal))) {
      errors.push(`${path}.literal has an unsupported type`);
    }
    if (typeof literal === "string" && literal.length > MAX_STRING) {
      errors.push(`${path}.literal exceeds ${MAX_STRING} characters`);
    }
  } else if (!validShortString(value[selector]) || !PATH_PATTERN.test(value[selector] as string)) {
    errors.push(`${path}.${selector} is not a valid bounded path`);
  }
};

const validateGesture = (
  gesture: unknown,
  definition: Pick<WidgetDefinition, "inputs">,
  path: string,
  errors: string[],
) => {
  if (!objectValue(gesture)) {
    errors.push(`${path} must be an object`);
    return;
  }
  if (Object.keys(gesture).some((key) => !["gesture", "portId", "delivery"].includes(key))) {
    errors.push(`${path} contains unsupported fields`);
  }
  if (!["press", "hold", "release", "rotate", "drag"].includes(String(gesture.gesture))) {
    errors.push(`${path}.gesture is unsupported`);
  }
  if (!validShortString(gesture.portId)
    || !definition.inputs.some((input) => input.id === gesture.portId)) {
    errors.push(`${path}.portId must reference a declared input`);
  }
  if (gesture.delivery !== undefined
    && !["start_end", "preview_commit", "commit"].includes(String(gesture.delivery))) {
    errors.push(`${path}.delivery is unsupported`);
  }
};

const validateNode = (
  node: unknown,
  definition: Pick<WidgetDefinition, "inputs">,
  depth: number,
  count: { value: number },
  path: string,
  errors: string[],
) => {
  if (!objectValue(node)) {
    errors.push(`${path} must be an object`);
    return;
  }
  count.value += 1;
  if (count.value > MAX_NODES) errors.push(`view exceeds ${MAX_NODES} nodes`);
  if (depth > MAX_DEPTH) errors.push(`${path} exceeds maximum depth ${MAX_DEPTH}`);
  if (!primitives.has(String(node.type))) errors.push(`${path}.type is unsupported`);
  if (Object.keys(node).some((key) => !allowedNodeKeys.has(key))) {
    errors.push(`${path} contains unsupported fields`);
  }
  if (node.id !== undefined && !validShortString(node.id)) errors.push(`${path}.id is invalid`);
  for (const key of ["icon"] as const) {
    if (node[key] !== undefined && !validShortString(node[key])) errors.push(`${path}.${key} is invalid`);
  }
  for (const key of ["colorToken", "backgroundToken"] as const) {
    if (node[key] !== undefined
      && (typeof node[key] !== "string" || !TOKEN_PATTERN.test(node[key] as string))) {
      errors.push(`${path}.${key} is invalid`);
    }
  }
  if (!finiteInRange(node.gap, 0, 64)) errors.push(`${path}.gap is out of range`);
  if (!finiteInRange(node.padding, 0, 64)) errors.push(`${path}.padding is out of range`);
  if (!finiteInRange(node.radius, 0, 64)) errors.push(`${path}.radius is out of range`);
  if (!finiteInRange(node.fontSize, 8, 72)) errors.push(`${path}.fontSize is out of range`);
  if (!finiteInRange(node.columns, 1, 8)) errors.push(`${path}.columns is out of range`);
  if (!finiteInRange(node.maxLines, 1, 8)) errors.push(`${path}.maxLines is out of range`);
  for (const key of ["min", "max", "step"] as const) {
    if (node[key] !== undefined && (typeof node[key] !== "number" || !Number.isFinite(node[key]))) {
      errors.push(`${path}.${key} must be finite`);
    }
  }
  if (typeof node.min === "number" && typeof node.max === "number" && node.min >= node.max) {
    errors.push(`${path}.min must be lower than max`);
  }
  for (const key of ["value", "label"] as const) {
    if (node[key] !== undefined) validateValue(node[key], `${path}.${key}`, errors);
  }
  if (node.source !== undefined) {
    if (objectValue(node.source) && "asset" in node.source) {
      if (Object.keys(node.source).length !== 1 || !validShortString(node.source.asset)) {
        errors.push(`${path}.source asset reference is invalid`);
      }
    } else validateValue(node.source, `${path}.source`, errors);
  }
  if (node.gestures !== undefined) {
    if (!Array.isArray(node.gestures) || node.gestures.length > 8) {
      errors.push(`${path}.gestures must be an array of at most 8 entries`);
    } else {
      node.gestures.forEach((gesture, index) => validateGesture(
        gesture, definition, `${path}.gestures[${index}]`, errors,
      ));
    }
  }
  if (!controls.has(String(node.type)) && Array.isArray(node.gestures) && node.gestures.length) {
    errors.push(`${path}.gestures are allowed only on control nodes`);
  }
  if (node.children !== undefined) {
    if (!containers.has(String(node.type)) || !Array.isArray(node.children) || node.children.length > 16) {
      errors.push(`${path}.children are invalid for this node`);
    } else {
      node.children.forEach((child, index) => validateNode(
        child, definition, depth + 1, count, `${path}.children[${index}]`, errors,
      ));
    }
  }
};

const validateAsset = (asset: unknown, index: number, errors: string[]) => {
  const path = `assets[${index}]`;
  if (!objectValue(asset)) {
    errors.push(`${path} must be an object`);
    return;
  }
  if (Object.keys(asset).some((key) => ![
    "id", "sha256", "mediaType", "byteLength", "purpose",
  ].includes(key))) errors.push(`${path} contains unsupported fields`);
  if (!validShortString(asset.id)) errors.push(`${path}.id is invalid`);
  if (typeof asset.sha256 !== "string" || !HASH_PATTERN.test(asset.sha256)) {
    errors.push(`${path}.sha256 is invalid`);
  }
  if (!["image/png", "image/jpeg", "image/webp"].includes(String(asset.mediaType))) {
    errors.push(`${path}.mediaType is unsupported`);
  }
  if (!Number.isInteger(asset.byteLength)
    || Number(asset.byteLength) <= 0 || Number(asset.byteLength) > MAX_ASSET_BYTES) {
    errors.push(`${path}.byteLength is out of range`);
  }
  if (!["icon", "artwork", "background"].includes(String(asset.purpose))) {
    errors.push(`${path}.purpose is unsupported`);
  }
};

export const validatePortableWidgetView = (
  view: unknown,
  definition: Pick<WidgetDefinition, "inputs">,
): PortableWidgetValidationResult => {
  const errors: string[] = [];
  if (!objectValue(view)) return { valid: false, errors: ["view must be an object"] };
  if (Object.keys(view).some((key) => !["schemaVersion", "root", "assets"].includes(key))) {
    errors.push("view contains unsupported fields");
  }
  if (view.schemaVersion !== 1) errors.push("schemaVersion must be 1");
  validateNode(view.root, definition, 1, { value: 0 }, "root", errors);
  const ids = new Set<string>();
  if (view.assets !== undefined) {
    if (!Array.isArray(view.assets) || view.assets.length > MAX_ASSETS) {
      errors.push(`assets must contain at most ${MAX_ASSETS} entries`);
    } else {
      view.assets.forEach((asset, index) => {
        validateAsset(asset, index, errors);
        if (objectValue(asset) && typeof asset.id === "string") {
          if (ids.has(asset.id)) errors.push(`assets[${index}].id is duplicated`);
          ids.add(asset.id);
        }
      });
    }
  }
  const referenced = new Set<string>();
  const visit = (node: unknown) => {
    if (!objectValue(node)) return;
    if (objectValue(node.source) && typeof node.source.asset === "string") {
      referenced.add(node.source.asset);
    }
    if (Array.isArray(node.children)) node.children.forEach(visit);
  };
  visit(view.root);
  for (const assetId of referenced) {
    if (!ids.has(assetId)) errors.push(`view references undeclared asset '${assetId}'`);
  }
  return { valid: errors.length === 0, errors: [...new Set(errors)] };
};

export const resolvePortableWidgetValue = (
  value: PortableWidgetValue | undefined,
  config: Record<string, unknown>,
  state: Record<string, unknown>,
) => {
  if (!value) return undefined;
  if ("literal" in value) return value.literal;
  const source = "config" in value ? config : state;
  const path = "config" in value ? value.config : value.state;
  const resolved = path.split(".").reduce<unknown>((current, segment) => (
    objectValue(current) ? current[segment] : undefined
  ), source);
  return resolved ?? value.fallback;
};

export type {
  PortableWidgetAssetRef,
  PortableWidgetGestureBinding,
  PortableWidgetNode,
  PortableWidgetValue,
  PortableWidgetView,
};
