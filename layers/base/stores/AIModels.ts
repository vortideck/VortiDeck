/**
 * AI Models Store - Centralized state management for AI provider models
 * Uses Pinia for reactive global state with automatic persistence
 */

import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import { fetch as tauriFetch } from '@tauri-apps/plugin-http';

export interface AIModel {
  id: string;
  name: string;
  provider: string;
  contextWindow?: number;
  description?: string;
  supportsVision?: boolean;
  supportsTools?: boolean;
  supportsTemperature?: boolean;
  createdAt?: number;
}

interface ModelCache {
  lastUpdated: string;
  providers: Record<string, {
    models: AIModel[];
    fetchedAt: string;
  }>;
}

const CACHE_VALIDITY_HOURS = 24;

export const useAIModelsStore = defineStore('aiModels', {
  state: () => ({
    models: {} as Record<string, AIModel[]>, // provider -> models mapping
    providerConfigs: [] as any[], // Store provider configs including default_model
    lastUpdated: null as string | null,
    isLoading: false,
  }),

  getters: {
    /**
     * Get models for a specific provider
     */
    getModelsByProvider: (state) => (provider: string): AIModel[] => {
      return state.models[provider] || [];
    },

    /**
     * Get all available providers
     */
    availableProviders: (state): string[] => {
      return Object.keys(state.models);
    },

    /**
     * Get providers that have models loaded
     */
    providersWithModels: (state): string[] => {
      return Object.entries(state.models)
        .filter(([_, models]) => models.length > 0)
        .map(([provider]) => provider);
    },

    /**
     * Get model by ID across all providers
     */
    getModelById: (state) => (modelId: string): AIModel | undefined => {
      for (const models of Object.values(state.models)) {
        const found = models.find(m => m.id === modelId);
        if (found) return found;
      }
      return undefined;
    },

    /**
     * Get formatted model name
     */
    getModelName: (state) => (modelId: string): string => {
      for (const models of Object.values(state.models)) {
        const found = models.find(m => m.id === modelId);
        if (found) return found.name;
      }
      return modelId;
    },

    /**
     * Check if cache is stale
     */
    isCacheStale: (state): boolean => {
      if (!state.lastUpdated) return true;
      const lastUpdate = new Date(state.lastUpdated);
      const hoursSinceUpdate = (Date.now() - lastUpdate.getTime()) / (1000 * 60 * 60);
      return hoursSinceUpdate > CACHE_VALIDITY_HOURS;
    },

    /**
     * Get the default model from the first provider config
     */
    defaultModel: (state): string | undefined => {
      if (state.providerConfigs.length === 0) return undefined;
      const firstProvider = state.providerConfigs[0];
      return firstProvider?.default_model || firstProvider?.defaultModel;
    },
  },

  actions: {
    /**
     * Initialize models from cache or fetch fresh
     * @param providerConfigs - Provider configurations
     * @param allowRefresh - Whether to fetch from APIs if cache is stale (default: false)
     *                      Only main window should set this to true
     */
    async initialize(providerConfigs: any[], allowRefresh: boolean = false) {
      console.log('🔄 Initializing AI models store...');

      // Store provider configs (includes default_model setting)
      this.providerConfigs = providerConfigs;

      try {
        // Load from cache file
        const cache = await this.loadCache();

        if (cache) {
          console.log('✅ Loaded models from cache');
          this.models = Object.fromEntries(
            Object.entries(cache.providers).map(([provider, data]: [string, any]) => [
              provider,
              data.models,
            ])
          );
          this.lastUpdated = cache.lastUpdated;

          // Only refresh if explicitly allowed (main window only)
          if (this.isCacheStale && allowRefresh) {
            console.log('⏰ Cache is stale, refreshing from APIs (main window only)...');
            await this.refresh(providerConfigs);
          } else if (this.isCacheStale) {
            console.log('⚠️ Cache is stale but using cached data (non-main window)');
          }
        } else if (allowRefresh) {
          console.log('⏰ No cache found, fetching fresh models (main window only)...');
          await this.refresh(providerConfigs);
        } else {
          console.log('⚠️ No cache found and refresh not allowed - models unavailable');
        }
      } catch (err) {
        console.error('❌ Failed to initialize AI models:', err);
      }
    },

    /**
     * Refresh models from all provider APIs
     */
    async refresh(providerConfigs: any[]) {
      this.isLoading = true;
      console.log('🔄 Refreshing AI models from provider APIs...');

      // Store provider configs (includes default_model setting)
      this.providerConfigs = providerConfigs;

      const newModels: Record<string, AIModel[]> = {};

      for (const config of providerConfigs) {
        try {
          const models = await this.fetchModelsForProvider(config);
          newModels[config.provider] = models;
          console.log(`✅ Fetched ${models.length} models for ${config.provider}`);
        } catch (err) {
          console.error(`❌ Failed to fetch models for ${config.provider}:`, err);
          newModels[config.provider] = [];
        }
      }

      // Update state
      this.models = newModels;
      this.lastUpdated = new Date().toISOString();
      this.isLoading = false;

      // Persist to cache
      await this.saveCache();

      console.log('✅ Models refreshed and cached');
    },

    /**
     * Fetch models from a single provider
     */
    async fetchModelsForProvider(config: any): Promise<AIModel[]> {
      const { provider, apiKey, url, modelsUrl, default_model, name, baseUrl, port, modelsEndpoint } = config;

      // Construct endpoint from components if available (flexible config)
      const endpoint = this.getModelsEndpoint(provider, url, modelsUrl, baseUrl, port, modelsEndpoint);

      // For Ollama without endpoint, return fallback (shouldn't happen as we construct it from baseUrl)
      if (!endpoint && provider === 'ollama') {
        console.warn(`⚠️ Ollama provider "${name || 'unknown'}" has no models endpoint, using fallback`);
        return [{
          id: default_model || 'llama2',
          name: name || default_model || 'Default Ollama Model',
          provider: 'ollama',
          contextWindow: 4096,
          supportsTemperature: true,
          supportsTools: false, // Ollama models may not support tools
        }];
      }

      // For custom providers without a models endpoint, return a fallback model
      if (!endpoint && provider === 'custom') {
        console.warn(`⚠️ Custom provider "${name || 'unknown'}" has no models endpoint, using fallback`);
        return [{
          id: default_model || 'default',
          name: name || default_model || 'Default Model',
          provider: 'custom',
          contextWindow: 4096,
          supportsTemperature: true,
          supportsTools: true,
        }];
      }

      // For custom providers without a models endpoint, return a fallback model
      if (!endpoint && provider === 'custom') {
        console.warn(`⚠️ Custom provider "${name || 'unknown'}" has no models endpoint, using fallback`);
        return [{
          id: default_model || 'default',
          name: name || default_model || 'Default Model',
          provider: 'custom',
          contextWindow: 4096,
          supportsTemperature: true,
          supportsTools: true,
        }];
      }

      if (!endpoint) {
        throw new Error(`No models endpoint for provider: ${provider}`);
      }

      // Build headers - Authorization is optional for custom providers without API keys
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
      };

      if (apiKey && apiKey.trim()) {
        headers['Authorization'] = `Bearer ${apiKey}`;
      }

      // Fetch from provider API
      const response = await tauriFetch(endpoint, {
        method: 'GET',
        headers,
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const data = await response.json() as any;
      const rawModels = data.data || data.models || [];

      // Filter and map models
      return this.filterAndMapModels(rawModels, provider);
    },

    /**
     * Get models endpoint for provider
     */
    getModelsEndpoint(
      provider: string,
      url?: string,
      customModelsUrl?: string,
      baseUrl?: string,
      port?: number,
      modelsEndpoint?: string
    ): string {
      // For custom providers with flexible config, construct from components
      if (provider === 'custom' && baseUrl && port) {
        const endpoint = modelsEndpoint || '/v1/models';
        return `http://${baseUrl}:${port}${endpoint}`;
      }

      // Always use custom models URL if provided (backward compatibility)
      if (customModelsUrl) return customModelsUrl;

      // Known provider endpoints (full URLs, no construction)
      const knownEndpoints: Record<string, string> = {
        openai: 'https://api.openai.com/v1/models',
        anthropic: 'https://api.anthropic.com/v1/models',
        openrouter: 'https://openrouter.ai/api/v1/models',
      };

      return knownEndpoints[provider] || '';
    },

    /**
     * Filter and map raw models to AIModel interface
     */
    filterAndMapModels(rawModels: any[], provider: string): AIModel[] {
      const excludePatterns = [
        'embedding',
        'tts-',
        'whisper',
        'moderation',
        'dall-e',
        'codex',
        'davinci-002',
        'babbage-002',
      ];

      // For custom providers, accept all models (no restrictive filtering)
      const isCustomProvider = provider === 'custom';

      return rawModels
        .filter((m: any) => {
          // Support both OpenAI format (m.id) and Ollama format (m.name, m.model)
          const id = (m.id || m.name || m.model || '').toLowerCase();

          if (!id) return false; // Skip if no identifier

          // Exclude non-chat models (apply to all providers)
          const shouldExclude = excludePatterns.some(pattern => id.includes(pattern));
          if (shouldExclude) return false;

          // Provider-specific filtering
          switch (provider) {
            case 'ollama':
            case 'custom':
              // Ollama and custom providers: accept all remaining models
              return true;

            case 'openai':
            case 'anthropic':
            case 'openrouter':
            case 'azure':
              // Known cloud providers: only include GPT models, o-series, Claude models
              return id.includes('gpt') ||
                     id.startsWith('o1') ||
                     id.startsWith('o3') ||
                     id.startsWith('o4') ||
                     id.includes('claude');

            default:
              // Unknown providers: accept all
              return true;
          }
        })
        .map((m: any) => {
          // Support both OpenAI format (m.id) and Ollama format (m.name, m.model)
          const modelId = m.id || m.name || m.model || 'unknown';

          return {
            id: modelId,
            name: this.formatModelName(modelId),
            provider,
            contextWindow: m.context_length || m.context_window || this.getDefaultContextWindow(modelId),
            description: m.description,
            supportsVision: this.inferVisionSupport(modelId),
            supportsTools: true,
            supportsTemperature: this.supportsTemperature(modelId),
            createdAt: m.created_at || m.modified_at,
          };
        });
    },

    /**
     * Format model name for display
     */
    formatModelName(id: string): string {
      // Replace dashes with spaces
      let formatted = id.replace(/-/g, ' ');

      // Preserve common acronyms in uppercase
      const acronyms = ['gpt', 'dall', 'tts', 'api', 'ai', 'ml'];
      acronyms.forEach(acronym => {
        const regex = new RegExp(`\\b${acronym}\\b`, 'gi');
        formatted = formatted.replace(regex, acronym.toUpperCase());
      });

      // Capitalize first letter of each word (if not already an acronym)
      formatted = formatted.replace(/\b\w/g, l => l.toUpperCase());

      return formatted;
    },

    /**
     * Get default context window for known models
     */
    getDefaultContextWindow(modelId: string): number {
      const id = modelId.toLowerCase();
      if (id.includes('gpt-4') || id.includes('gpt-5')) return 128000;
      if (id.includes('gpt-3.5')) return 16000;
      if (id.includes('claude-3')) return 200000;
      return 4096;
    },

    /**
     * Infer vision support from model ID
     */
    inferVisionSupport(modelId: string): boolean {
      const id = modelId.toLowerCase();
      return id.includes('vision') || id.includes('gpt-4');
    },

    /**
     * Check if model supports temperature parameter
     */
    supportsTemperature(modelId: string): boolean {
      const id = modelId.toLowerCase();
      return !(id.includes('gpt-5') || id.startsWith('o1') || id.startsWith('o3') || id.startsWith('o4'));
    },

    /**
     * Load cache from file
     */
    async loadCache(): Promise<ModelCache | null> {
      try {
        const cacheData = await invoke<string>('read_model_cache');
        return JSON.parse(cacheData);
      } catch {
        return null;
      }
    },

    /**
     * Save cache to file
     */
    async saveCache() {
      // Don't save if models object is empty or invalid
      if (!this.models || Object.keys(this.models).length === 0) {
        console.warn('⚠️ Skipping cache save - no models to cache');
        return;
      }

      // Check if at least one provider has models
      const hasAnyModels = Object.values(this.models).some(models => models.length > 0);
      if (!hasAnyModels) {
        console.warn('⚠️ Skipping cache save - all providers returned 0 models');
        return;
      }

      const cache: ModelCache = {
        lastUpdated: this.lastUpdated || new Date().toISOString(),
        providers: Object.fromEntries(
          Object.entries(this.models).map(([provider, models]) => [
            provider,
            {
              models,
              fetchedAt: new Date().toISOString(),
            },
          ])
        ),
      };

      await invoke('write_model_cache', {
        content: JSON.stringify(cache, null, 2)
      });
    },
  },

  // Enable Tauri sync for cross-window state synchronization
  // The @tauri-store/pinia plugin will handle automatic syncing across all windows
  // Must call store.$tauri.start() after creating the store to enable sync
});
