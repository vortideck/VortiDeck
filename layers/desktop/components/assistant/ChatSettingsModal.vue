<template lang="pug">
  Teleport(to="body")
    BaseModal(:is-open="isOpen" @close="closeModal")
      div(class="vd-modal-card w-full max-w-md")
        // Header
        div(class="flex items-center justify-between px-6 py-4 border-b border-themeBorder2")
          h2(class="text-lg font-semibold text-themeText") Chat Settings
          button(
            @click="closeModal"
            class="vd-modal-close-btn"
          )
            svg(class="w-5 h-5 text-themeText2" fill="none" stroke="currentColor" viewBox="0 0 24 24")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")

        // Body
        div(class="px-6 py-4 space-y-4 max-h-[60vh] overflow-y-auto")
          // Provider Selection
          div
            label(class="block text-sm font-medium text-themeText mb-2") AI Provider
            select(
              v-model="settings.provider"
              @change="onProviderChange"
              :disabled="providersWithModelCounts.length === 0"
              class="w-full px-3 py-2 bg-themeBackground border border-themeBorder2 rounded text-themeText text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
            )
              option(v-if="providersWithModelCounts.length === 0" value="") No providers configured - add in settings
              option(v-for="providerInfo in providersWithModelCounts" :key="providerInfo.provider" :value="providerInfo.provider")
                | {{ formatProviderName(providerInfo.provider) }} ({{ providerInfo.modelCount }} models)

          // Model Selection
          div
            label(class="block text-sm font-medium text-themeText mb-2") Model
            select(
              v-model="settings.model"
              :disabled="availableModels.length === 0"
              class="w-full px-3 py-2 bg-themeBackground border border-themeBorder2 rounded text-themeText text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
            )
              option(v-if="availableModels.length === 0" value="") No models available - refresh models in settings
              option(v-for="model in availableModels" :key="model.id" :value="model.id")
                template(v-if="model.description")
                  | {{ model.name }} - {{ model.description }}
                template(v-else)
                  | {{ model.name }}

          // Local AI Discovery & Test Connection
          div(class="p-3 bg-themeBackground2/70 border border-themeBorder2 rounded-lg space-y-2.5")
            div(class="flex items-center justify-between")
              div(class="flex items-center gap-2")
                span(class="text-sm") 💻
                span(class="text-xs font-semibold text-themeText") Local AI & Offline Engine
              span(class="text-[10px] text-themeText2 font-mono") Ollama / LM Studio
            
            div(class="flex items-center gap-2")
              button(
                type="button"
                @click="testLocalAIConnection('ollama')"
                :disabled="isTestingLocalAI"
                class="flex-1 py-1.5 px-2 rounded bg-themeBackground hover:bg-themeBackground3 border border-themeBorder2 text-xs text-themeText flex items-center justify-center gap-1.5 transition-all cursor-pointer disabled:opacity-50"
              )
                span 🦙
                span {{ isTestingLocalAI ? 'Checking...' : 'Test Ollama (11434)' }}
              button(
                type="button"
                @click="testLocalAIConnection('lmstudio')"
                :disabled="isTestingLocalAI"
                class="flex-1 py-1.5 px-2 rounded bg-themeBackground hover:bg-themeBackground3 border border-themeBorder2 text-xs text-themeText flex items-center justify-center gap-1.5 transition-all cursor-pointer disabled:opacity-50"
              )
                span 🧪
                span {{ isTestingLocalAI ? 'Checking...' : 'Test LM Studio (1234)' }}

            // Test Status / Model Discovery Banner
            div(v-if="localAITestStatus" :class="localAITestStatus.success ? 'bg-emerald-500/10 border-emerald-500/30 text-emerald-300' : 'bg-red-500/10 border-red-500/30 text-red-300'" class="p-2 rounded border text-xs flex items-start justify-between gap-2")
              div(class="space-y-1 flex-1")
                div(class="font-medium flex items-center gap-1.5")
                  span {{ localAITestStatus.success ? '✅' : '❌' }}
                  span {{ localAITestStatus.message }}
                div(v-if="localAITestStatus.models && localAITestStatus.models.length > 0" class="text-[11px] opacity-90")
                  span Discovered {{ localAITestStatus.models.length }} models: {{ localAITestStatus.models.slice(0, 4).map(m => m.name).join(', ') }}{{ localAITestStatus.models.length > 4 ? '...' : '' }}
              button(@click="localAITestStatus = null" class="opacity-60 hover:opacity-100 text-xs cursor-pointer") ✕

          // Temperature
          div
            label(class="block text-sm font-medium text-themeText mb-2")
              | Temperature
              span(class="text-themeText2 font-normal") ({{ settings.temperature.toFixed(1) }})
            input(
              v-model.number="settings.temperature"
              type="range"
              min="0"
              max="2"
              step="0.1"
              class="w-full accent-blue-500"
            )
            div(class="flex justify-between text-xs text-themeText2 mt-1")
              span Precise
              span Creative

          // Max Tokens
          div
            label(class="block text-sm font-medium text-themeText mb-2") Max Tokens
            input(
              v-model.number="settings.maxTokens"
              type="number"
              min="100"
              max="128000"
              step="100"
              class="w-full px-3 py-2 bg-themeBackground border border-themeBorder2 rounded text-themeText text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
            )

          // Tool Timeout
          div
            label(class="block text-sm font-medium text-themeText mb-2")
              | Tool Execution Timeout
              span(class="text-themeText2 font-normal text-xs ml-2") ({{ settings.toolTimeout }}s)
            div(class="flex gap-2 items-center")
              input(
                v-model.number="settings.toolTimeout"
                type="range"
                min="30"
                max="1800"
                step="30"
                class="flex-1 accent-blue-500"
              )
              input(
                v-model.number="settings.toolTimeout"
                type="number"
                min="30"
                max="1800"
                step="30"
                class="w-20 px-2 py-1 bg-themeBackground border border-themeBorder2 rounded text-themeText text-xs focus:outline-none focus:ring-2 focus:ring-blue-500"
              )
            div(class="flex justify-between text-xs text-themeText2 mt-1")
              span 30s
              span 30 minutes

          // Streaming Mode
          div
            label(class="flex items-center gap-3 cursor-pointer")
              input(
                v-model="settings.stream"
                type="checkbox"
                class="w-4 h-4 accent-blue-500"
              )
              div
                div(class="text-sm font-medium text-themeText") Enable Streaming
                div(class="text-xs text-themeText2") Stream responses from API (requires verified organization for GPT-5)

          // System Prompt
          div
            label(class="block text-sm font-medium text-themeText mb-2")
              | System Prompt
              span(class="text-themeText2 font-normal text-xs ml-2") (Optional - customize AI behavior)
            textarea(
              v-model="settings.systemPrompt"
              rows="3"
              placeholder="You are a helpful AI assistant..."
              class="w-full px-3 py-2 bg-themeBackground border border-themeBorder2 rounded text-themeText text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
            )
            div(class="text-xs text-themeText2 mt-1")
              | Leave empty to use default. Guides AI behavior, not tool descriptions.

          // MCP Tools
          div
            label(class="block text-sm font-medium text-themeText mb-3") MCP Tools
            div(class="space-y-2")
              // All Tools
              label(class="flex items-center gap-2 cursor-pointer")
                input(
                  v-model="settings.toolsMode"
                  type="radio"
                  value="all"
                  class="accent-blue-500"
                )
                span(class="text-sm text-themeText") All Tools
                  span(class="text-themeText2") ({{ availableTools.length }} available)
                  span(v-if="hasToolLimit && availableTools.length > 128" class="text-yellow-500 ml-1") ⚠ Exceeds API limit (128)

              // Selected Tools
              label(class="flex items-center gap-2 cursor-pointer")
                input(
                  v-model="settings.toolsMode"
                  type="radio"
                  value="selected"
                  class="accent-blue-500"
                )
                span(class="text-sm text-themeText") Selected Tools
                  span(v-if="settings.selectedTools.length > 0" class="text-themeText2") ({{ settings.selectedTools.length }} selected)
                  span(v-if="hasToolLimit && settings.selectedTools.length > 128" class="text-yellow-500 ml-1") ⚠ Exceeds API limit (128)

              // Tool selection (if selected mode)
              div(v-if="settings.toolsMode === 'selected'" class="ml-6 space-y-2")
                // Search input
                div(class="relative")
                  input(
                    v-model="toolSearchQuery"
                    type="text"
                    placeholder="Search tools..."
                    class="w-full px-3 py-2 pl-9 pr-8 bg-themeBackground border border-themeBorder2 rounded text-themeText text-xs focus:outline-none focus:ring-2 focus:ring-blue-500"
                  )
                  svg(class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-themeText2" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                    path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z")
                  button(
                    v-if="toolSearchQuery"
                    @click="toolSearchQuery = ''"
                    class="absolute right-2 top-1/2 -translate-y-1/2 p-1 hover:bg-themeBackground3 rounded transition-colors"
                  )
                    svg(class="w-3 h-3 text-themeText2" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                      path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")

                // Quick selection buttons
                div(class="flex items-center justify-between text-xs text-themeText2 pt-1")
                  span {{ settings.selectedTools.length }} of {{ availableTools.length }} selected
                  div(class="flex gap-2")
                    button(
                      type="button"
                      @click="selectAllTools"
                      class="hover:text-themeText hover:underline"
                    ) Select All
                    button(
                      type="button"
                      @click="deselectAllTools"
                      class="hover:text-themeText hover:underline"
                    ) Deselect All

                // Tool list
                div(class="max-h-48 overflow-y-auto space-y-1 pr-1")
                  div(
                    v-for="tool in filteredTools"
                    :key="tool.name"
                    class="p-2 hover:bg-themeBackground2 rounded flex items-start gap-2 cursor-pointer border border-transparent hover:border-themeBorder2 transition-colors"
                    @click="toggleTool(tool.name)"
                  )
                    input(
                      type="checkbox"
                      :checked="settings.selectedTools.includes(tool.name)"
                      @click.stop="toggleTool(tool.name)"
                      class="mt-0.5 accent-blue-500"
                    )
                    div(class="flex-1 min-w-0")
                      div(class="text-xs font-mono font-medium text-themeText truncate") {{ tool.name }}
                      div(class="text-xs text-themeText2 line-clamp-2") {{ tool.description }}

              // No Tools
              label(class="flex items-center gap-2 cursor-pointer")
                input(
                  v-model="settings.toolsMode"
                  type="radio"
                  value="none"
                  class="accent-blue-500"
                )
                span(class="text-sm text-themeText") No Tools (Chat Only)

        // Footer
        div(class="flex items-center justify-between px-6 py-4 border-t border-themeBorder2")
          button(
            @click="saveAsDefault"
            class="vd-modal-btn px-4 py-2 text-sm text-themeText2 hover:text-themeText"
          ) Save as Default
          div(class="flex gap-2")
            button(
              @click="closeModal"
              class="vd-modal-btn px-4 py-2 text-sm"
            ) Cancel
            button(
              @click="applySettings"
              class="vd-modal-primary-btn px-4 py-2 text-sm"
            ) Apply
</template>

<script setup lang="ts">
import BaseModal from "../Modal/BaseModal.vue";
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { readTextFile, BaseDirectory } from "@tauri-apps/plugin-fs";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { fetch as tauriFetch } from "@tauri-apps/plugin-http";
import type { AIModel } from "@/stores/AIModels";
import { useAIModelsStore } from "@/stores/AIModels";

interface ChatSettings {
  provider: string;
  model: string;
  temperature: number;
  maxTokens: number;
  toolsMode: "all" | "selected" | "none";
  selectedTools: string[];
  stream: boolean;
  systemPrompt: string;
  toolTimeout: number;
}

interface ToolInfo {
  name: string;
  description: string;
}

const props = defineProps<{
  modelValue: boolean;
  currentSettings?: Partial<ChatSettings>;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: boolean];
  apply: [settings: ChatSettings];
  saveDefault: [settings: ChatSettings];
}>();

