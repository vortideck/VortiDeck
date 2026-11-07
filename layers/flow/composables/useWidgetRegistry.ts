/**
 * Widget Registry System
 *
 * Centralized registry for all flow node widgets (core + plugins).
 * Enables plugins to register custom widgets that can be used by any node.
 */

import type { Component } from 'vue';

export interface WidgetDefinition {
  /** Unique widget identifier (e.g., 'slider', 'duration', 'plugin.foo.chart') */
  id: string;

  /** Vue component for rendering the widget */
  component: Component;

  /** Widget source: 'core' for built-in, 'plugin' for plugin-provided */
  source: 'core' | 'plugin';

  /** Plugin ID if this widget comes from a plugin */
  pluginId?: string;

  /** Schema validation and metadata */
  schema?: {
    /** Data types this widget accepts (e.g., ['number', 'string']) */
    accepts?: string[];

    /** Additional widget-specific options */
    options?: Record<string, any>;
  };

  /** Human-readable description */
  description?: string;
}

/**
 * Global widget registry - stores all available widgets
 */
const widgetRegistry = new Map<string, WidgetDefinition>();

/**
 * Register a widget in the global registry
 *
 * @param definition Widget definition to register
 * @throws Error if widget ID is already registered
 */
export function registerWidget(definition: WidgetDefinition): void {
  if (widgetRegistry.has(definition.id)) {
    console.warn(`Widget '${definition.id}' is already registered. Overwriting.`);
  }

  widgetRegistry.set(definition.id, definition);
  console.log(`✅ Registered widget: ${definition.id} (${definition.source})`);
}

/**
 * Get a widget definition by its ID
 *
 * @param id Widget identifier
 * @returns Widget definition or undefined if not found
 */
export function getWidget(id: string): WidgetDefinition | undefined {
  return widgetRegistry.get(id);
}

/**
 * Get the Vue component for a widget
 *
 * @param id Widget identifier
 * @returns Vue component or undefined if widget not found
 */
export function getWidgetComponent(id: string): Component | undefined {
  return widgetRegistry.get(id)?.component;
}

/**
 * Get all widgets from a specific plugin
 *
 * @param pluginId Plugin identifier
 * @returns Array of widget definitions from that plugin
 */
export function getWidgetsByPlugin(pluginId: string): WidgetDefinition[] {
  return Array.from(widgetRegistry.values())
    .filter(widget => widget.pluginId === pluginId);
}

/**
 * Get all registered widgets
 *
 * @returns Array of all widget definitions
 */
export function getAllWidgets(): WidgetDefinition[] {
  return Array.from(widgetRegistry.values());
}

/**
 * Unregister a widget
 *
 * @param id Widget identifier to remove
 * @returns true if widget was removed, false if it didn't exist
 */
export function unregisterWidget(id: string): boolean {
  const existed = widgetRegistry.has(id);
  widgetRegistry.delete(id);

  if (existed) {
    console.log(`🗑️ Unregistered widget: ${id}`);
  }

  return existed;
}

/**
 * Unregister all widgets from a plugin
 *
 * @param pluginId Plugin identifier
 * @returns Number of widgets unregistered
 */
export function unregisterPluginWidgets(pluginId: string): number {
  const pluginWidgets = getWidgetsByPlugin(pluginId);

  for (const widget of pluginWidgets) {
    widgetRegistry.delete(widget.id);
  }

  console.log(`🗑️ Unregistered ${pluginWidgets.length} widgets from plugin: ${pluginId}`);
  return pluginWidgets.length;
}

/**
 * Check if a widget is registered
 *
 * @param id Widget identifier
 * @returns true if widget exists in registry
 */
export function hasWidget(id: string): boolean {
  return widgetRegistry.has(id);
}

/**
 * Clear all widgets from registry (useful for testing/cleanup)
 */
export function clearRegistry(): void {
  widgetRegistry.clear();
  console.log('🧹 Cleared widget registry');
}

/**
 * Get registry statistics
 *
 * @returns Registry stats object
 */
export function getRegistryStats() {
  const all = getAllWidgets();
  const core = all.filter(w => w.source === 'core').length;
  const plugin = all.filter(w => w.source === 'plugin').length;

  return {
    total: all.length,
    core,
    plugin,
    plugins: new Set(all.filter(w => w.pluginId).map(w => w.pluginId)).size
  };
}
