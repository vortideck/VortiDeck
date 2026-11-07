import { useMainStore } from '../stores/Main';

export default defineNuxtPlugin(async (nuxtApp) => {
  if (typeof window === 'undefined') return;

  const mainStore = useMainStore();

  const getSystemTheme = (): 'light' | 'dark' => {
    return window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  };

  const applyThemeToDOM = () => {
    const html = document.documentElement;
    const colorMode = mainStore.getColorMode as 'light' | 'dark' | 'system';
    const palette = (mainStore.getThemePalette || 'vortideck').toLowerCase();
    
    // Resolve mode
    const resolvedMode = colorMode === 'system' ? getSystemTheme() : colorMode;

    // Remove all previous theme classes
    const classesToRemove: string[] = [];
    html.classList.forEach((cls) => {
      if (cls.startsWith('theme-') || cls === 'light' || cls === 'dark') {
        classesToRemove.push(cls);
      }
    });
    classesToRemove.forEach((cls) => html.classList.remove(cls));

    // Apply primary light/dark marker
    html.classList.add(resolvedMode);

    // Apply specific palette class
    const paletteClass = `theme-${palette}-${resolvedMode}`;
    html.classList.add(paletteClass);

    // Backward-compatibility aliases
    if (palette === 'github') {
      html.classList.add(`theme-github-${resolvedMode}`);
      html.classList.add(`theme-default-${resolvedMode}`);
    } else {
      html.classList.add(`theme-${palette}-${resolvedMode}`);
      html.classList.add(`theme-default-${resolvedMode}`);
    }
  };

  // 1. Initial theme resolution from localStorage or store
  try {
    const savedMode = (localStorage.getItem('vortideck_color_mode') || localStorage.getItem('vortideck_theme')) as 'light' | 'dark' | 'system' | null;
    const savedPalette = localStorage.getItem('vortideck_theme_palette');
    if (savedMode && (savedMode === 'light' || savedMode === 'dark' || savedMode === 'system')) {
      mainStore.colorMode = savedMode;
      mainStore.theme = savedMode === 'system' ? 'dark' : savedMode;
    }
    if (savedPalette) {
      mainStore.themePalette = savedPalette;
    }
  } catch (err) {
    console.warn('Could not read saved theme settings:', err);
  }

  // Apply immediately to HTML
  applyThemeToDOM();

  // Watch Pinia store theme changes
  watch(
    [() => mainStore.getColorMode, () => mainStore.getThemePalette],
    () => {
      applyThemeToDOM();
    }
  );

  // System theme listener for real-time OS theme changes
  if (window.matchMedia) {
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
      if (mainStore.getColorMode === 'system') {
        applyThemeToDOM();
      }
    });
  }

  // 2. Cross-window synchronization via Tauri events
  try {
    const { listen } = await import('@tauri-apps/api/event');
    await listen<{ colorMode?: 'light' | 'dark' | 'system'; themePalette?: string }>('theme-settings-changed', (event) => {
      if (event.payload?.colorMode) mainStore.colorMode = event.payload.colorMode;
      if (event.payload?.themePalette) mainStore.themePalette = event.payload.themePalette;
      applyThemeToDOM();
    });
  } catch {
    // Browser mode: storage listener fallback
    window.addEventListener('storage', (e) => {
      if (e.key === 'vortideck_color_mode' && e.newValue) {
        mainStore.colorMode = e.newValue as 'light' | 'dark' | 'system';
        applyThemeToDOM();
      }
      if (e.key === 'vortideck_theme_palette' && e.newValue) {
        mainStore.themePalette = e.newValue;
        applyThemeToDOM();
      }
    });
  }
});