const isOpen = computed({
  get: () => props.modelValue,
  set: (value) => emit("update:modelValue", value),
});

const settings = ref<ChatSettings>({
  provider: "openai",
  model: "gpt-4",
  temperature: 0.7,
  maxTokens: 4000,
  toolsMode: "all",
  selectedTools: [],
  stream: false,
  systemPrompt: "",
  toolTimeout: 30,
});

const availableProviders = ref<Array<{ provider: string; name: string; modelCount: number }>>([]);
const availableTools = ref<ToolInfo[]>([]);
const toolSearchQuery = ref("");

// Local AI test state
const isTestingLocalAI = ref(false);
const localAITestStatus = ref<{
  success: boolean;
  message: string;
  models?: Array<{ id: string; name: string }>;
} | null>(null);

// Use centralized AI models store
const aiModelsStore = useAIModelsStore();

// Computed available models (reactive to store changes)
const availableModels = computed(() => {
  const models = aiModelsStore.getModelsByProvider(settings.value.provider);
  return models;
});

// Format provider name for display
const formatProviderName = (provider: string) => {
  const names: Record<string, string> = {
    openai: "OpenAI",
    anthropic: "Anthropic",
    openrouter: "OpenRouter",
    azure: "Azure",
    custom: "Custom / Local",
    ollama: "Ollama (Local)",
  };
  return names[provider] || provider.charAt(0).toUpperCase() + provider.slice(1);
};

