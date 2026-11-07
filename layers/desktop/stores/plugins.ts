import { invoke } from '@tauri-apps/api/core';
import { createPinia, defineStore } from "pinia";
import piniaPluginPersistedstate from "pinia-plugin-persistedstate";

const pinia = createPinia();
pinia.use(piniaPluginPersistedstate);


export interface PluginInfo {
  id: string;
  name: string;
  version: string;
  author: string;
  description: string;
  icon_path?: string;
  web_extensions: boolean;
  enabled: boolean;
}

export const usePluginsStore = defineStore('plugins', {
  state: () => ({
    plugins: [] as PluginInfo[],
    loading: false,
    error: null as string | null,
    lastFetched: null as number | null,
  }),

  getters: {
    // Get all plugins with web extensions
    webExtensionPlugins: (state) => {
      return state.plugins.filter(p => p.web_extensions === true);
    },

    // Get plugin by ID
    getPluginById: (state) => {
      return (id: string) => state.plugins.find(p => p.id === id);
    },

    // Check if plugins need refresh (cache for 5 minutes)
    needsRefresh: (state) => {
      if (state.plugins.length === 0) return true;
      if (!state.lastFetched) return true;
      const fiveMinutes = 5 * 60 * 1000;
      return Date.now() - state.lastFetched > fiveMinutes;
    },
  },

  actions: {
    async fetchPlugins(force = true) {
      // Skip if recently fetched and not forced
      if (!force && !this.needsRefresh) {
        return;
      }

      this.loading = true;
      this.error = null;

      try {
        const pluginData = await invoke<{ plugins?: PluginInfo[] }>('get_detailed_plugin_info');

        if (pluginData?.plugins) {
          this.plugins = pluginData.plugins;
          this.lastFetched = Date.now();
        }
      } catch (err) {
        this.error = err instanceof Error ? err.message : String(err);
        console.error('Failed to fetch plugins:', err);
      } finally {
        this.loading = false;
      }
    },

    async openPluginWindow(pluginId: string) {
      try {
        await invoke('open_plugin_window', { pluginId });
        return true;
      } catch (err) {
        console.error('Failed to open plugin window:', err);
        throw err;
      }
    },

    // Clear cache and force refresh
    async refresh() {
      await this.fetchPlugins(true);
    },
  },
  persist: {
    storage: sessionStorage,
  },
});
