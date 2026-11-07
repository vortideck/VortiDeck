// Import necessary dependencies
// import { useStorage } from "@vueuse/core";
import { createPinia } from "pinia";
import { type PeerInfo } from "@base/types/Peer";
import { defineStore } from "pinia"; // Import defineStore for Pinia
import piniaPluginPersistedstate from "pinia-plugin-persistedstate";

const pinia = createPinia();
pinia.use(piniaPluginPersistedstate);

type ConnectionStatus = "connected" | "connecting" | "disconnected" | "paired";
type ConnectionMode = "direct" | "relay";
export type ConnectionPreference = "automatic" | "direct" | "relay";
export type ActiveConnectionTransport = "lan" | "tailscale" | "relay";
export type ActionRuntimeStatus = "initializing" | "ready" | "failed";
export interface ActionRuntimeReadiness {
  status: ActionRuntimeStatus;
  phase: string;
  warnings: string[];
  error: string | null;
}

// Define the Pinia store
export const useConnectionStore = defineStore("connectionStore", {
  state: () => ({
    serverInfo: {},
    paired: [] as PeerInfo[],
    paired_active: [] as PeerInfo[],
    status: "disconnected" as ConnectionStatus,
    connectionMode: "direct" as ConnectionMode,
    connectionPreference: "automatic" as ConnectionPreference,
    activeConnectionTransport: null as ActiveConnectionTransport | null,
    activeConnectionLatencyMs: null as number | null,
    relayFallbackAvailable: false,
    activeDesktopId: null as string | null,
    availableDesktops: [] as any[],
    relaySessionId: null as string | null,
    desktopRelayStatus: "disconnected" as "connected" | "disconnected",
    wsUrl: null as string | null,  // Store WebSocket URL for reconnection
    featureLimits: {} as Record<string, number>,  // Cache feature limits from server_info
    actionRuntimeReadiness: {
      status: "initializing",
      phase: "connecting",
      warnings: [],
      error: null,
    } as ActionRuntimeReadiness,
    errorMessage: null as string | null,  // User-friendly error message for modals
  }),
  getters: {
    getServerInfo(state) {
      return state.serverInfo
    },
    getPaired(state) {
      return state.paired;
    },
    getPairedActive(state) {
      return state.paired_active;
    },
    getStatus(state) {
      if (state.paired_active.length >= 1) {
        if (state.status === "connected") return "paired";
        else return state.status;
      } else return state.status;
    },
    getConnectionMode(state) {
      return state.connectionMode;
    },
    getConnectionPreference(state) {
      return state.connectionPreference;
    },
    getActiveConnectionTransport(state) {
      return state.activeConnectionTransport;
    },
    getActiveConnectionLatencyMs(state) {
      return state.activeConnectionLatencyMs;
    },
    getRelayFallbackAvailable(state) {
      return state.relayFallbackAvailable;
    },
    getActiveDesktopId(state) {
      return state.activeDesktopId;
    },
    getAvailableDesktops(state) {
      return state.availableDesktops;
    },
    getRelaySessionId(state) {
      return state.relaySessionId;
    },
    getDesktopRelayStatus(state) {
      return state.desktopRelayStatus;
    },
    getWsUrl(state) {
      return state.wsUrl;
    },
    getFeatureLimits(state) {
      return state.featureLimits;
    },
    getActionRuntimeReadiness(state) {
      return state.actionRuntimeReadiness;
    },
    getErrorMessage(state) {
      return state.errorMessage;
    },
  },
  actions: {
    setServerInfo(serverInfo: Record<string, unknown>) {
      this.serverInfo = serverInfo
    },
    addPeer(peer: PeerInfo) {
      // Check if peer already exists to prevent duplicates
      const existingIndex = this.paired.findIndex(p => p.client_id === peer.client_id);
      if (existingIndex >= 0) {
        // Update existing peer
        this.paired[existingIndex] = peer;
      } else {
        // Add new peer
        this.paired.push(peer);
      }
    },
    addActivePeer(peer: PeerInfo) {
      // Check if peer already exists to prevent duplicates
      const existingIndex = this.paired_active.findIndex(p => p.client_id === peer.client_id);
      if (existingIndex >= 0) {
        // Update existing peer info (in case of reconnection with updated info)
        this.paired_active[existingIndex] = peer;
      } else {
        // Add new peer
        this.paired_active.push(peer);
      }
    },
    removeActivePeer(peer: PeerInfo | string) {
      // Handle both PeerInfo object and string client_id
      const clientId = typeof peer === 'string' ? peer : peer.client_id;
      this.paired_active = this.paired_active.filter((p) => p.client_id !== clientId);
    },
    setPairedActive(peers: PeerInfo[]) {
      this.paired_active = peers;
    },

    setPeers(peers: PeerInfo[] | null) {
      if (peers) this.paired = peers;
    },
    setStatus(status: ConnectionStatus) {
      this.status = status;
    },
    setConnectionMode(mode: ConnectionMode) {
      this.connectionMode = mode;
    },
    setConnectionPreference(preference: ConnectionPreference) {
      this.connectionPreference = preference;
    },
    setActiveConnectionTransport(transport: ActiveConnectionTransport | null) {
      this.activeConnectionTransport = transport;
    },
    setActiveConnectionLatencyMs(latencyMs: number | null) {
      this.activeConnectionLatencyMs = latencyMs;
    },
    setRelayFallbackAvailable(available: boolean) {
      this.relayFallbackAvailable = available;
    },
    setActiveDesktopId(desktopId: string | null) {
      this.activeDesktopId = desktopId;
    },
    setAvailableDesktops(desktops: any[]) {
      this.availableDesktops = desktops;
    },
    setRelaySessionId(sessionId: string | null) {
      this.relaySessionId = sessionId;
    },
    setDesktopRelayStatus(status: "connected" | "disconnected") {
      this.desktopRelayStatus = status;
    },
    setWsUrl(url: string | null) {
      this.wsUrl = url;
    },
    setFeatureLimits(limits: Record<string, number>) {
      this.featureLimits = limits;
      console.log('📊 Feature limits cached in store:', limits);
    },
    setActionRuntimeReadiness(readiness: ActionRuntimeReadiness) {
      this.actionRuntimeReadiness = readiness;
    },
    setErrorMessage(message: string | null) {
      this.errorMessage = message;
    },
  },
  persist: [
    {
      storage: sessionStorage,
      // Runtime state is session-scoped and refreshed from the active connection.
      pick: ['serverInfo', 'paired', 'paired_active', 'status', 'connectionMode', 'activeConnectionTransport', 'activeConnectionLatencyMs', 'relayFallbackAvailable', 'activeDesktopId', 'availableDesktops', 'relaySessionId', 'desktopRelayStatus', 'wsUrl'],
    },
    {
      key: 'vortideck-connection-preferences',
      storage: localStorage,
      pick: ['connectionPreference'],
    },
  ],
});