// 1-Click Local Model Testing & Discovery
const testLocalAIConnection = async (type: "ollama" | "lmstudio") => {
  isTestingLocalAI.value = true;
  localAITestStatus.value = null;

  try {
    if (type === "ollama") {
      const res = await tauriFetch("http://localhost:11434/api/tags", {
        method: "GET",
        connectTimeout: 4000,
      });

      if (!res.ok) {
        throw new Error(`Ollama returned status ${res.status}`);
      }

      const data = (await res.json()) as any;
      const rawModels = data.models || [];
      const discoveredModels = rawModels.map((m: any) => ({
        id: m.name || m.model,
        name: m.name || m.model,
      }));

      // Add ollama to available providers if missing
      if (!availableProviders.value.some(p => p.provider === "ollama")) {
        availableProviders.value.push({
          provider: "ollama",
          name: "Ollama (Local)",
          modelCount: discoveredModels.length,
        });
      }

      // Update AI models store for ollama
      aiModelsStore.models["ollama"] = discoveredModels.map((m: any) => ({
        id: m.id,
        name: m.name,
        provider: "ollama",
        contextWindow: 8192,
        supportsTools: true,
        supportsTemperature: true,
      }));

      settings.value.provider = "ollama";
      if (discoveredModels.length > 0) {
        settings.value.model = discoveredModels[0].id;
      }

      localAITestStatus.value = {
        success: true,
        message: `Connected to Ollama! Found ${discoveredModels.length} models.`,
        models: discoveredModels,
      };
    } else {
      // LM Studio on 1234
      const res = await tauriFetch("http://localhost:1234/v1/models", {
        method: "GET",
        connectTimeout: 4000,
      });

      if (!res.ok) {
        throw new Error(`LM Studio returned status ${res.status}`);
      }

      const data = (await res.json()) as any;
      const rawModels = data.data || [];
      const discoveredModels = rawModels.map((m: any) => ({
        id: m.id,
        name: m.id,
      }));

      // Add custom to available providers if missing
      if (!availableProviders.value.some(p => p.provider === "custom")) {
        availableProviders.value.push({
          provider: "custom",
          name: "LM Studio / Local",
          modelCount: discoveredModels.length,
        });
      }

      aiModelsStore.models["custom"] = discoveredModels.map((m: any) => ({
        id: m.id,
        name: m.name,
        provider: "custom",
        contextWindow: 8192,
        supportsTools: true,
        supportsTemperature: true,
      }));

      settings.value.provider = "custom";
      if (discoveredModels.length > 0) {
        settings.value.model = discoveredModels[0].id;
      }

      localAITestStatus.value = {
        success: true,
        message: `Connected to LM Studio! Found ${discoveredModels.length} models.`,
        models: discoveredModels,
      };
    }
  } catch (err: any) {
    localAITestStatus.value = {
      success: false,
      message: `Could not connect to ${type === "ollama" ? "Ollama (port 11434)" : "LM Studio (port 1234)"}. Ensure the local server is running.`,
    };
  } finally {
    isTestingLocalAI.value = false;
  }
};

