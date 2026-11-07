import { ref, computed } from 'vue';
import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

export interface ActiveAppContext {
  id: string;
  type: 'deck_button' | 'flow_node' | 'deck' | 'overlay' | 'custom';
  title: string;
  subtitle?: string;
  sourceWindow: string;
  timestamp: number;
  data: Record<string, any>;
}

const activeContext = ref<ActiveAppContext | null>(null);
const contextHistory = ref<ActiveAppContext[]>([]);
const MAX_HISTORY = 5;

let isListening = false;
let unlistenFn: UnlistenFn | null = null;

export const useAppAutoContext = () => {
  const currentWindow = getCurrentWindow();

  const initContextListener = async (onUpdate?: (context: ActiveAppContext) => void) => {
    if (isListening) return;
    isListening = true;

    try {
      unlistenFn = await listen<ActiveAppContext>('vortideck-app-context', (event) => {
        const payload = event.payload;
        if (!payload || !payload.id) return;

        activeContext.value = payload;

        // Maintain deduplicated history
        const existingIdx = contextHistory.value.findIndex(c => c.id === payload.id);
        if (existingIdx !== -1) {
          contextHistory.value.splice(existingIdx, 1);
        }
        contextHistory.value.unshift(payload);
        if (contextHistory.value.length > MAX_HISTORY) {
          contextHistory.value.pop();
        }

        if (onUpdate) {
          onUpdate(payload);
        }
      });
    } catch (e) {
      console.warn('Failed to listen for app auto context:', e);
    }
  };

  const disposeContextListener = () => {
    unlistenFn?.();
    unlistenFn = null;
    isListening = false;
  };

  const setContext = async (
    type: ActiveAppContext['type'],
    title: string,
    data: Record<string, any>,
    subtitle?: string
  ) => {
    const newContext: ActiveAppContext = {
      id: `${type}_${Date.now()}`,
      type,
      title,
      subtitle,
      sourceWindow: currentWindow.label,
      timestamp: Date.now(),
      data,
    };

    activeContext.value = newContext;
    contextHistory.value.unshift(newContext);
    if (contextHistory.value.length > MAX_HISTORY) {
      contextHistory.value.pop();
    }

    try {
      await emit('vortideck-app-context', newContext);
    } catch (err) {
      console.warn('Failed to emit app auto context:', err);
    }

    return newContext;
  };

  const clearContext = () => {
    activeContext.value = null;
  };

  const selectFromHistory = (contextItem: ActiveAppContext) => {
    activeContext.value = contextItem;
  };

  return {
    activeContext,
    contextHistory,
    setContext,
    clearContext,
    selectFromHistory,
    initContextListener,
    disposeContextListener,
  };
};
