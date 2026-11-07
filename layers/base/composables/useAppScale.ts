import { ref } from 'vue';

export interface ScalePreset {
  label: string;
  value: number;
}

export const SCALE_PRESETS: ScalePreset[] = [
  { label: "80% (Very Compact)", value: 0.8 },
  { label: "90% (Compact)", value: 0.9 },
  { label: "100% (Default)", value: 1.0 },
  { label: "110% (Comfortable)", value: 1.1 },
  { label: "120% (Large)", value: 1.2 },
  { label: "130% (Extra Large)", value: 1.3 },
];

const currentScale = ref<number>(1.0);
let isInitialized = false;

export const useAppScale = () => {
  const applyScale = async (scaleVal: number, broadcast = true) => {
    const clamped = Math.min(Math.max(Number(scaleVal.toFixed(2)), 0.75), 1.5);
    currentScale.value = clamped;

    if (import.meta.client) {
      // Clear buggy CSS zoom from both html and containers
      document.documentElement.style.zoom = '';
      
      // Clean, layout-safe REM scaling (16px base * scale)
      const baseFontSize = 16 * clamped;
      document.documentElement.style.fontSize = `${baseFontSize}px`;
      document.documentElement.style.setProperty('--vortideck-ui-scale', `${clamped}`);
      document.documentElement.setAttribute('data-ui-scale', String(clamped));

      try {
        localStorage.setItem('vortideck_ui_scale', String(clamped));
      } catch (err) {
        console.warn('Failed to persist ui scale:', err);
      }

      if (broadcast) {
        try {
          const { emit } = await import('@tauri-apps/api/event');
          await emit('ui-scale-changed', { scale: clamped });
        } catch {
          // Running in standard browser or Tauri API not available
        }
      }
    }
  };

  const setScale = (val: number) => {
    applyScale(val, true);
  };

  const zoomIn = () => {
    const next = Math.round((currentScale.value + 0.1) * 10) / 10;
    setScale(next);
  };

  const zoomOut = () => {
    const prev = Math.round((currentScale.value - 0.1) * 10) / 10;
    setScale(prev);
  };

  const resetZoom = () => {
    setScale(1.0);
  };

  const initScale = async () => {
    if (!import.meta.client) return;

    // Load persisted scale
    try {
      const saved = localStorage.getItem('vortideck_ui_scale');
      if (saved) {
        const parsed = parseFloat(saved);
        if (!isNaN(parsed) && parsed >= 0.7 && parsed <= 1.6) {
          applyScale(parsed, false);
        }
      } else {
        applyScale(1.0, false);
      }
    } catch {
      applyScale(1.0, false);
    }

    if (!isInitialized) {
      isInitialized = true;

      // Re-assert on window focus
      window.addEventListener('focus', () => {
        if (currentScale.value) {
          document.documentElement.style.zoom = '';
          document.documentElement.style.fontSize = `${16 * currentScale.value}px`;
        }
      });

      try {
        const { listen } = await import('@tauri-apps/api/event');
        await listen<{ scale: number }>('ui-scale-changed', (event) => {
          if (event.payload?.scale && event.payload.scale !== currentScale.value) {
            applyScale(event.payload.scale, false);
          }
        });
      } catch {
        // Browser mode
      }
    }
  };

  return {
    scale: currentScale,
    presets: SCALE_PRESETS,
    setScale,
    zoomIn,
    zoomOut,
    resetZoom,
    initScale,
  };
};