// Load registered providers from MCP config
const loadRegisteredProviders = async () => {
  try {
    const configContent = await readTextFile("resources/mcp/config.json", {
      baseDir: BaseDirectory.AppConfig,
    });
    const config = JSON.parse(configContent);

    if (config?.models?.model) {
      availableProviders.value = config.models.model
        .filter((m: any) => m.provider)
        .map((m: any) => ({
          provider: m.provider,
          name: m.name || formatProviderName(m.provider),
          modelCount: 0,
        }));
    }
  } catch (error) {
    console.error("❌ Failed to load providers:", error);
    availableProviders.value = [];
  }
};

// Computed providers with live model counts
const providersWithModelCounts = computed(() => {
  return availableProviders.value.map((p) => {
    const modelCount = aiModelsStore.getModelsByProvider(p.provider).length;
    return {
      ...p,
      modelCount,
    };
  });
});

// Filtered tools based on search query
const filteredTools = computed(() => {
  if (!toolSearchQuery.value.trim()) {
    return availableTools.value;
  }

  const searchTerms = toolSearchQuery.value
    .toLowerCase()
    .split(/\s+/)
    .filter((term) => term.length > 0);

  return availableTools.value.filter((tool) => {
    const nameMatch = searchTerms.every((term) =>
      tool.name.toLowerCase().includes(term)
    );
    const descMatch = searchTerms.every((term) =>
      tool.description.toLowerCase().includes(term)
    );
    return nameMatch || descMatch;
  });
});

