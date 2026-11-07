/**
 * Widget inference utility
 * Automatically detects the appropriate UI widget type from JSON Schema properties
 */

/**
 * Infer widget type from JSON Schema property definition
 *
 * @param prop - JSON Schema property object
 * @returns Widget type identifier ('slider', 'color', 'toggle', etc.)
 *
 * @example
 * inferWidget({ type: 'number', minimum: 0, maximum: 100 }) // returns 'slider'
 * inferWidget({ type: 'string', format: 'color' }) // returns 'color'
 * inferWidget({ type: 'boolean' }) // returns 'toggle'
 */
export function inferWidget(prop: any): string {
  // Explicit widget hint takes precedence
  if (prop['ui:widget']) return prop['ui:widget'];

  // Format-based inference
  if (prop.format === 'color') return 'color';
  if (prop.format === 'file') return 'file';

  // Array with enum = multi-select
  if (prop.type === 'array') {
    // Check if items have enum, or if there's a top-level enum
    if (prop.items?.enum || prop.enum) {
      return 'multiselect';
    }
    return 'text'; // Array without enum falls back to text
  }

  // Enum = select dropdown (single selection)
  if (prop.enum && prop.enum.length > 0) return 'select';

  // Type-based inference
  if (prop.type === 'boolean') return 'toggle';

  if (prop.type === 'number') {
    // If it has min/max, use slider
    if (prop.minimum !== undefined && prop.maximum !== undefined) {
      return 'slider';
    }
    return 'number';
  }

  if (prop.type === 'string') {
    if (prop.maxLength && prop.maxLength > 100) return 'textarea';
    return 'text';
  }

  return 'text'; // default fallback
}

/**
 * Create widget options from JSON Schema property
 *
 * @param prop - JSON Schema property object
 * @param widget - Widget type (from inferWidget)
 * @returns Widget options object
 */
export function createWidgetOptions(prop: any, widget: string) {
  return {
    min: prop.minimum,
    max: prop.maximum,
    step: prop.multipleOf || (prop.type === 'number' ? 1 : undefined),
    accept: prop.typeOptions?.acceptedTypes,
    multiline: widget === 'textarea',
    rows: prop.rows || 3,
    // For multi-select, extract enum from items or top-level
    options: prop.items?.enum || prop.enum || [],
    // For file-selector, pass mode from typeOptions or ui:fileMode
    mode: prop.typeOptions?.mode || prop['ui:fileMode'] || (widget === 'file-selector' ? 'open' : undefined),
  };
}
