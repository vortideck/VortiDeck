<template lang="pug">
  div(:class="['h-screen flex flex-col bg-themeBackground text-themeText overflow-hidden select-none', themeHtmlClass]")
   

    // Main chat area
    div(class="flex-1 flex flex-col min-h-0")
      // Header with drag region
      div(data-tauri-drag-region class="bg-gradient-to-r cursor-move from-blue-500/10 to-purple-500/10 border-b border-themeBorder2 px-3 py-2.5 flex items-center justify-between select-none flex-shrink-0")
        // Left branding
        div(data-tauri-drag-region class="flex items-center gap-2 cursor-move pointer-events-none")
          svg(class="w-5 h-5 text-blue-400 shrink-0" viewBox="0 0 24 24" fill="currentColor")
            path(d="M12 2L14.5 9.5L22 12L14.5 14.5L12 22L9.5 14.5L2 12L9.5 9.5L12 2Z")
          h3(class="text-sm font-semibold text-themeText") Studio Copilot
          span(class="text-[10px] font-mono font-medium px-2 py-0.5 rounded-full bg-themeBackground2 text-themeText2 border border-themeBorder") Local assistant
          span(v-if="contextInfo" class="text-xs text-themeText2 px-2 py-0.5 bg-themeBackground2 rounded") {{ contextInfo }}

        // Right controls
        div(class="flex items-center gap-1 shrink-0")
          // New Window button (Parallel Copilot)
          button(
            @click="openNewAssistantWindow"
            class="p-1.5 rounded-lg border border-transparent text-themeText2 hover:text-themeText hover:bg-themeBackground2 transition-colors cursor-pointer flex items-center justify-center"
            title="Open New Assistant Window (Parallel Copilot)"
          )
            svg(class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M19 11V19a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2h8m4-2h5m-2.5-2.5v5")

          // Pin button (Always on top)
          button(
            @click="togglePin"
            :class="isPinned ? 'bg-amber-500 text-white shadow-sm ring-1 ring-amber-300' : 'text-themeText2 hover:text-themeText hover:bg-themeBackground2 border-transparent'"
            class="p-1.5 rounded-lg border transition-all cursor-pointer flex items-center justify-center"
            :title="isPinned ? 'Always on Top: Active (Floating over all windows)' : 'Always on Top: Inactive (Click to float over all windows)'"
          )
            svg(class="w-4 h-4" viewBox="0 0 24 24" fill="currentColor")
              path(d="M16 12V4h1V2H7v2h1v8l-2 2v2h5v6l1 1l1-1v-6h5v-2l-2-2z")

          // Close button
          button(
            @click="closeWindow"
            class="p-1.5 hover:bg-red-500/20 text-themeText2 hover:text-red-400 rounded-lg transition-colors cursor-pointer flex items-center justify-center"
            title="Close"
          )
            svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")

      // Conversation sidebar
      AssistantConversationHistory(
        v-model:current-id="currentConversationId"
        @select="loadConversation"
        @create="createNewConversation"
        @open-settings="showSettingsModal = true"
        ref="conversationHistory"
      )

      // Chat Settings Modal
      AssistantChatSettingsModal(
        v-model="showSettingsModal"
        :current-settings="currentChatSettings"
        @apply="applyChatSettings"
        @save-default="saveDefaultChatSettings"
      )
      // Chat Messages Area
      div(
        ref="messagesContainer"
        class="flex-1 min-h-0 px-4 py-4 overflow-y-auto scrollbar-visible"
      )
        div(class="space-y-2")
          // Sleek Minimalist Welcome & Quick Prompts
          div(v-if="messages.length === 0" class="py-8 px-3 text-center space-y-4 flex flex-col items-center justify-center")
            div(class="w-10 h-10 rounded-xl bg-blue-500/10 border border-blue-500/20 flex items-center justify-center text-blue-400")
              svg(class="w-5 h-5" viewBox="0 0 24 24" fill="currentColor")
                path(d="M12 2L14.5 9.5L22 12L14.5 14.5L12 22L9.5 14.5L2 12L9.5 9.5L12 2Z")
            
            div(class="space-y-1")
              h4(class="text-sm font-semibold text-themeText") Studio Copilot
              p(class="text-xs text-themeText2 max-w-[240px]") Real-time assistant for workflows, OBS, and stream automations.

            // Sleek 2x2 Quick Chips
            div(class="grid grid-cols-2 gap-2 w-full max-w-xs pt-1")
              button(
                @click="quickPrompt('List all active flows and their trigger configurations')"
                class="flex items-center gap-2 px-2.5 py-2 rounded-lg bg-themeBackground2 hover:bg-themeBackground3 border border-themeBorder2 text-left transition-all group"
              )
                span(class="text-xs") ⚡
                span(class="text-[11px] font-medium text-themeText2 group-hover:text-themeText truncate") Active Flows

              button(
                @click="quickPrompt('Switch OBS to BRB Scene and mute desktop audio')"
                class="flex items-center gap-2 px-2.5 py-2 rounded-lg bg-themeBackground2 hover:bg-themeBackground3 border border-themeBorder2 text-left transition-all group"
              )
                span(class="text-xs") 🎛️
                span(class="text-[11px] font-medium text-themeText2 group-hover:text-themeText truncate") OBS Control

              button(
                @click="quickPrompt('Show recent stream and webhook events from the SQLite inbox')"
                class="flex items-center gap-2 px-2.5 py-2 rounded-lg bg-themeBackground2 hover:bg-themeBackground3 border border-themeBorder2 text-left transition-all group"
              )
                span(class="text-xs") 🔍
                span(class="text-[11px] font-medium text-themeText2 group-hover:text-themeText truncate") Event Logs

              button(
                @click="quickPrompt('Run my celebration macro')"
                class="flex items-center gap-2 px-2.5 py-2 rounded-lg bg-themeBackground2 hover:bg-themeBackground3 border border-themeBorder2 text-left transition-all group"
              )
                span(class="text-xs") 🚀
                span(class="text-[11px] font-medium text-themeText2 group-hover:text-themeText truncate") Run Macro

          // Messages
          div(
            v-for="(message, index) in messages"
            :key="index"
            class="flex flex-col "
            :class="message.role === 'user' ? 'items-end' : 'items-start'"
          )
            // Model info header (only for assistant messages)
            div(
              v-if="message.role === 'assistant' && message.model"
              class="text-xs text-gray-500 dark:text-gray-400 mb-1 px-1 font-mono flex items-center gap-2"
            )
              span {{ message.provider }} • {{ message.model }}
              span(v-if="message.usage" class="text-themeText3") |
              span(
                v-if="message.usage?.evidence === 'provider_measured'"
                class="text-themeText3"
              )
                | {{ message.usage.prompt_tokens }}→{{ message.usage.completion_tokens }} measured tokens
              span(
                v-else-if="message.usage?.evidence === 'partial'"
                class="text-themeText3"
              )
                | {{ message.usage.prompt_tokens }}→{{ message.usage.completion_tokens }} partially measured
              span(v-else-if="message.usage" class="text-themeText3") usage unavailable

            div(
              v-if="message.content || message.role === 'user'"
              :class="message.role === 'user' ? 'bg-blue-500 text-white' : 'bg-themeBackground2 text-themeText'"
              class="max-w-[85%] px-4 py-2 rounded-lg select-text w-fit"
            )
              // Message content (use cached formatted content for performance)
              div(class="text-sm select-text break-words leading-relaxed prose prose-sm prose-invert max-w-none" v-html="message.formattedContent || message.content")

              // Interactive Collapsible Tool Execution Badges
              div(v-if="message.toolExecutions && message.toolExecutions.length > 0" class="mt-2 space-y-1.5")
                div(
                  v-for="(tool, tIdx) in message.toolExecutions"
                  :key="tIdx"
                  class="border border-themeBorder2/70 bg-black/20 rounded-lg p-2 text-xs font-mono transition-all"
                )
                  div(
                    @click="tool.expanded = !tool.expanded"
                    class="flex items-center justify-between cursor-pointer gap-2 select-none"
                  )
                    div(class="flex items-center gap-1.5 truncate")
                      span(class="text-blue-400 font-semibold") ⚙️ {{ tool.name }}
                      span(v-if="tool.summary" class="text-themeText2 text-[11px] truncate max-w-[140px]") ({{ tool.summary }})
                    div(class="flex items-center gap-1.5 shrink-0")
                      span(v-if="tool.durationMs" class="text-[10px] text-themeText2") {{ tool.durationMs }}ms
                      span(class="px-1.5 py-0.5 rounded text-[10px] font-semibold" :class="tool.status === 'success' ? 'bg-emerald-500/20 text-emerald-400' : tool.status === 'error' ? 'bg-red-500/20 text-red-400' : 'bg-blue-500/20 text-blue-400 animate-pulse'")
                        | {{ tool.status === 'success' ? '✅ Done' : tool.status === 'error' ? '❌ Failed' : '⏳ Running' }}
                      svg(class="w-3.5 h-3.5 text-themeText2 transition-transform" :class="tool.expanded ? 'rotate-180' : ''" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                        path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7")

                  // Collapsible tool payload inspection
                  div(v-if="tool.expanded" class="mt-2 pt-2 border-t border-themeBorder2/40 space-y-1.5 text-[11px]")
                    div(v-if="tool.input")
                      span(class="text-themeText2 font-semibold block text-[10px] uppercase tracking-wider") Input Arguments:
                      pre(class="p-1.5 bg-black/40 rounded text-neutral-300 overflow-x-auto text-[10px] max-h-32") {{ JSON.stringify(tool.input, null, 2) }}
                    div(v-if="tool.output")
                      span(class="text-themeText2 font-semibold block text-[10px] uppercase tracking-wider") Output Result:
                      pre(class="p-1.5 bg-black/40 rounded text-emerald-300/90 overflow-x-auto text-[10px] max-h-32") {{ typeof tool.output === 'string' ? tool.output : JSON.stringify(tool.output, null, 2) }}

              // Quick action buttons for AI suggestions
              div(
                v-if="message.role === 'assistant' && message.suggestions && message.suggestions.length > 0"
                class="mt-2 flex flex-wrap gap-2"
              )
                button(
                  v-for="(suggestion, idx) in message.suggestions"
                  :key="idx"
                  @click="applySuggestion(suggestion)"
                  class="text-xs px-2 py-1 bg-blue-500/20 hover:bg-blue-500/30 rounded border border-blue-500/30 transition-colors"
                )
                  | {{ suggestion.label }}

      // MCP Tool Progress Indicator
      div(v-if="toolProgress.active" class="px-4 py-3 border-t border-themeBorder2 bg-themeBackground2/50 flex-shrink-0")
        div(class="flex items-center gap-3")
          // Progress spinner
          div(class="flex-shrink-0")
            svg(class="w-5 h-5 text-blue-400 animate-spin" fill="none" viewBox="0 0 24 24")
              circle(class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4")
              path(class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z")

          // Progress content
          div(class="flex-1 min-w-0")
            div(class="flex items-center justify-between mb-1")
              span(class="text-sm font-medium text-themeText") Running MCP Tool
              span(class="text-xs text-themeText2") {{ Math.round(toolProgress.percentage) }}%

            // Progress bar
            div(class="w-full bg-themeBackground3 rounded-full h-2 overflow-hidden")
              div(
                class="h-full bg-gradient-to-r from-blue-500 to-purple-500 transition-all duration-300 ease-out"
                :style="{ width: toolProgress.percentage + '%' }"
              )

            // Progress message
            div(v-if="toolProgress.message" class="text-xs text-themeText2 mt-1") {{ toolProgress.message }}

      // Active Studio Auto-Context Banner (Above Chat Input)
      div(v-if="activeContext" class="px-4 py-1.5 bg-blue-500/10 border-t border-blue-500/25 flex items-center justify-between gap-2 flex-shrink-0 text-xs")
        div(class="flex items-center gap-1.5 truncate")
          span(class="text-blue-400 font-semibold shrink-0") 🎯 Studio Context:
          span(class="text-themeText truncate font-medium") {{ activeContext.title }}
          span(v-if="activeContext.subtitle" class="text-themeText2 text-[11px] truncate") • {{ activeContext.subtitle }}
        div(class="flex items-center gap-1.5 shrink-0")
          // Context History Dropdown
          div(v-if="contextHistory.length > 1" class="relative group")
            button(class="px-1.5 py-0.5 rounded text-[10px] bg-themeBackground2 hover:bg-themeBackground3 border border-themeBorder2 text-themeText2 hover:text-themeText cursor-pointer")
              | ▾ {{ contextHistory.length }} recent
            div(class="absolute right-0 bottom-full mb-1 w-56 p-1 bg-themeBackground border border-themeBorder2 rounded-lg shadow-xl hidden group-hover:block z-50 space-y-1")
              div(
                v-for="item in contextHistory"
                :key="item.id"
                @click="selectFromHistory(item)"
                class="p-1.5 rounded hover:bg-themeBackground2 text-[11px] cursor-pointer truncate text-themeText"
                :class="item.id === activeContext.id ? 'font-semibold text-blue-400' : ''"
              )
                | {{ item.title }}
          button(@click="clearContext" class="text-themeText2 hover:text-red-400 p-0.5 rounded transition-colors cursor-pointer" title="Dismiss Context") ✕

      // Loading indicator
      div(v-if="isLoading" class="px-4 py-2 flex items-center gap-2 text-themeText2 text-sm flex-shrink-0 border-t border-themeBorder2")
        div(v-if="!isStreaming" class="flex items-center gap-1")
          div(class="w-2 h-2 bg-blue-400 rounded-full animate-bounce" style="animation-delay: 0ms")
          div(class="w-2 h-2 bg-blue-400 rounded-full animate-bounce" style="animation-delay: 150ms")
          div(class="w-2 h-2 bg-blue-400 rounded-full animate-bounce" style="animation-delay: 300ms")
        span {{ isStreaming ? 'AI is responding...' : 'Processing...' }}

      // Context-Aware Dynamic Suggestion Chips
      div(class="px-3 py-1.5 bg-themeBackground2/70 border-t border-themeBorder2 flex items-center gap-1.5 overflow-x-auto flex-shrink-0 custom-scrollbar-hidden")
        span(class="text-[10px] uppercase font-bold text-themeText3 shrink-0 flex items-center gap-1 tracking-wider pl-1")
          span 💡
          span Suggestions:
        button(
          v-for="(chip, cIdx) in contextSuggestions"
          :key="cIdx"
          @click="applyQuickPrompt(chip.prompt)"
          class="shrink-0 px-2.5 py-1 rounded-full text-[11px] font-medium bg-themeBackground hover:bg-themeBackground3 border border-themeBorder2/80 hover:border-blue-500/50 text-themeText hover:text-blue-400 flex items-center gap-1.5 transition-all cursor-pointer shadow-2xs group"
        )
          span(class="text-xs group-hover:scale-110 transition-transform") {{ chip.icon }}
          span {{ chip.label }}

      // Input Area
      div(class="border-t border-themeBorder2 px-4 py-3 flex-shrink-0")
        div(class="flex gap-2")
          input(
            v-model="userInput"
            @keydown.enter="sendMessage"
            @keydown.esc="closeWindow"
            placeholder="Ask me anything..."
            class="flex-1 px-3 py-2 bg-themeBackground2 border border-themeBorder2 rounded-lg text-themeText text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 transition-all"
            ref="chatInput"
            autofocus
          )
          // Stop button (when loading)
          button(
            v-if="isLoading"
            @click="stopGeneration"
            class="px-4 py-2 bg-red-500 hover:bg-red-600 text-white rounded-lg transition-colors flex items-center justify-center"
            title="Stop generation"
          )
            svg(class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20")
              rect(x="4" y="4" width="12" height="12" rx="1")

          // Send button (when not loading)
          button(
            v-else
            @click="sendMessage"
            :disabled="!userInput.trim()"
            class="px-4 py-2 bg-blue-500 hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-lg transition-colors flex items-center gap-2"
          )
            svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8")

      // Quick Tips (collapsible)
      div(v-if="showTips" class="border-t border-themeBorder2 px-4 py-2 bg-themeBackground3/50 flex-shrink-0")
        button(
          @click="tipsExpanded = !tipsExpanded"
          class="w-full flex items-center justify-between text-xs text-themeText2 hover:text-themeText transition-colors"
        )
          span ✨ Quick Tips
          svg(
            class="w-3 h-3 transition-transform"
            :class="{ 'rotate-180': tipsExpanded }"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          )
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7")

        Transition(
          enter-active-class="transition-all duration-200"
          enter-from-class="max-h-0 opacity-0"
          enter-to-class="max-h-40 opacity-100"
          leave-active-class="transition-all duration-150"
          leave-from-class="max-h-40 opacity-100"
          leave-to-class="max-h-0 opacity-0"
        )
          div(v-if="tipsExpanded" class="mt-2 space-y-1 text-xs text-themeText2 overflow-hidden")
            div(v-for="tip in quickTips" :key="tip" class="flex items-start gap-2")
              span(class="text-blue-400") •
              span {{ tip }}
  </template>

<script setup lang="ts">
definePageMeta({
  layout: false,
});

import { ref, computed, onMounted, onUnmounted, nextTick, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import {
  isCurrentAssistantTaskChunk,
  type AssistantTaskChunk,
} from "@desktop/utils/assistantTaskStream";
import { useAIModelsStore } from "@/stores/AIModels";
import { useMainStore } from "@/stores/Main";
import { useAppAutoContext } from "../composables/useAppAutoContext";
import { useContextSuggestions } from "../composables/useContextSuggestions";
import AssistantConversationHistory from "../components/assistant/ConversationHistory.vue";
import { marked } from "marked";

// Get window label to identify which parent called us
const currentWindow = getCurrentWindow();
const route = useRoute();
const mainStore = useMainStore();
const {
  activeContext,
  contextHistory,
  clearContext,
  selectFromHistory,
  initContextListener,
  disposeContextListener,
} = useAppAutoContext();

// Bulletproof reactive theme binding on HTML & Body
const colorMode = computed(() => (mainStore.colorMode || (typeof window !== 'undefined' ? localStorage.getItem('vortideck_color_mode') : 'dark') || 'dark') as string);
const themePalette = computed(() => (mainStore.themePalette || (typeof window !== 'undefined' ? localStorage.getItem('vortideck_theme_palette') : 'vortideck') || 'vortideck') as string);
const themeHtmlClass = computed(() => {
  const mode = colorMode.value === 'light' ? 'light' : 'dark';
  const pal = themePalette.value.toLowerCase();
  return `${mode} theme-${pal}-${mode} theme-vortideck-${mode} theme-default-${mode}`;
});

useHead({
  htmlAttrs: {
    class: themeHtmlClass,
  },
  bodyAttrs: {
    class: 'bg-themeBackground text-themeText min-h-screen overflow-hidden',
  },
});

// State
const isLoading = ref(false);
const isStreaming = ref(false);
const streamingMessageIndex = ref<number | null>(null);
const activeTaskId = ref<string | null>(null);
const activeStreamSequence = ref(0);
const userInput = ref("");

// MCP Tool Progress Tracking
const toolProgress = ref<{
  active: boolean;
  token: string;
  progress: number;
  total: number | null;
  percentage: number;
  message: string;
}>({
  active: false,
  token: "",
  progress: 0,
  total: null,
  percentage: 0,
  message: "",
});

export interface MessageToolExecution {
  name: string;
  summary?: string;
  status: "running" | "success" | "error";
  durationMs?: number;
  input?: any;
  output?: any;
  expanded?: boolean;
}

const messages = ref<
  Array<{
    role: "user" | "assistant";
    content: string;
    formattedContent?: string; // Cache formatted content for performance
    suggestions?: Array<{ label: string; value: string; action?: string }>;
    toolExecutions?: MessageToolExecution[];
    model?: string;
    provider?: string;
    usage?: {
      prompt_tokens: number;
      completion_tokens: number;
      total_tokens: number;
      evidence: "provider_measured" | "partial" | "unknown";
    };
  }>
>([]);

const tipsExpanded = ref(false);
const chatInput = ref<HTMLInputElement>();
const messagesContainer = ref<HTMLDivElement>();

// Conversation management
const currentConversationId = ref<string | null>(null);
const conversationHistory = ref<InstanceType<typeof AssistantConversationHistory>>();

// Chat settings
const showSettingsModal = ref(false);

// Use centralized AI models store (Pinia)
const aiModelsStore = useAIModelsStore();

const currentChatSettings = ref({
  provider: "openai",
  model: "gpt-4", // Default fallback - will be updated when models load or user changes settings
  temperature: 0.7,
  maxTokens: 4000,
  toolsMode: "all" as "all" | "selected" | "none",
  selectedTools: [] as string[],
  stream: false, // Default streaming disabled
  systemPrompt: "", // Empty = use default system prompt
});

// Context from parent window
const context = ref<string>("");
const contextInfo = ref<string>("");
const currentInputValue = ref<string>("");
const formData = ref<Record<string, any>>({});
const showTips = ref(true);

// Computed
const quickTips = computed(() => {
  // Context-specific tips
  const contextTips: Record<string, string[]> = {
    command_input: [
      'Try: "Suggest a command to list files"',
      'Ask: "What command opens VS Code?"',
      'Say: "Help me create a git commit"',
    ],
    executable_path: [
      'Try: "Find path to Chrome browser"',
      'Ask: "Where is Python installed?"',
      'Say: "Suggest common program paths"',
    ],
    program_args: [
      'Try: "What arguments does npm need?"',
      'Ask: "Explain this command flag"',
      'Say: "Suggest safe arguments"',
    ],
  };

  return (
    contextTips[context.value] || [
      'Try: "List all active automation flows"',
      'Ask: "Switch OBS to BRB scene and mute audio"',
      'Say: "Show recent events from the inbox"',
    ]
  );
});

// Dynamic Context-Aware Suggestion Chips (Modular Composable)
const { contextSuggestions } = useContextSuggestions(activeContext);

const applyQuickPrompt = (promptText: string) => {
  userInput.value = promptText;
  sendMessage();
};

// State
const isPinned = ref(false);

// Methods
const closeWindow = async () => {
  await currentWindow.close();
};

const togglePin = async () => {
  try {
    isPinned.value = !isPinned.value;
    await currentWindow.setAlwaysOnTop(isPinned.value);
  } catch (err) {
    console.error("Failed to toggle pin state:", err);
  }
};

const quickPrompt = async (prompt: string) => {
  userInput.value = prompt;
  await nextTick();
  await sendMessage();
};

const sendMessage = async () => {
  if (!userInput.value.trim() || isLoading.value) return;

  const userMessage = userInput.value.trim();
  userInput.value = "";

  // Create conversation file if doesn't exist (first message)
  if (!currentConversationId.value) {
    console.log("Creating conversation file for first message...");
    try {
      const id = await invoke<string>("create_and_attach_conversation", {
        title: "New Chat",
        model: currentChatSettings.value.model,
        providerName: currentChatSettings.value.provider,
      });
      currentConversationId.value = id;
      console.log("Conversation file created:", id);

      // Immediately refresh conversation list to show new conversation
      if (conversationHistory.value?.loadConversations) {
        conversationHistory.value.loadConversations();
      }
    } catch (err) {
      console.error("Failed to create conversation:", err);
      return;
    }
  }

  // Add user message (pre-format for performance)
  messages.value.push({
    role: "user",
    content: userMessage,
    formattedContent: formatMessage(userMessage),
  });

  // Scroll to bottom
  nextTick(() => {
    if (messagesContainer.value) {
      messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight;
    }
  });

  isLoading.value = true;
  const taskId = crypto.randomUUID();
  activeTaskId.value = taskId;
  activeStreamSequence.value = 0;

  // Add placeholder for assistant message (for streaming)
  const assistantMessageIndex = messages.value.length;
  messages.value.push({
    role: "assistant",
    content: "",
    formattedContent: "",
    model: currentChatSettings.value.model,
    provider: currentChatSettings.value.provider,
  });

  // Don't set streaming yet - let it be triggered by first chunk
  // This allows the typing indicator to show while waiting
  streamingMessageIndex.value = assistantMessageIndex;

  try {
    // Build context for AI
    const contextMessage = buildContextMessage();

    // Call AI backend (using MCP chat)
    // Messages are auto-persisted in Rust ChatSession
    const response = await invoke<{
      message: string;
      suggestions?: any[];
      model?: string;
      provider?: string;
      usage?: {
        prompt_tokens: number;
        completion_tokens: number;
        total_tokens: number;
        evidence: "provider_measured" | "partial" | "unknown";
      };
    }>("send_chat_message", {
      message: userMessage,
      context: contextMessage,
      conversationId: currentConversationId.value,
      taskId,
    });

    // Update the placeholder message with final data
    const finalMessage = messages.value[assistantMessageIndex];
    if (!finalMessage) throw new Error("Assistant response placeholder was removed");
    finalMessage.content = response.message;
    finalMessage.formattedContent = formatMessage(response.message);
    finalMessage.suggestions = response.suggestions;
    finalMessage.model = response.model;
    finalMessage.provider = response.provider;
    finalMessage.usage = response.usage;

    console.log("📊 Received usage data:", JSON.stringify(response.usage, null, 2));
    console.log("📊 Full response:", JSON.stringify(response, null, 2));

    // Auto-scroll only if user is already at the bottom
    nextTick(() => {
      if (messagesContainer.value) {
        const container = messagesContainer.value;
        const isAtBottom =
          container.scrollHeight - container.scrollTop - container.clientHeight < 50;

        if (isAtBottom) {
          container.scrollTop = container.scrollHeight;
        }
      }
    });

    // Refresh conversation list to show new conversation and updated message count
    if (conversationHistory.value?.loadConversations) {
      conversationHistory.value.loadConversations();
    }
  } catch (error) {
    console.error("AI Assistant error:", error);
    const errorMsg = `Sorry, I encountered an error: ${error}`;

    // Update placeholder with error message
    const errorMessage = messages.value[assistantMessageIndex];
    if (errorMessage) {
      errorMessage.content = errorMsg;
      errorMessage.formattedContent = formatMessage(errorMsg);
    }
  } finally {
    isStreaming.value = false;
    streamingMessageIndex.value = null;
    if (activeTaskId.value === taskId) activeTaskId.value = null;
    isLoading.value = false;
  }
};

const buildContextMessage = (): string => {
  let ctxMsg = "";

  // Auto-injected studio context (Deck Button, Flow Node, Deck, etc.)
  if (activeContext.value) {
    ctxMsg += `[ACTIVE STUDIO CONTEXT]\n`;
    ctxMsg += `Type: ${activeContext.value.type}\n`;
    ctxMsg += `Title: ${activeContext.value.title}\n`;
    if (activeContext.value.subtitle) ctxMsg += `Details: ${activeContext.value.subtitle}\n`;
    ctxMsg += `Payload Data: ${JSON.stringify(activeContext.value.data, null, 2)}\n\n`;
  }

  if (context.value) {
    ctxMsg += `Context: ${context.value}\n`;
  }

  if (currentInputValue.value) {
    ctxMsg += `Current input value: "${currentInputValue.value}"\n`;
  }

  if (Object.keys(formData.value).length > 0) {
    ctxMsg += `Form data: ${JSON.stringify(formData.value, null, 2)}\n`;
  }

  return ctxMsg;
};


const applySuggestion = async (suggestion: {
  label: string;
  value: any;
  action?: string;
}) => {
  try {
    const actionType = suggestion.action || "fill";

    if (actionType === "flow_update_node" || actionType === "flow_apply_patch") {
      const refusal =
        "Flow edit not applied. This suggestion has no verified revision or persistence receipt. Review it in the Flow Editor instead.";
      messages.value.push({
        role: "assistant",
        content: refusal,
        formattedContent: formatMessage(refusal),
      });
      await nextTick();
      if (messagesContainer.value) {
        messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight;
      }
      return;
    } else {
      // Send suggestion back to parent window
      await invoke("assistant_fill_value", {
        targetWindow: currentWindow.label,
        value: typeof suggestion.value === "string" ? suggestion.value : JSON.stringify(suggestion.value),
        action: actionType,
      });
    }
  } catch (err) {
    console.error("Failed to apply suggestion:", err);
  }
};

// Format message content using marked for markdown rendering
const formatMessage = (content: string): string => {
  return marked(content) as string;
};

// Conversation management methods
const createNewConversation = async () => {
  // Don't create if current conversation is already empty
  if (currentConversationId.value === null && messages.value.length === 0) {
    console.log("Already have an empty conversation, not creating new one");
    return;
  }

  console.log("Creating new conversation in memory only...");

  // Just clear current state - don't create file yet
  currentConversationId.value = null;
  messages.value = [];

  // File will be created when first message is sent
  console.log("New empty conversation ready (will save on first message)");
};

const applyChatSettings = async (settings: any) => {
  console.log("🔧 Applying chat settings:", settings);

  // Check if provider changed - if so, reinitialize session
  const providerChanged = currentChatSettings.value.provider !== settings.provider;
  console.log(`🔧 Provider changed? ${providerChanged} (${currentChatSettings.value.provider} -> ${settings.provider})`);

  currentChatSettings.value = { ...settings };

  if (providerChanged) {
    console.log(`🔄 Provider changed to ${settings.provider} - reinitializing chat session`);
    try {
      // Reinitialize chat with new provider
      await invoke("init_chat", {
        request: {
          provider: settings.provider,
          modelName: settings.model,
        }
      });
      console.log("✅ Chat session reinitialized with new provider");
    } catch (err) {
      console.error("❌ Failed to reinitialize chat:", err);
      return; // Don't proceed if reinitialization failed
    }
  }

  // Update current chat session with new settings (only if we have an active conversation)
  if (currentConversationId.value) {
    try {
      await invoke("update_chat_settings", {
        conversationId: currentConversationId.value,
        provider: settings.provider,
        model: settings.model,
        temperature: settings.temperature,
        maxTokens: settings.maxTokens,
        toolsMode: settings.toolsMode,
        selectedTools: settings.selectedTools,
        stream: settings.stream,
        systemPrompt: settings.systemPrompt || null,
        toolTimeout: settings.toolTimeout || 30,
      });
    } catch (err) {
      console.error("Failed to apply chat settings:", err);
    }
  } else {
    console.log("📝 Settings saved locally (will apply when conversation is created)");
  }
};

const saveDefaultChatSettings = async (settings: any) => {
  console.log("Saving default chat settings:", settings);

  // Save to app config/settings store
  try {
    await invoke("save_default_chat_settings", settings);
  } catch (err) {
    console.error("Failed to save default settings:", err);
  }
};

const stopGeneration = async () => {
  console.log("🛑 Stopping AI generation...");

  // Abort the entire generation (including any tool calls)
  try {
    await invoke("abort_generation", {
      conversationId: currentConversationId.value,
      taskId: activeTaskId.value,
    });
    console.log("✅ AI generation aborted");
  } catch (err) {
    console.error("⚠️ Failed to abort generation:", err);
  }

  // The command remains the task owner until an in-flight tool has returned
  // and its real result is persisted. `finally` clears loading at that point.
  isStreaming.value = false;
  toolProgress.value.active = false;

  console.log("✅ Generation stopped");
};

const loadConversation = async (id: string) => {
  try {
    console.log("Loading conversation:", id);

    const storedMessages = await invoke<any[]>("load_conversation_messages", {
      conversationId: id,
      useContextManagement: true,
    });

    // Convert stored messages to UI format (pre-format for performance)
    // Filter out system and tool messages - they're internal, not for display
    messages.value = storedMessages
      .filter((m) => m.role !== "system" && m.role !== "tool")
      .filter((m) => m.content && m.content.trim() !== "") // Skip empty messages
      .map((m) => {
        const content = m.content || "";
        return {
          role: m.role,
          content,
          formattedContent: formatMessage(content),
          suggestions: [],
          model: m.model,
          provider: m.provider,
        };
      });

    currentConversationId.value = id;

    // Attach to chat session and get its settings
    const settings = await invoke<any>("attach_conversation_to_session", {
      conversationId: id,
      useContextManagement: true,
    });

    // Apply settings (this will reinitialize if provider changed)
    await applyChatSettings(settings);
    console.log("📋 Restored conversation settings:", settings);

    console.log("Conversation loaded:", messages.value.length, "messages");
  } catch (err) {
    console.error("Failed to load conversation:", err);
  }
};

// Store unlisten functions for cleanup
let unlistenStreamChunk: (() => void) | null = null;
let unlistenToolProgress: (() => void) | null = null;

// Listen for context updates from parent window
onMounted(async () => {
  // Get initial context from window title or query params
  const label = currentWindow.label;
  console.log("Assistant window mounted:", label);

  // Streaming is task-scoped; events from another assistant window or an old
  // request must never mutate this conversation's placeholder.
  unlistenStreamChunk = await listen<AssistantTaskChunk>("assistant-chunk", (event) => {
    const payload = event.payload;
    const conversationId = currentConversationId.value;
    if (!conversationId) return;
    if (!isCurrentAssistantTaskChunk(
      payload,
      activeTaskId.value,
      conversationId,
      activeStreamSequence.value,
    )) return;
    activeStreamSequence.value = payload.sequence;

    if (payload.done) {
      isStreaming.value = false;
      return;
    }

    // First chunk triggers streaming mode
    if (!isStreaming.value) {
      isStreaming.value = true;
    }

    const chunkText = payload.content;
    if (streamingMessageIndex.value !== null && chunkText) {
      const msg = messages.value[streamingMessageIndex.value];
      if (msg) {
        msg.content += chunkText;
        msg.formattedContent = formatMessage(msg.content);

        // Auto-scroll only if user is already at the bottom (not if they scrolled up)
        nextTick(() => {
          if (messagesContainer.value) {
            const container = messagesContainer.value;
            const isAtBottom =
              container.scrollHeight - container.scrollTop - container.clientHeight < 50;

            if (isAtBottom) {
              container.scrollTop = container.scrollHeight;
            }
          }
        });
      }
    }
  });

  // Listen for MCP tool progress events
  unlistenToolProgress = await listen<{
    progress_token: string;
    progress: number;
    total: number | null;
    message: string | null;
  }>("mcp-tool-progress", (event) => {
    console.log("📊 MCP Tool Progress:", event.payload);

    const { progress_token, progress, total, message } = event.payload;

    // Calculate percentage
    const percentage = total ? (progress / total) * 100 : 0;

    // Update progress state
    toolProgress.value = {
      active: true,
      token: progress_token,
      progress,
      total,
      percentage,
      message: message || `${progress}${total ? `/${total}` : ""}`,
    };

    // Attach / update tool execution badge on latest assistant message
    if (messages.value.length > 0) {
      const lastMsg = messages.value[messages.value.length - 1];
      if (lastMsg && lastMsg.role === "assistant") {
        if (!lastMsg.toolExecutions) lastMsg.toolExecutions = [];
        const toolName = message || "MCP Tool";
        let existing = lastMsg.toolExecutions.find((t) => t.name === toolName);
        if (!existing) {
          lastMsg.toolExecutions.push({
            name: toolName,
            summary: `${progress}${total ? `/${total}` : ""}`,
            status: total && progress >= total ? "success" : "running",
            expanded: false,
          });
        } else {
          existing.status = total && progress >= total ? "success" : "running";
          existing.summary = `${progress}${total ? `/${total}` : ""}`;
        }
      }
    }

    // Auto-hide when complete
    if (total && progress >= total) {
      setTimeout(() => {
        toolProgress.value.active = false;
      }, 2000); // Keep showing for 2 seconds after completion
    }
  });

  // Initialize studio auto-context listener
  await initContextListener();

  // Initialize AI models from Pinia store (loads MCP provider configs)
  try {
    console.log("🔄 Initializing AI models store...");
    const mcpConfig = await invoke<any>("read_mcp_config");
    const providerConfigs = mcpConfig?.models?.model || [];
    // Assistant window: only use cache, don't fetch from APIs
    await aiModelsStore.initialize(providerConfigs, false);
    console.log("✅ AI models store initialized (cache only)");
  } catch (err) {
    console.warn("⚠️ Failed to initialize AI models store:", err);
  }

  // Load default chat settings from config
  let defaultSettings: any;
  try {
    defaultSettings = await invoke<any>("get_default_chat_settings");
    currentChatSettings.value = defaultSettings;
    console.log("📋 Loaded default chat settings:", defaultSettings);
  } catch (err) {
    console.error("❌ Failed to load chat settings from config:", err);
    console.error("   Please configure at least one AI provider in settings");
    // Don't initialize chat if no config exists - fail explicitly
    return;
  }

  // Initialize chat session with the default provider
  try {
    console.log("🚀 Initializing chat session with provider:", defaultSettings.provider);
    await invoke("init_chat", {
      request: {
        provider: defaultSettings.provider,  // Use provider from default settings
        modelName: defaultSettings.model,
      }
    });
    console.log("✅ Chat session ready with", defaultSettings.provider);

    // Apply full settings to backend session
    await applyChatSettings(defaultSettings);
    console.log("✅ Applied default settings to chat session");
  } catch (err) {
    console.error("❌ Chat session initialization failed:", err);
  }

  // Load conversation if passed via URL query parameter
  const queryConvId = route.query.conversationId;
  if (queryConvId && typeof queryConvId === "string") {
    currentConversationId.value = queryConvId;
    try {
      await loadConversation(queryConvId);
    } catch (_) {}
  }

  // Listen for context updates
  const unlisten = await listen("assistant-context-update", async (event: any) => {
    console.log("Context update received:", event.payload);
    if (event.payload.context) context.value = event.payload.context;
    if (event.payload.contextInfo) contextInfo.value = event.payload.contextInfo;
    if (event.payload.currentValue) currentInputValue.value = event.payload.currentValue;
    if (event.payload.formData) formData.value = event.payload.formData;
    if (event.payload.conversationId) {
      currentConversationId.value = event.payload.conversationId;
      try {
        await loadConversation(event.payload.conversationId);
      } catch (_) {}
    }
  });

  // Request initial context from parent
  await invoke("get_assistant_context", {
    windowLabel: label,
  });

  // Reveal window smoothly after theme and context are loaded
  try {
    await currentWindow.show();
    await currentWindow.setFocus();
  } catch (_) {}

  // Focus input
  nextTick(() => {
    chatInput.value?.focus();
  });
});

const openNewAssistantWindow = async () => {
  try {
    const newLabel = `assistant-${Date.now()}`;
    await invoke("open_assistant_window", {
      windowLabel: newLabel,
      context: context.value,
      contextInfo: contextInfo.value,
      currentValue: currentInputValue.value,
      formData: formData.value,
    });
  } catch (error) {
    console.error("Failed to open new assistant window:", error);
  }
};

// Cleanup listeners on unmount
onUnmounted(() => {
  if (unlistenStreamChunk) unlistenStreamChunk();
  if (unlistenToolProgress) unlistenToolProgress();
  disposeContextListener();
});
</script>

<style scoped>
/* Custom scrollbar for messages */
::-webkit-scrollbar {
  width: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.1);
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.2);
}

/* Markdown styling for AI messages */
:deep(pre) {
  background: rgba(0, 0, 0, 0.3);
  border-radius: 8px;
  padding: 12px;
  margin: 8px 0;
  overflow-x: auto;
  font-size: 12px;
}

:deep(code) {
  font-family: "Consolas", "Monaco", "Courier New", monospace;
  font-size: 12px;
}

:deep(p code),
:deep(li code) {
  background: rgba(0, 0, 0, 0.3);
  padding: 2px 6px;
  border-radius: 4px;
}

:deep(p) {
  margin: 4px 0;
}

:deep(ul),
:deep(ol) {
  margin: 8px 0;
  padding-left: 20px;
}

:deep(li) {
  margin: 4px 0;
}

:deep(h1),
:deep(h2),
:deep(h3) {
  margin: 12px 0 8px 0;
  font-weight: 600;
}

:deep(blockquote) {
  border-left: 3px solid rgba(59, 130, 246, 0.5);
  margin: 8px 0;
  padding-left: 12px;
  color: rgba(255, 255, 255, 0.7);
}

:deep(a) {
  color: #60a5fa;
  text-decoration: underline;
}

:deep(hr) {
  border: none;
  border-top: 1px solid rgba(255, 255, 255, 0.1);
  margin: 12px 0;
}

/* Smooth animations */
.animate-bounce {
  animation: bounce 1s infinite;
}

@keyframes bounce {
  0%,
  100% {
    transform: translateY(0);
  }
  50% {
    transform: translateY(-4px);
  }
}
</style>
