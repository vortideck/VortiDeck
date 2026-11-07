/**
 * Core Widget Auto-Registration
 *
 * Automatically registers all built-in widgets with the widget registry.
 * Call this once on app initialization.
 */

import { registerWidget } from './useWidgetRegistry';

// Import all core widgets
import SliderWidget from '../components/Flow/Widgets/SliderWidget.vue';
import ColorWidget from '../components/Flow/Widgets/ColorWidget.vue';
import ToggleWidget from '../components/Flow/Widgets/ToggleWidget.vue';
import FileSelectorWidget from '../components/Flow/Widgets/FileSelectorWidget.vue';
import MultiSelectWidget from '../components/Flow/Widgets/MultiSelectWidget.vue';
import SelectWidget from '../components/Flow/Widgets/SelectWidget.vue';
import PreviewFileWidget from '../components/Flow/Widgets/PreviewFileWidget.vue';
import DurationPickerWidget from '../components/Flow/Widgets/DurationPickerWidget.vue';

/**
 * Register all core widgets
 *
 * This should be called once during app initialization
 * to populate the widget registry with built-in widgets.
 */
export function registerCoreWidgets(): void {
  console.log('🔧 Registering core widgets...');

  registerWidget({
    id: 'slider',
    component: SliderWidget,
    source: 'core',
    schema: {
      accepts: ['number'],
      options: { min: true, max: true, step: true }
    },
    description: 'Numeric slider for bounded values'
  });

  registerWidget({
    id: 'color',
    component: ColorWidget,
    source: 'core',
    schema: {
      accepts: ['string']
    },
    description: 'Color picker with hex input'
  });

  registerWidget({
    id: 'toggle',
    component: ToggleWidget,
    source: 'core',
    schema: {
      accepts: ['boolean']
    },
    description: 'Boolean toggle switch'
  });

  registerWidget({
    id: 'file',
    component: FileSelectorWidget,
    source: 'core',
    schema: {
      accepts: ['string'],
      options: { mode: true, accept: true }
    },
    description: 'File/directory selector with open/save/directory modes'
  });

  registerWidget({
    id: 'multiselect',
    component: MultiSelectWidget,
    source: 'core',
    schema: {
      accepts: ['array']
    },
    description: 'Multi-selection dropdown'
  });

  registerWidget({
    id: 'select',
    component: SelectWidget,
    source: 'core',
    schema: {
      accepts: ['string', 'number']
    },
    description: 'Single-selection dropdown'
  });

  registerWidget({
    id: 'preview-file',
    component: PreviewFileWidget,
    source: 'core',
    schema: {
      accepts: ['string'],
      options: { fileData: true, fileName: true }
    },
    description: 'Universal file preview (images, text, code, etc.)'
  });

  registerWidget({
    id: 'duration',
    component: DurationPickerWidget,
    source: 'core',
    schema: {
      accepts: ['number']
    },
    description: 'Duration picker with hours/minutes/seconds'
  });

  console.log('✅ Core widgets registered');
}

/**
 * List of all core widget IDs
 */
export const CORE_WIDGET_IDS = [
  'slider',
  'color',
  'toggle',
  'file',
  'multiselect',
  'select',
  'preview-file',
  'duration'
] as const;

export type CoreWidgetId = typeof CORE_WIDGET_IDS[number];
