import { LazyStore } from '@tauri-apps/plugin-store';

type ThemeMode = 'system' | 'dark' | 'light';

const STORAGE_KEY = 'theme';
const STORE_FILE = 'settings.json';

let store: LazyStore | null = null;

function getStore(): LazyStore | null {
  if (store) return store;
  try {
    store = new LazyStore(STORE_FILE);
    return store;
  } catch {
    return null;
  }
}

function getSavedThemeSync(): ThemeMode {
  if (typeof localStorage !== 'undefined') {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved && ['system', 'dark', 'light'].includes(saved)) {
      return saved as ThemeMode;
    }
  }
  return 'dark';
}

function getSystemTheme(): 'dark' | 'light' {
  if (typeof window !== 'undefined' && window.matchMedia) {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }
  return 'dark';
}

function resolveTheme(mode: ThemeMode): 'dark' | 'light' {
  if (mode === 'system') return getSystemTheme();
  return mode;
}

function applyTheme(theme: 'dark' | 'light') {
  if (typeof document === 'undefined') return;
  const html = document.documentElement;
  html.classList.remove('theme-mobile-dark', 'theme-mobile-light');
  html.classList.add(theme === 'dark' ? 'theme-mobile-dark' : 'theme-mobile-light');
}

// Module-level state from localStorage (sync, instant)
// Theme class is already applied by inline <head> script in nuxt.config
const themeMode = ref<ThemeMode>(getSavedThemeSync());
const resolvedTheme = ref<'dark' | 'light'>(resolveTheme(themeMode.value));

let listenerAttached = false;

export function useTheme() {
  async function setThemeMode(mode: ThemeMode) {
    themeMode.value = mode;
    resolvedTheme.value = resolveTheme(mode);
    applyTheme(resolvedTheme.value);

    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(STORAGE_KEY, mode);
    }
    const s = getStore();
    if (s) {
      await s.set(STORAGE_KEY, mode);
      await s.save();
    }
  }

  async function initTheme() {
    // Check Tauri store for persisted value (may differ from localStorage on Android)
    const s = getStore();
    if (s) {
      try {
        const saved = await s.get<ThemeMode>(STORAGE_KEY);
        if (saved && ['system', 'dark', 'light'].includes(saved) && saved !== themeMode.value) {
          themeMode.value = saved;
          resolvedTheme.value = resolveTheme(saved);
          applyTheme(resolvedTheme.value);
          if (typeof localStorage !== 'undefined') {
            localStorage.setItem(STORAGE_KEY, saved);
          }
        }
      } catch {
        // Tauri store not available (browser dev)
      }
    }

    // Listen for OS theme changes (only once)
    if (!listenerAttached && typeof window !== 'undefined' && window.matchMedia) {
      listenerAttached = true;
      window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
        if (themeMode.value === 'system') {
          resolvedTheme.value = getSystemTheme();
          applyTheme(resolvedTheme.value);
        }
      });
    }
  }

  const themeClass = computed(() =>
    resolvedTheme.value === 'dark' ? 'theme-mobile-dark' : 'theme-mobile-light'
  );

  return {
    themeMode: readonly(themeMode),
    resolvedTheme: readonly(resolvedTheme),
    themeClass,
    setThemeMode,
    initTheme,
  };
}
