import { createWidgetOptions, inferWidget } from "./widgetInference";

type DefaultValueResolver = (type: string, enumOptions?: string[]) => any;

const defaultValueForType: DefaultValueResolver = (type, enumOptions) => {
  if (enumOptions && enumOptions.length > 0) {
    return enumOptions[0];
  }

  switch (type) {
    case "string":
      return "";
    case "number":
      return 0;
    case "boolean":
      return false;
    case "array":
      return [];
    case "object":
      return {};
    default:
      return "";
  }
};

/**
 * Convert JSON Schema properties to flow node inputs.
 * Keeps a single source of truth for required/default/widget/displayOptions inference.
 */
export const convertJsonSchemaToNodeInputs = (
  schema: any,
  options: { defaultValueResolver?: DefaultValueResolver } = {}
) => {
  if (!schema?.properties) return [];

  const resolveDefault = options.defaultValueResolver || defaultValueForType;
  const requiredFields = Array.isArray(schema.required) ? schema.required : [];

  return Object.entries(schema.properties).map(([key, rawProp]: [string, any]) => {
    const prop = rawProp || {};
    const widget = inferWidget(prop);

    let isRequired = true;
    if (prop.optional === true) {
      isRequired = false;
    } else if (requiredFields.length > 0) {
      isRequired = requiredFields.includes(key);
    }

    return {
      id: key,
      name: prop.description || key,
      type: prop.type || "string",
      value:
        prop.default !== undefined
          ? prop.default
          : resolveDefault(prop.type, prop.enum),
      attached: false,
      enum: prop.enum || undefined,
      required: isRequired,
      forceInput: prop.forceInput || false,
      displayOptions: prop.displayOptions || undefined,
      widget,
      widgetOptions: createWidgetOptions(prop, widget),
    };
  });
};

export const getDefaultValueForSchemaType = defaultValueForType;
