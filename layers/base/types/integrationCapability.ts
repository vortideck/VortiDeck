export type CapabilityValueType =
  | "any"
  | "string"
  | "number"
  | "integer"
  | "boolean"
  | "object"
  | "array";

export type CapabilityWidget =
  | "text"
  | "textarea"
  | "number"
  | "color"
  | "toggle"
  | "select"
  | "multi_select"
  | "json";

export interface CapabilityOption {
  label: string;
  value: unknown;
  parameters?: CapabilityParameter[];
}

export interface CapabilityParameter {
  id: string;
  label: string;
  description: string;
  valueType: CapabilityValueType;
  required: boolean;
  widget: CapabilityWidget;
  defaultValue?: unknown;
  options?: CapabilityOption[];
  optionSource?: { resolverId: string; dependsOn?: string[] } | null;
}

export interface CapabilityDefinition {
  integrationId: string;
  capabilityId: string;
  version: number;
  kind: "action" | "query" | "trigger";
  presentation: {
    name: string;
    description: string;
    category: string;
    icon?: string | null;
    aliases?: string[];
  };
  inputs: CapabilityParameter[];
  outputs?: Array<{ id: string; label: string; valueType: CapabilityValueType }>;
  allowAdditionalInputs: boolean;
  contract: Record<string, unknown>;
}

export const capabilityOptionKey = (value: unknown): string => JSON.stringify(value);

export const visibleCapabilityInputs = (
  inputs: CapabilityParameter[],
  values: Record<string, unknown>,
): CapabilityParameter[] => {
  const visible: CapabilityParameter[] = [];
  const append = (parameters: CapabilityParameter[]) => {
    for (const parameter of parameters) {
      visible.push(parameter);
      const selected = parameter.options?.find(
        (option) => capabilityOptionKey(option.value) === capabilityOptionKey(values[parameter.id]),
      );
      if (selected?.parameters?.length) append(selected.parameters);
    }
  };
  append(inputs);
  return visible;
};

export const validateCapabilityValues = (
  capability: CapabilityDefinition | null,
  values: Record<string, unknown>,
): boolean => {
  if (!capability) return false;
  return visibleCapabilityInputs(capability.inputs || [], values).every((input) => {
    if (!input.required) return true;
    const value = values[input.id];
    return value !== undefined && value !== null && value !== "";
  });
};
