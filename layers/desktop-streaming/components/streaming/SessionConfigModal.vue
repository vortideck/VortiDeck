<template lang="pug">
div
  //- // Drag icon for visual feedback during drag operations
  //- div#drag-icon(class="fixed top-[-1000px] left-[-1000px] h-[2rem] w-[2rem] flex items-center justify-center aspect-square bg-black text-white font-bold pointer-events-none z-[9999]")
  //-   MaterialSymbolsTerminalOutline(class="w-6 h-6 text-white")

  Transition(name="fade")
    div(v-if="modelValue" @click="closeModal" tabindex="-1" class="backdrop-blur-sm bg-[rgba(0,0,0,0.55)] fixed top-0 flex items-center justify-center left-0 right-0 z-50 w-full p-4 overflow-x-hidden overflow-y-auto md:inset-0 h-screen md:h-full")

  Transition(name="jump")
    div(v-if="modelValue" class="fixed top-0 pointer-events-auto flex items-center justify-center left-0 right-0 z-50 mx-auto w-full max-w-4xl p-4 md:inset-0 h-screen md:h-min my-auto scrollbar-visible")
      div(class="relative bg-themeBackground2 rounded-xl shadow-2xl border border-white/10 w-full max-h-[90vh] flex flex-col" @click.stop)

        // Header
        div(class="px-6 py-4 border-b border-themeBorder2 flex items-center justify-between bg-themeBackground3")
          div
            h2(class="text-xl font-bold text-themeText flex items-center gap-2")
              MaterialSymbolsVideoLibraryOutline(class="w-6 h-6")
              span(v-if="sessionData.id") Edit Session
              span(v-else) Create New Session
            p(class="text-sm text-themeText2 mt-1") Configure your streaming session template

          button(
            @click="closeModal"
            class="p-2 hover:bg-themeBackground rounded-lg transition-colors"
          )
            MaterialSymbolsCloseOutline(class="w-5 h-5 text-themeText2")

        // Content
        div(class="overflow-y-auto max-h-[calc(90vh-140px)]")
          div(class="p-6 space-y-6")

            // Basic Information
            div
              h3(class="text-lg font-semibold text-themeText mb-4 flex items-center gap-2")
                MaterialSymbolsInfoOutline(class="w-5 h-5")
                | Basic Information

              div(class="space-y-4")
                // Session Name
                div
                  label(class="block text-sm font-medium text-themeText mb-2") Session Name
                  input(
                    v-model="sessionData.name"
                    type="text"
                    placeholder="e.g., Monday Gaming Stream"
                    class="w-full px-4 py-2 bg-themeBackground border border-themeBorder2 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 text-themeText"
                  )

                // Session Description
                div
                  label(class="block text-sm font-medium text-themeText mb-2") Description (Optional)
                  textarea(
                    v-model="sessionData.description"
                    rows="2"
                    placeholder="Brief description of this session..."
                    class="w-full px-4 py-2 bg-themeBackground border border-themeBorder2 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 text-themeText resize-none"
                  )

            // Platform Account Selection
            div
              div(class="flex items-center justify-between mb-4")
                h3(class="text-lg font-semibold text-themeText flex items-center gap-2")
                  MaterialSymbolsDevicesOutline(class="w-5 h-5")
                  | Select Streaming Accounts
                button(
                  @click="openManageAccounts"
                  class="px-3 py-1.5 bg-blue-500 hover:bg-blue-600 text-white rounded-lg text-sm font-medium flex items-center gap-1.5 transition-colors"
                )
                  MaterialSymbolsSettingsOutline(class="w-4 h-4")
                  span Manage Accounts

              p(class="text-sm text-themeText2 mb-4") Choose which accounts to use for this session. You can select multiple accounts from different platforms.

              // Show platforms with their accounts
              div(class="space-y-4")
                div(
                  v-for="platform in platformsWithAccounts"
                  :key="platform.platform"
                  class="bg-themeBackground rounded-lg border border-themeBorder2 p-4"
                )
                  // Platform header
                  div(class="flex items-center gap-3 mb-3")
                    component(:is="getPlatformIcon(platform.platform)" class="w-6 h-6")
                    h4(class="font-semibold text-themeText capitalize") {{ getPlatformDisplayName(platform.platform) }}
                    span(v-if="platform.accounts.length === 0" class="text-xs text-themeText2") (No accounts connected)

                  // Accounts list
                  div(v-if="platform.accounts.length > 0" class="space-y-2")
                    button(
                      v-for="account in platform.accounts"
                      :key="account.id"
                      @click="toggleAccount(platform.platform, account)"
                      :class="[isAccountSelected(account.id) ? 'bg-blue-500/20 border-blue-500 ring-2 ring-blue-500/30' : 'bg-themeBackground2 border-themeBorder2 hover:bg-themeBackground3', 'w-full p-3 rounded-lg border transition-all flex items-center justify-between']"
                    )
                      div(class="flex items-center gap-3")
                        img(
                          v-if="account.extra_data?.picture"
                          :src="account.extra_data.picture"
                          :alt="getAccountDisplayName(account)"
                          class="w-8 h-8 rounded-full"
                        )
                        div(v-else class="w-8 h-8 rounded-full bg-gray-700 flex items-center justify-center text-sm font-bold") {{ getAccountDisplayName(account)[0].toUpperCase() }}
                        div(class="text-left")
                          p(class="text-sm font-medium text-themeText") {{ getAccountDisplayName(account) }}
                          p(v-if="account.extra_data?.email" class="text-xs text-themeText2") {{ account.extra_data.email }}
                          p(v-if="needsReconnection(account)" class="text-xs text-red-400") ⚠ Needs reconnection
                      div(class="flex items-center gap-2")
                        span(v-if="account.is_primary" class="text-xs px-2 py-1 bg-yellow-600 rounded") Primary
                        span(v-if="isAccountSelected(account.id)" class="text-xs text-blue-400") ✓ Selected

                  // No accounts connected - show connect button
                  div(v-else class="text-center py-4")
                    p(class="text-sm text-themeText2 mb-3") No {{ platform.platform }} accounts connected
                    button(
                      @click="openManageAccounts"
                      class="inline-flex items-center gap-2 px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg text-sm font-medium transition-colors"
                    )
                      span + Connect {{ platform.platform }} Account

            // Commands Configuration
            div
              div(class="flex items-center justify-between mb-4")
                h3(class="text-lg font-semibold text-themeText flex items-center gap-2")
                  MaterialSymbolsTerminalOutline(class="w-5 h-5")
                  | Chat Commands
          

              div(v-if="sessionData.commands.length === 0" class="text-center py-8 bg-themeBackground rounded-lg border border-dashed border-themeBorder2")
                MaterialSymbolsTerminalOutline(class="w-12 h-12 text-gray-600 mx-auto mb-2 opacity-50")
                p(class="text-themeText2") No commands configured yet
                p(class="text-sm text-themeText3 mt-1") Add commands to trigger deck actions from chat

              div(v-else class="space-y-3")
                div(
                  v-for="(command, index) in sessionData.commands"
                  :key="index"
                  class="p-4 bg-themeBackground rounded-lg border border-themeBorder2 "
                )
                  div(class="flex items-start gap-3")
                    div(class="flex-1 grid grid-cols-2 gap-3")
                      // Command Trigger
                      div
                        label(class="block text-xs font-medium text-themeText2 mb-1.5") Command
                        input(
                          v-model="command.trigger"
                          type="text"
                          placeholder="!example"
                          class="w-full px-3 py-2 bg-themeBackground2 border border-themeBorder2 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 text-themeText"
                        )

                      // Deck Action Selection
                      div(class="")
                        label(class="block text-xs font-medium text-themeText2 mb-1.5") Deck Action
                        div(
                          @dragover="handleDragOver($event, index)"
                          @dragleave="handleDragLeave(index)"
                          @drop="handleDrop($event, index)"
                          :class="isDraggingOver === index ? 'ring-2 ring-blue-500 bg-blue-500/5' : ''"
                          class="transition-all rounded-lg relative"
                        )
                          Multiselect(
                            :model-value="getSelectedAction(command.action)"
                            @update:model-value="updateCommandAction(command, $event)"
                            :options="groupedActionsForMultiselect"
                            :searchable="true"
                            :close-on-select="true"
                            :show-labels="false"
                            label="label"
                            track-by="action"
                            group-values="items"
                            group-label="group"
                            :group-select="false"
                            placeholder="Search or drag action from deck..."
                            class="action-multiselect"
                          )
                            template(#noResult) No actions found
                            template(#noOptions) No actions available

                          // Drop indicator overlay
                          div(
                            v-if="isDraggingOver === index"
                            class="absolute inset-0 pointer-events-none flex items-center justify-center bg-blue-500/10 rounded-lg border-2 border-blue-500"
                          )
                            p(class="text-blue-500 font-semibold text-xs") Drop to assign

                  CommandInputBindingsEditor(:model-value="command.inputBindings || {}" @update:model-value="command.inputBindings = $event")

                  // Permissions Section
                  div(class="mt-3 p-3 bg-themeBackground2 rounded-lg border border-themeBorder2")
                    details(class="group")
                      summary(class="cursor-pointer text-xs font-medium text-themeText2 flex items-center gap-2")
                        span 🔒 Permissions (Who can use this command?)
                        span(class="text-themeText3 text-xs") Click to configure

                      div(class="mt-3 space-y-3")
                        // Platform-specific roles
                        div(v-for="platform in sessionData.platforms" :key="platform.platform")
                          label(class="block text-xs font-medium text-themeText2 mb-1.5")
                            | {{ platform.platform.charAt(0).toUpperCase() + platform.platform.slice(1) }} Roles
                          div(class="flex flex-wrap gap-2")
                            label(
                              v-for="role in getPlatformRoles(platform.platform)"
                              :key="role"
                              class="inline-flex items-center gap-1.5 px-2 py-1 bg-themeBackground3 border border-themeBorder2 rounded cursor-pointer hover:bg-blue-500/10 transition-colors"
                            )
                              input(
                                type="checkbox"
                                :checked="isRoleSelected(command, platform.platform, role)"
                                @change="toggleRole(command, platform.platform, role)"
                                class="w-3 h-3 text-blue-500 rounded"
                              )
                              span(class="text-xs text-themeText") {{ role }}
                          p(class="text-xs text-themeText3 mt-1") Everyone permits all viewers. No selected roles blocks this platform unless a whitelist grants access.

                        // Whitelist
                        div
                          label(class="block text-xs font-medium text-themeText2 mb-1.5")
                            | ✅ Whitelist (Only these users)
                          input(
                            :value="arrayToString(command.userWhitelist)"
                            @input="command.userWhitelist = stringToArray($event.target.value)"
                            type="text"
                            placeholder="username1, username2 (leave empty to allow all)"
                            class="w-full px-2 py-1.5 bg-themeBackground3 border border-themeBorder2 rounded text-xs focus:outline-none focus:ring-1 focus:ring-blue-500 text-themeText"
                          )
                          p(class="text-xs text-themeText3 mt-1") A nonempty whitelist permits only these users and overrides role selection. Blacklisted users remain blocked. Leave empty to use roles.

                        // Blacklist
                        div
                          label(class="block text-xs font-medium text-themeText2 mb-1.5")
                            | 🚫 Blacklist (Block these users)
                          input(
                            :value="arrayToString(command.userBlacklist)"
                            @input="command.userBlacklist = stringToArray($event.target.value)"
                            type="text"
                            placeholder="username1, username2"
                            class="w-full px-2 py-1.5 bg-themeBackground3 border border-themeBorder2 rounded text-xs focus:outline-none focus:ring-1 focus:ring-blue-500 text-themeText"
                          )
                          p(class="text-xs text-themeText3 mt-1") Comma-separated usernames to block from using this command.

                  div(class="flex justify-end")
                    button(
                      @click="removeCommand(index)"
                      class="p-2 hover:bg-red-500/20 rounded-lg transition-colors"
                    )
                      MaterialSymbolsDeleteOutline(class="w-4 h-4 text-red-400")

              // Quick-add drop zone
              div(
                @dragover.prevent="quickDropHover = true"
                @dragleave="quickDropHover = false"
                @drop="handleQuickDrop"
                :class="quickDropHover ? 'border-blue-500 bg-blue-500/10' : 'border-themeBorder2'"
                class="p-4 border-2 border-dashed rounded-lg transition-all mt-4 cursor-pointer hover:border-blue-400 hover:bg-blue-500/5"
              )
                div(class="flex items-center justify-center gap-2 text-themeText2")
                  MaterialSymbolsTerminalOutline(class="w-5 h-5")
                  p(class="text-sm") Drag deck actions here to create commands

              button(
                  @click="addCommand"
                  class="px-3 py-1.5 ml-auto mt-4 bg-blue-500 hover:bg-blue-600 text-white rounded-lg text-sm font-medium flex items-center gap-1.5 transition-colors"
                )
                  MaterialSymbolsAddOutline(class="w-4 h-4")
                  span Add Command


            // Session Settings
            div
              h3(class="text-lg font-semibold text-themeText mb-4 flex items-center gap-2")
                MaterialSymbolsSettingsOutline(class="w-5 h-5")
                | Session Settings

              div(class="space-y-3")
                // Auto-start when going live
                div(class="flex items-center justify-between p-3 bg-themeBackground rounded-lg border border-themeBorder2")
                  div
                    p(class="font-medium text-themeText") Auto-start on Live
                    p(class="text-sm text-themeText2") Automatically activate when stream goes live
                  label(class="relative inline-flex items-center cursor-pointer")
                    input(
                      v-model="sessionData.settings.autoStart"
                      type="checkbox"
                      class="sr-only peer"
                    )
                    div(class="w-11 h-6 bg-gray-600 peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-blue-500 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-500")

                // Enable notifications
                div(class="flex items-center justify-between p-3 bg-themeBackground rounded-lg border border-themeBorder2")
                  div
                    p(class="font-medium text-themeText") Enable Notifications
                    p(class="text-sm text-themeText2") Get notified about session events
                  label(class="relative inline-flex items-center cursor-pointer")
                    input(
                      v-model="sessionData.settings.notifications"
                      type="checkbox"
                      class="sr-only peer"
                    )
                    div(class="w-11 h-6 bg-gray-600 peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-blue-500 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-500")

                // Auto-resume
                div(class="p-3 bg-themeBackground rounded-lg border border-themeBorder2 space-y-3")
                  div(class="flex items-center justify-between")
                    div
                      p(class="font-medium text-themeText") Auto-resume Session
                      p(class="text-sm text-themeText2") Automatically reactivate this session if your stream crashes and restarts
                    label(class="relative inline-flex items-center cursor-pointer")
                      input(
                        v-model="sessionData.settings.autoResume"
                        type="checkbox"
                        class="sr-only peer"
                      )
                      div(class="w-11 h-6 bg-gray-600 peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-blue-500 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-500")

                  // Grace period and priority (only show if auto-resume enabled)
                  div(v-if="sessionData.settings.autoResume" class="grid grid-cols-2 gap-3 pt-2 border-t border-themeBorder2")
                    div
                      label(class="block text-xs font-medium text-themeText2 mb-1.5")
                        | Grace Period (minutes)
                        span(class="ml-1 text-themeText3" title="How long to wait before giving up on auto-resume") ⓘ
                      input(
                        v-model.number="sessionData.settings.gracePeriodMinutes"
                        type="number"
                        min="1"
                        max="120"
                        placeholder="30"
                        class="w-full px-3 py-2 bg-themeBackground2 border border-themeBorder2 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 text-themeText"
                      )
                      p(class="text-xs text-themeText3 mt-1") If stream restarts within this time, session auto-resumes
                    div
                      label(class="block text-xs font-medium text-themeText2 mb-1.5")
                        | Priority (0-10)
                        span(class="ml-1 text-themeText3" title="Higher priority sessions resume first") ⓘ
                      input(
                        v-model.number="sessionData.settings.priority"
                        type="number"
                        min="0"
                        max="10"
                        placeholder="0"
                        class="w-full px-3 py-2 bg-themeBackground2 border border-themeBorder2 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 text-themeText"
                      )
                      p(class="text-xs text-themeText3 mt-1") Higher values = more important (0 = normal)

        // Footer Actions
        div(class="px-6 py-4 border-t border-themeBorder2 bg-themeBackground3 flex items-center justify-between")
          button(
            @click="closeModal"
            class="px-4 py-2 bg-themeBackground2 hover:bg-themeBackground rounded-lg font-medium transition-colors text-themeText"
          ) Cancel

          div(class="flex items-center gap-2")
            button(
              v-if="sessionData.id"
              @click="saveDraft"
              class="px-4 py-2 bg-themeBackground2 hover:bg-themeBackground rounded-lg font-medium transition-colors text-themeText"
            ) Save Draft

            button(
              @click="saveAndActivate"
              :disabled="!isValid"
              :class="[isValid ? 'bg-blue-500 hover:bg-blue-600' : 'bg-gray-600 cursor-not-allowed opacity-50', 'px-4 py-2 rounded-lg font-medium transition-colors text-white']"
            )
              span(v-if="sessionData.id") Save Changes
              span(v-else) Create Session
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { openUrl } from '@tauri-apps/plugin-opener';
import MaterialSymbolsVideoLibraryOutline from '~icons/material-symbols/video-library-outline';
import MaterialSymbolsCloseOutline from '~icons/material-symbols/close';
import MaterialSymbolsInfoOutline from '~icons/material-symbols/info';
import MaterialSymbolsDevicesOutline from '~icons/material-symbols/devices';
import MaterialSymbolsTerminalOutline from '~icons/material-symbols/terminal';
import MaterialSymbolsAddOutline from '~icons/material-symbols/add';
import MaterialSymbolsDeleteOutline from '~icons/material-symbols/delete-outline';
import MaterialSymbolsSettingsOutline from '~icons/material-symbols/settings';
import LogosTwitch from '~icons/logos/twitch';
import LogosYoutube from '~icons/logos/youtube-icon';
import SimpleIconsKick from '~icons/simple-icons/kick';
import LogosTiktok from '~icons/logos/tiktok-icon';
import Multiselect from 'vue-multiselect';
import 'vue-multiselect/dist/vue-multiselect.css';
import { useConnectedPlatforms, type ConnectedAccount } from '../../composables/streaming/useConnectedPlatforms';
import type { PlatformStream } from '../../composables/streaming/useSessionManager';
import CommandInputBindingsEditor from './CommandInputBindingsEditor.vue';

const props = defineProps<{
  modelValue: boolean;
  session?: any;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: boolean];
  'save': [session: any];
}>();

// Import connected platforms composable

const { getPlatformDisplayName, platformsWithAccounts, fetchAccounts, getAccountDisplayName, needsReconnection } = useConnectedPlatforms();

// Session data
const sessionData = ref({
  id: '',
  name: '',
  description: '',
  platforms: [] as PlatformStream[], // Changed from string[] to PlatformStream[]
  commands: [] as Array<{
    id?: string;
    permissionVersion?: number;
    inputBindings?: Record<string, import('@base/types/actionInputs').CommandInputSource>;
    trigger: string;
    action: string;
    cooldown: number | null;
    enabled: boolean;
    allowedRoles?: Record<string, string[]>;
    userWhitelist?: string[];
    userBlacklist?: string[];
  }>,
  settings: {
    autoStart: true,
    notifications: true,
    autoResume: true,
  },
});

// Drag-and-drop state management
const isDraggingOver = ref<number | null>(null);
const quickDropHover = ref(false);

// Available platforms (moved to composable)

// Deck actions - loaded from deck store
const availableDeckActions = ref<Array<{ id: string; name: string; deckName: string; action: string }>>([]);

// Load deck actions and connected accounts on mount
onMounted(async () => {
  await loadDeckActions();
  await fetchAccounts(); // Load connected platforms (uses cache if available)
});

async function loadDeckActions() {
  try {
    const actions: Array<{ id: string; name: string; deckName: string; action: string }> = [];

    // Load Programs
    try {
      const programs = await invoke<any[]>('get_programs_with_meta', { includeIcons: false });
      if (programs && Array.isArray(programs)) {
        programs.forEach((program) => {
          actions.push({
            id: `program-${program.id}`,
            name: program.name || `Program ${program.id}`,
            deckName: 'Programs',
            action: `{"Program":${program.id}}`,
          });
        });
      }
    } catch (err) {
      console.warn('Failed to load programs:', err);
    }

    // Load Macros
    try {
      const macros = await invoke<any[]>('get_all_macros');
      if (macros && Array.isArray(macros)) {
        macros.forEach((macro) => {
          actions.push({
            id: `macro-${macro.id}`,
            name: macro.name || `Macro ${macro.id}`,
            deckName: 'Macros',
            action: `{"MacroId":${macro.id}}`,
          });
        });
      }
    } catch (err) {
      console.warn('Failed to load macros:', err);
    }

    // Load HTTP Requests
    try {
      const httpRequests = await invoke<any[]>('list_http_requests');
      if (httpRequests && Array.isArray(httpRequests)) {
        httpRequests.forEach((request) => {
          actions.push({
            id: `http-${request.id}`,
            name: request.name || `HTTP Request ${request.id}`,
            deckName: 'HTTP Requests',
            action: `{"HttpRequestId":${request.id}}`,
          });
        });
      }
    } catch (err) {
      console.warn('Failed to load HTTP requests:', err);
    }

    // Load saved actions from every registered integration
    try {
      const savedActions = await invoke<Record<string, any[]>>('get_all_saved_integration_actions');
      for (const [integrationId, providerActions] of Object.entries(savedActions || {})) {
        if (Array.isArray(providerActions)) {
          providerActions.forEach((action) => {
            actions.push({
              id: `${integrationId}-${action.stable_id}`,
              name: action.name || `${integrationId.toUpperCase()} Action ${action.id}`,
              deckName: `${integrationId.toUpperCase()} Actions`,
              action: JSON.stringify({ SavedActionId: action.stable_id }),
            });
          });
        }
      }
    } catch (err) {
      console.warn('Failed to load integration actions:', err);
    }

    // Load Focus Windows
    try {
      const focusWindows = await invoke<any[]>('get_focus_windows');
      if (focusWindows && Array.isArray(focusWindows)) {
        focusWindows.forEach((window) => {
          actions.push({
            id: `window-${window.id}`,
            name: window.name || `Focus Window ${window.id}`,
            deckName: 'Focus Windows',
            action: `{"FocusWindowId":${window.id}}`,
          });
        });
      }
    } catch (err) {
      console.warn('Failed to load focus windows:', err);
    }

    // Load Flows
    try {
      const flows = await invoke<any[]>('list_flows');
      if (flows && Array.isArray(flows)) {
        flows.forEach((flow) => {
          actions.push({
            id: `flow-${flow.id}`,
            name: flow.name || `Flow ${flow.id}`,
            deckName: 'Flows',
            action: `{"FlowId":"${flow.id}"}`,
          });
        });
      }
    } catch (err) {
      console.warn('Failed to load flows:', err);
    }

    availableDeckActions.value = actions;
    console.log(`📋 Loaded ${actions.length} total actions from all sources`);
  } catch (err) {
    console.error('Failed to load deck actions:', err);
    availableDeckActions.value = [];
  }
}

// Group deck actions by deck name
const groupedDeckActions = computed(() => {
  const groups: Record<string, Array<{ id: string; name: string; deckName: string; action: string }>> = {};

  for (const action of availableDeckActions.value) {
    if (!groups[action.deckName]) {
      groups[action.deckName] = [];
    }
    groups[action.deckName].push(action);
  }

  return groups;
});

// Group actions for multiselect (with proper group structure)
const groupedActionsForMultiselect = computed(() => {
  const groups: Record<string, any> = {};

  // Group actions by deckName
  availableDeckActions.value.forEach(action => {
    if (!groups[action.deckName]) {
      groups[action.deckName] = [];
    }
    groups[action.deckName].push({
      ...action,
      label: action.name,
      value: action.action
    });
  });

  // Convert to array format for vue-multiselect groups
  return Object.entries(groups).map(([groupName, items]) => ({
    group: groupName,
    items: items
  }));
});

// Flatten for searching across all actions
const flattenedActions = computed(() => {
  return availableDeckActions.value.map(action => ({
    ...action,
    label: action.name,
    value: action.action
  }));
});

// Get selected action object from action string
function getSelectedAction(actionString: string) {
  return flattenedActions.value.find(a => a.action === actionString) || null;
}

// Update command action when multiselect changes
function updateCommandAction(command: any, selectedAction: any) {
  command.action = selectedAction ? selectedAction.action : '';
}

// Validation
const isValid = computed(() => {
  return sessionData.value.name.trim().length > 0 &&
         sessionData.value.platforms.length > 0 &&
         sessionData.value.commands.every(command => !command.enabled ||
           (command.trigger.length > 0 && !/\s/.test(command.trigger) && command.action.length > 0));
});

// Load session data if editing
watch(() => props.session, (newSession) => {
  if (newSession) {
    // Deep clone to avoid mutating original session
    sessionData.value = {
      ...newSession,
      platforms: newSession.platforms ? newSession.platforms.map(p => ({...p})) : [],
      commands: newSession.commands ? JSON.parse(JSON.stringify(newSession.commands)) : [],
      settings: newSession.settings ? {...newSession.settings} : {
        autoStart: true,
        notifications: true,
        autoResume: true,
      },
      runs: newSession.runs ? [...newSession.runs] : [],
    };
  } else {
    resetForm();
  }
}, { immediate: true });

function resetForm() {
  sessionData.value = {
    id: '',
    name: '',
    description: '',
    platforms: [],
    commands: [],
    settings: {
      autoStart: true,
      notifications: true,
      autoResume: true,
    },
  };
}

function toggleAccount(platform: string, account: ConnectedAccount) {
  const index = sessionData.value.platforms.findIndex(p => p.accountId === String(account.id));

  if (index > -1) {
    // Already selected - remove it
    sessionData.value.platforms.splice(index, 1);
  } else {
    // Not selected - add it
    const platformStream: PlatformStream = {
      platform: platform as any,
      accountId: String(account.id),
      // Optional fields will be filled when stream goes live
      streamId: undefined,
      title: undefined,
      startedAt: undefined,
      viewerCount: undefined,
    };
    sessionData.value.platforms.push(platformStream);
  }
}

function isAccountSelected(accountId: number): boolean {
  return sessionData.value.platforms.some(p => p.accountId === String(accountId));
}

function getPlatformIcon(platformId: string) {
  const iconMap: Record<string, any> = {
    twitch: LogosTwitch,
    youtube: LogosYoutube,
    kick: SimpleIconsKick,
    tiktok: LogosTiktok,
  };
  return iconMap[platformId.toLowerCase()];
}

function addCommand() {
  sessionData.value.commands.push({
    trigger: '',
    action: '',
    cooldown: null,
    enabled: true,
    permissionVersion: 1,
    allowedRoles: publicCommandRoles(),
    userWhitelist: [],
    userBlacklist: [],
  });
}

function removeCommand(index: number) {
  sessionData.value.commands.splice(index, 1);
}

// Drag-and-drop helper functions
function createActionFromReference(
  reference: import("@base/types/actionReference").ActionReference,
): string {
  const formatMap: Record<string, string> = {
    'program': JSON.stringify({ Program: Number(reference.id) }),
    'macro': JSON.stringify({ MacroId: Number(reference.id) }),
    'flow': JSON.stringify({ FlowId: reference.id }),
    'saved_action': JSON.stringify({ SavedActionId: reference.id }),
    'http_request': JSON.stringify({ HttpRequestId: Number(reference.id) }),
    'focus_window': JSON.stringify({ FocusWindowId: Number(reference.id) }),
    'system_control': JSON.stringify({ SystemControl: reference.id }),
  };
  return formatMap[reference.kind] || '';
}

function handleDragOver(event: DragEvent, commandIndex: number) {
  event.preventDefault();
  isDraggingOver.value = commandIndex;
}

function handleDragLeave(commandIndex: number) {
  isDraggingOver.value = null;
}

function handleDrop(event: DragEvent, commandIndex: number) {
  event.preventDefault();
  isDraggingOver.value = null;

  try {
    const jsonData = event.dataTransfer?.getData("application/json");
    if (!jsonData) return;

    const payload = JSON.parse(jsonData);

    if (payload.event === 'action-reference') {
      const actionString = createActionFromReference(payload.data.reference);
      const command = sessionData.value.commands[commandIndex];

      if (actionString && command) {
        command.action = actionString;
        console.log('✅ Action assigned to command:', actionString);
      }
    }
  } catch (err) {
    console.error('❌ Invalid drop data:', err);
  }
}

function handleQuickDrop(event: DragEvent) {
  event.preventDefault();
  quickDropHover.value = false;

  try {
    const jsonData = event.dataTransfer?.getData("application/json");
    if (!jsonData) return;

    const payload = JSON.parse(jsonData);

    if (payload.event === 'action-reference') {
      const actionString = createActionFromReference(payload.data.reference);

      if (actionString) {
        // Create new command with pre-filled action
        sessionData.value.commands.push({
          trigger: '',  // User fills this in
          action: actionString,
          cooldown: null,
          enabled: true,
          permissionVersion: 1,
          allowedRoles: publicCommandRoles(),
          userWhitelist: [],
          userBlacklist: [],
        });

        console.log('✅ New command created with action:', payload.data.label);
      }
    }
  } catch (err) {
    console.error('❌ Invalid drop data:', err);
  }
}

// Role management functions
function publicCommandRoles(): Record<string, string[]> {
  return Object.fromEntries(['twitch', 'youtube', 'kick', 'tiktok', 'discord'].map(platform => [platform, ['everyone']]));
}

function getPlatformRoles(platform: string): string[] {
  const roleMap: Record<string, string[]> = {
    twitch: ['everyone', 'subscriber', 'vip', 'moderator', 'broadcaster'],
    youtube: ['everyone', 'verified', 'member', 'moderator', 'owner'],
    kick: ['everyone', 'subscriber', 'moderator', 'broadcaster'],
  };
  return roleMap[platform.toLowerCase()] || ['everyone'];
}

function isRoleSelected(command: any, platform: string, role: string): boolean {
  if (!command.allowedRoles) return command.permissionVersion !== 1 && role === 'everyone';
  const platformRoles = command.allowedRoles[platform];
  return platformRoles ? platformRoles.includes(role) : command.permissionVersion !== 1 && role === 'everyone';
}

function toggleRole(command: any, platform: string, role: string) {
  if (command.permissionVersion !== 1) {
    const oldRoles = command.allowedRoles || {};
    command.allowedRoles = publicCommandRoles();
    for (const [name, selected] of Object.entries(oldRoles)) {
      if (Array.isArray(selected) && selected.length) command.allowedRoles[name] = [...selected];
    }
    command.permissionVersion = 1;
  }
  if (!command.allowedRoles) {
    command.allowedRoles = {};
  }
  if (!command.allowedRoles[platform]) {
    command.allowedRoles[platform] = [];
  }

  const roles = command.allowedRoles[platform];
  const index = roles.indexOf(role);

  if (index > -1) {
    roles.splice(index, 1);
  } else {
    roles.push(role);
  }
}

// String/Array conversion helpers for whitelist/blacklist
function arrayToString(arr: string[] | undefined): string {
  return arr?.join(', ') || '';
}

function stringToArray(str: string): string[] {
  return str.split(',').map(s => s.trim()).filter(s => s.length > 0);
}

function closeModal() {
  emit('update:modelValue', false);
  // resetForm is called automatically by the watch when props.session becomes null
}

function saveDraft() {
  if (!isValid.value) return;
  emit('save', { ...sessionData.value, status: 'draft' });
  closeModal();
}

function saveAndActivate() {
  if (!isValid.value) return;
  const status = (sessionData.value as any).status || 'draft';
  emit('save', { ...sessionData.value, status });
  closeModal();
}

async function openManageAccounts() {
  try {
    await openUrl('https://vortideck.com/account');
  } catch (error) {
    console.error('Failed to open account management URL:', error);
  }
}
</script>

<style lang="scss" scoped>
.fade-enter-active {
  animation: fade-in 0.3s;
}

.fade-leave-active {
  animation: fade-out 0.3s;
}

@keyframes fade-in {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

@keyframes fade-out {
  from {
    opacity: 1;
  }
  to {
    opacity: 0;
  }
}

.jump-enter-active {
  animation: jump-in 0.3s;
}

.jump-leave-active {
  animation: jump-out 0.3s;
}

@keyframes jump-in {
  0% {
    transform: translateY(-100vh);
    opacity: 0;
  }
  100% {
    transform: translateY(0);
    opacity: 1;
  }
}

@keyframes jump-out {
  0% {
    transform: translateY(0);
    opacity: 1;
  }
  100% {
    transform: translateY(100vh);
    opacity: 0;
  }
}

// Override vue-multiselect default styles with theme-aware colors
:deep(.action-multiselect) {
  .multiselect {
    background: hsl(var(--background2));
    border: 1px solid hsl(var(--border2));
    border-radius: 0.5rem;
    min-height: 38px;
    font-size: 0.875rem;
  }

  .multiselect__tags {
    background: hsl(var(--background2));
    border: none;
    padding: 6px 40px 0 8px;
    min-height: 38px;
  }

  .multiselect__single {
    background: hsl(var(--background2));
    color: hsl(var(--text));
    font-size: 0.875rem;
    margin-bottom: 6px;
    padding: 2px 0 0 0;
  }

  .multiselect__input {
    background: hsl(var(--background2));
    color: hsl(var(--text));
    font-size: 0.875rem;
    padding: 2px 0 0 0;
    margin-bottom: 6px;
  }

  .multiselect__placeholder {
    color: hsl(var(--text2));
    font-size: 0.875rem;
    margin-bottom: 6px;
    padding-top: 2px;
  }

  .multiselect__select {
    height: 38px;
    &:before {
      border-color: hsl(var(--text2)) transparent transparent;
    }
  }

  .multiselect__content-wrapper {
    background: hsl(var(--background2));
    border: 1px solid hsl(var(--border2));
    border-radius: 0.5rem;
    margin-top: 4px;
    max-height: 240px;
    overflow-y: auto;
  }

  .multiselect__content {
    background: hsl(var(--background2));
  }

  .multiselect__element {
    background: hsl(var(--background2));
  }

  .multiselect__option {
    color: hsl(var(--text));
    font-size: 0.875rem;
    padding: 8px 12px;
    min-height: unset;
    line-height: 1.4;

    &--highlight {
      background: hsl(var(--background3));
      color: hsl(var(--text));

      &:after {
        display: none;
      }
    }

    &--selected {
      background: rgb(59, 130, 246, 0.2);
      color: rgb(59, 130, 246);
      font-weight: 500;

      &.multiselect__option--highlight {
        background: rgb(59, 130, 246, 0.3);
        color: rgb(59, 130, 246);
      }
    }

    &--group {
      background: hsl(var(--background)) !important;
      color: hsl(var(--text)) !important;
      font-weight: 600;
      font-size: 0.75rem;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      padding: 6px 12px;
      cursor: default;
      pointer-events: none;
    }
  }

  .multiselect__option--group-selected {
    background: hsl(var(--background3));
    color: hsl(var(--text));
  }

  .multiselect--active {
    .multiselect__tags {
      border-color: rgb(59, 130, 246);
    }
  }

  .multiselect__spinner {
    background: hsl(var(--background2));

    &:before,
    &:after {
      border-top-color: rgb(59, 130, 246);
    }
  }
}
</style>
