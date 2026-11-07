export interface WidgetConfigValidationResult {
  valid: boolean;
  value: Record<string, unknown>;
  errors: string[];
}

type JsonSchema = Record<string, unknown>;

const matchesType = (value: unknown, type: unknown) => {
  if (type === "string") return typeof value === "string";
  if (type === "number") return typeof value === "number" && Number.isFinite(value);
  if (type === "integer") return Number.isInteger(value);
  if (type === "boolean") return typeof value === "boolean";
  if (type === "array") return Array.isArray(value);
  if (type === "object") return value !== null && typeof value === "object" && !Array.isArray(value);
  return true;
};

export const validateWidgetConfig = (
  schema: JsonSchema,
  config: Record<string, unknown>,
): WidgetConfigValidationResult => {
  const properties = schema.properties && typeof schema.properties === "object"
    ? schema.properties as Record<string, JsonSchema>
    : {};
  const value = { ...config };
  const errors: string[] = [];

  for (const [key, property] of Object.entries(properties)) {
    if (value[key] === undefined && property.default !== undefined) value[key] = property.default;
    const candidate = value[key];
    if (candidate === undefined) continue;
    if (!matchesType(candidate, property.type)) {
      errors.push(`${key} must be ${String(property.type ?? "valid")}`);
      continue;
    }
    if (Array.isArray(property.enum) && !property.enum.some((entry) => Object.is(entry, candidate))) {
      errors.push(`${key} is not an allowed value`);
    }
    if (typeof candidate === "number") {
      if (typeof property.minimum === "number" && candidate < property.minimum) {
        errors.push(`${key} must be at least ${property.minimum}`);
      }
      if (typeof property.maximum === "number" && candidate > property.maximum) {
        errors.push(`${key} must be at most ${property.maximum}`);
      }
    }
  }
  for (const key of Array.isArray(schema.required) ? schema.required : []) {
    if (typeof key === "string" && value[key] === undefined) errors.push(`${key} is required`);
  }
  if (schema.additionalProperties === false) {
    for (const key of Object.keys(value)) {
      if (!properties[key]) errors.push(`${key} is not supported`);
    }
  }
  return { valid: errors.length === 0, value, errors };
};
