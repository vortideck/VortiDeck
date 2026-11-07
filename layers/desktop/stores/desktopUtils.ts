import { createPinia } from "pinia";
import { type PeerInfo } from "@/types/Peer";
import { defineStore } from "pinia"; // Import defineStore for Pinia
import piniaPluginPersistedstate from "pinia-plugin-persistedstate";

const pinia = createPinia();
pinia.use(piniaPluginPersistedstate);

interface SplashscreenTask {
  id: string;
  name: string;
  status: 'pending' | 'running' | 'completed' | 'failed';
  progress?: number;
  message?: string;

}

export const useDesktopUtils = defineStore("desktopUtils", {
  state: () => {
    return {
      // cookies settings
      isSidebar: false,
      wasClicked: false,
      integrationStatuses: {} as Record<string, boolean>, // Universal integration status tracking
      deckEditorMenu: "macros" as "keys" | "scripts" | "actions" | "flows" | "macros" | "utilities" | "context-menu",
      isUserModal: false,
      isSimulatorOpen: false,
      isShowInstruction: null as boolean | null,
      splashscreenTasks: {} as Record<string, SplashscreenTask>,
      mcpTools: [] as any[]
    };
  },
  getters: {
    getIsSidebar: (state) => {
      return state.isSidebar;
    },
    getIsUserModal: (state) => {
      return state.isUserModal;
    },
    getWasClicked: (state) => {
      return state.wasClicked;
    },

    // Universal integration status getters
    getIntegrationStatus: (state) => (integrationId: string) => {
      return state.integrationStatuses[integrationId] || false;
    },
    getIsObsPluginActive: (state) => {
      return state.integrationStatuses["obs"] || false;
    },
    getDeckEditorMenu: (state) => {
      return state.deckEditorMenu;
    },
    getIsSimulatorOpen: (state) => {
      return state.isSimulatorOpen;
    },
    getIsShowInstruction: (state) => {
      return state.isShowInstruction;
    },
    getSplashscreenTasks: (state) => {
      return state.splashscreenTasks;
    },
    getMcpTools: (state) => {
      return state.mcpTools;
    }
  },
  actions: {
    toggleSidebar() {
      this.wasClicked = true;
      this.isSidebar = !this.isSidebar;
    },
    setUserModal(status: boolean) {
      this.isUserModal = status;
    },
    closeSidebar() {
      this.isSidebar = false;
      this.wasClicked = false;
    },

    // Universal integration status management
    setIntegrationStatus(integrationId: string, status: boolean) {
      this.integrationStatuses[integrationId] = status;
    },
    setIsObsPluginActive(status: boolean) {
      this.integrationStatuses["obs"] = status;
    },
    setDeckEditorMenu(value: "keys" | "scripts" | "actions" | "macros" | "flows" | "utilities" | "context-menu") {
      this.deckEditorMenu = value;
    },

    setIsSimulatorOpen(status: boolean) {
      this.isSimulatorOpen = status;
    },
    setIsShowinstruction(bool: boolean) {
      this.isShowInstruction = bool;
    },
    
    addSplashscreenTask(task: SplashscreenTask) {
      this.splashscreenTasks[task.id] = task;
    },
    
    updateSplashscreenTask(id: string, updates: Partial<SplashscreenTask>) {
      if (this.splashscreenTasks[id]) {
        this.splashscreenTasks[id] = { ...this.splashscreenTasks[id], ...updates };
      }
    },
    
    removeSplashscreenTask(id: string) {
      delete this.splashscreenTasks[id];
    },
    
    clearSplashscreenTasks() {
      this.splashscreenTasks = {};
    },

    addMcpTools(tools: any[]) {
      for (const tool of tools) {
        // Check if a tool with this name already exists
        const existingToolIndex = this.mcpTools.findIndex(t => t.name === tool.name);
        if (existingToolIndex === -1) {
          // Only add if the tool doesn't already exist
          this.mcpTools.push(tool);
          console.log(`Added new MCP tool: ${tool.name}`);
        } else {
          // Optionally update the existing tool with new information
          console.log(`Tool ${tool.name} already exists, not adding duplicate`);
          // Uncomment below if you want to update existing tools instead of skipping
          // this.mcpTools[existingToolIndex] = { ...this.mcpTools[existingToolIndex], ...tool };
        }
      }
    },
    setMcpTools(tools: any[]) {
      this.mcpTools = tools;
    },

    // other options...
  },
  persist: {
    storage: sessionStorage,
  },
});