// Check if current provider has tool limit
const hasToolLimit = computed(() => {
  return ["openai", "azure"].includes(settings.value.provider);
});

// Load available MCP tools from backend
const loadAvailableTools = async () => {
  try {
    const tools = await invoke<ToolInfo[]>("get_available_mcp_tools");
    availableTools.value = tools;
  } catch (error) {
    console.error("❌ Failed to load MCP tools:", error);
    availableTools.value = [];
  }
};

// Tool selection handlers
const toggleTool = (toolName: string) => {
  const index = settings.value.selectedTools.indexOf(toolName);
  if (index === -1) {
    settings.value.selectedTools.push(toolName);
  } else {
    settings.value.selectedTools.splice(index, 1);
  }
};

const selectAllTools = () => {
  settings.value.selectedTools = availableTools.value.map((t) => t.name);
};

const deselectAllTools = () => {
  settings.value.selectedTools = [];
};

const onProviderChange = () => {
  const models = availableModels.value;
  if (models.length > 0) {
    settings.value.model = models[0].id;
  }
};

const applySettings = () => {
  emit("apply", { ...settings.value });
  closeModal();
};

const saveAsDefault = () => {
  emit("saveDefault", { ...settings.value });
  emit("apply", { ...settings.value });
  closeModal();
};

const closeModal = () => {
  isOpen.value = false;
};

// Initialize settings from props
watch(
  () => props.currentSettings,
  (newSettings) => {
    if (newSettings) {
      settings.value = { ...settings.value, ...newSettings };
    }
  },
  { immediate: true }
);

onMounted(async () => {
  await loadRegisteredProviders();
  await loadAvailableTools();
});
</script>
